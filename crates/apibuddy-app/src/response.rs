use crate::{
    state::{Dialog, RequestTab, Response, Workbench},
    ui,
};
use nsh_base::{Editor, EditorApi, EditorPosition, EditorSelection, checkbox, editor, theme};
use nsh_core::prelude::*;
use regex::RegexBuilder;
use std::ops::Range;

const DISPLAY_LIMIT: usize = 256 * 1024;
const MATCH_LIMIT: usize = 2000;

/// Session-only viewing preferences belong to the tab, not its mounted editor.
#[derive(Clone, Copy)]
pub struct ResponseView {
    pub section: Signal<usize>,
    pub search_open: Signal<bool>,
    query: Signal<String>,
    case_sensitive: Signal<bool>,
    pub selected: Signal<Option<usize>>,
    pointer_open: Signal<bool>,
    pointer: Signal<String>,
    wrap: Signal<bool>,
}

impl ResponseView {
    pub fn clear_inputs(self) {
        self.query.set(String::new());
        self.pointer.set(String::new());
    }

    pub fn new() -> Self {
        Self {
            section: signal(0),
            search_open: signal(false),
            query: signal(String::new()),
            case_sensitive: signal(false),
            selected: signal(None),
            pointer_open: signal(false),
            pointer: signal(String::new()),
            wrap: signal(true),
        }
    }
}

#[derive(Clone, PartialEq)]
struct DisplayContent {
    text: String,
    truncated: bool,
}

fn display_content(response: &Response, section: usize) -> DisplayContent {
    let source = if section == 2 {
        &response.headers
    } else {
        &response.body
    };
    let pretty = if section == 0 && !response.running && source.len() <= DISPLAY_LIMIT {
        serde_json::from_str::<serde_json::Value>(source)
            .ok()
            .and_then(|value| serde_json::to_string_pretty(&value).ok())
    } else {
        None
    };
    display_text(pretty.as_deref().unwrap_or(source))
}

fn display_text(source: &str) -> DisplayContent {
    let end = source.floor_char_boundary(DISPLAY_LIMIT.min(source.len()));
    DisplayContent {
        // Textareas normalize newlines. Use identical text for both backends' UTF-16 ranges.
        text: source[..end].replace("\r\n", "\n").replace('\r', "\n"),
        truncated: source.len() > end,
    }
}

fn json_field<'a>(
    value: &'a serde_json::Value,
    pointer: &str,
) -> Result<&'a serde_json::Value, String> {
    if !pointer.is_empty() && !pointer.starts_with('/') {
        return Err("路径需以 / 开头；留空查看完整 JSON".into());
    }
    let mut chars = pointer.chars();
    while let Some(c) = chars.next() {
        if c == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return Err("路径转义：~0 表示 ~，~1 表示 /".into());
        }
    }
    value
        .pointer(pointer)
        .ok_or_else(|| "未找到此 JSON 字段".into())
}

#[derive(Clone, Default, PartialEq)]
struct Matches {
    ranges: Vec<Range<u32>>,
    limited: bool,
    error: Option<String>,
}

fn find_matches(text: &str, query: &str, case_sensitive: bool) -> Matches {
    if query.is_empty() {
        return Matches::default();
    }
    let Ok(pattern) = RegexBuilder::new(&regex::escape(query))
        .case_insensitive(!case_sensitive)
        .build()
    else {
        return Matches {
            error: Some("搜索文本过长，请缩短关键词".into()),
            ..Default::default()
        };
    };
    let mut found = Matches::default();
    let (mut byte, mut utf16) = (0, 0u32);
    for matched in pattern.find_iter(text) {
        if found.ranges.len() == MATCH_LIMIT {
            found.limited = true;
            break;
        }
        // Walk each prefix once; lowercasing the whole string would corrupt offsets.
        utf16 += text[byte..matched.start()].encode_utf16().count() as u32;
        let end = utf16 + matched.as_str().encode_utf16().count() as u32;
        found.ranges.push(utf16..end);
        byte = matched.end();
        utf16 = end;
    }
    found
}

fn next_match(current: Option<usize>, count: usize, backwards: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match current.filter(|i| *i < count) {
        None if backwards => count - 1,
        None => 0,
        Some(i) if backwards => (i + count - 1) % count,
        Some(i) => (i + 1) % count,
    })
}

fn editor_position(text: &str, offset: u32) -> EditorPosition {
    let mut position = EditorPosition::new(0, 0);
    let mut units = 0;
    for c in text.chars() {
        if units >= offset {
            break;
        }
        units += c.len_utf16() as u32;
        if c == '\n' {
            position.line += 1;
            position.column = 0;
        } else {
            position.column += c.len_utf16() as u32;
        }
    }
    position
}

pub fn response_view(store: Workbench, tab: RequestTab) -> impl IntoElement {
    let state = tab.response_view;
    let search_ref = element_ref();
    let response_ref = component_ref::<Editor>();
    let editor_focused = signal(false);
    let running = computed(move || tab.response.with(|response| response.running));
    // Parse once per completed response, not on every path edit or elapsed-time tick.
    let parsed = computed(move || {
        if !state.pointer_open.get() || state.section.get() == 2 {
            return Err(String::new());
        }
        tab.content_version.get();
        tab.response.with_untracked(|r| {
            if r.running {
                return Err("等待响应结束后提取 JSON 字段".to_string());
            }
            serde_json::from_str::<serde_json::Value>(&r.body)
                .map_err(|_| "响应不是有效 JSON".to_string())
        })
    });
    let field = computed(move || {
        let pointer = state.pointer.get();
        let section = state.section.get();
        parsed.with(|parsed| {
            let value = json_field(parsed.as_ref().map_err(Clone::clone)?, &pointer)?;
            if section == 0 {
                serde_json::to_string_pretty(value)
            } else {
                serde_json::to_string(value)
            }
            .map_err(|_| "无法显示此 JSON 字段".to_string())
        })
    });
    let content = computed(move || {
        let section = state.section.get();
        if state.pointer_open.get() && section != 2 {
            return field.with(|result| display_text(result.as_deref().unwrap_or_default()));
        }
        tab.content_version.get();
        tab.response.with_untracked(|r| display_content(r, section))
    });
    let matches = computed(move || {
        if !state.search_open.get() {
            return Matches::default();
        }
        let query = state.query.get();
        let case_sensitive = state.case_sensitive.get();
        content.with(|value| find_matches(&value.text, &query, case_sensitive))
    });
    let previous_view = stored((
        state.section.get_untracked(),
        state.pointer_open.get_untracked(),
        state.pointer.get_untracked(),
    ));
    effect(move || {
        let view = (
            state.section.get(),
            state.pointer_open.get(),
            state.pointer.get(),
        );
        if previous_view.with(|previous| std::mem::replace(previous, view.clone())) != view {
            state.selected.set(None);
        }
    });
    let navigate = move |backwards| {
        let next = matches.with_untracked(|m| {
            next_match(state.selected.get_untracked(), m.ranges.len(), backwards)
        });
        state.selected.set(next);
        if let Some(range) = next.and_then(|i| matches.with_untracked(|m| m.ranges.get(i).cloned()))
        {
            if let Some(editor) = response_ref.api::<EditorApi>() {
                let selection = content.with_untracked(|c| {
                    EditorSelection::new(
                        editor_position(&c.text, range.start),
                        editor_position(&c.text, range.end),
                    )
                });
                editor.set_selections(vec![selection]);
                editor.reveal_position(selection.anchor, true);
            }
        }
    };
    let open_search = move || {
        state.search_open.set(true);
        // auto_focus handles the first mount; these commands handle an already open field.
        search_ref.focus();
        search_ref.set_selection(
            0,
            state
                .query
                .with_untracked(|s| s.encode_utf16().count() as u32),
        );
    };
    on_key_with(
        "mod+f",
        KeyOptions {
            when: nsh_core::keymap::When::Always,
            ..Default::default()
        },
        move |_| {
            if store.dialog.get_untracked() == Dialog::None {
                open_search();
            }
        },
    );
    view()
        .flex_1()
        .min_h(px(120.0))
        .border_t_1()
        .border_color(|| theme().border)
        .child(
            view()
                .min_h(px(40.0))
                .py_2()
                .flex_row()
                .items_center()
                .gap_3()
                .px_4()
                .bg(|| theme().secondary)
                .child(ui::label("响应").font_weight(FontWeight::SEMIBOLD))
                .child(
                    ui::label(move || {
                        tab.response.with(|r| {
                            if r.message.is_empty() {
                                "等待发送".into()
                            } else {
                                r.message.clone()
                            }
                        })
                    })
                    .flex_1()
                    .number_of_lines(3),
                )
                .child(ui::muted(move || {
                    tab.response
                        .with(|r| format!("{} ms  ·  {} B", r.elapsed, r.bytes))
                }))
                .child(ui::action("复制响应", move || {
                    let value = tab.response.with_untracked(|r| {
                        if state.section.get_untracked() == 2 {
                            r.headers.clone()
                        } else {
                            r.body.clone()
                        }
                    });
                    crate::backend::copy_text(value);
                    store.notice.set("响应已复制".into());
                })),
        )
        .child(
            view()
                .px_4()
                .flex_row()
                .items_center()
                .child(
                    view()
                        .flex_1()
                        .child(ui::tabs(&["Pretty", "Raw", "响应头"], state.section)),
                )
                .child(
                    ui::action(
                        move || {
                            if state.wrap.get() {
                                "换行 ✓"
                            } else {
                                "换行"
                            }
                        },
                        move || state.wrap.update(|wrap| *wrap = !*wrap),
                    )
                    .accessibility_label("切换响应自动换行"),
                )
                .child(
                    ui::action("JSON 字段", move || {
                        state.pointer_open.update(|open| *open = !*open)
                    })
                    .disabled(move || state.section.get() == 2),
                )
                .child(ui::action("查找", open_search).accessibility_label("查找响应")),
        )
        .child(when(
            move || state.pointer_open.get() && state.section.get() != 2,
            move || {
                view()
                    .px_4()
                    .py_1()
                    .gap_1()
                    .child(
                        view()
                            .flex_row()
                            .gap_2()
                            .items_center()
                            .child(
                                ui::field("JSON 字段路径", state.pointer, move |value| {
                                    state.pointer.set(value.into())
                                })
                                .placeholder("JSON Pointer，如 /data/0/name；留空查看完整 JSON")
                                .flex_1(),
                            )
                            .child(
                                ui::action("复制字段 JSON", move || {
                                    if let Ok(value) = field.get_untracked() {
                                        crate::backend::copy_text(value);
                                        store.notice.set("字段 JSON 已复制".into());
                                    }
                                })
                                .disabled(move || field.with(Result::is_err)),
                            )
                            .child(
                                ui::action("×", move || state.pointer_open.set(false))
                                    .accessibility_label("关闭 JSON 字段"),
                            ),
                    )
                    .child(when(
                        move || field.with(Result::is_err),
                        move || {
                            ui::muted(move || {
                                field.with(|f| f.as_ref().err().cloned().unwrap_or_default())
                            })
                            .text_size(px(11.0))
                        },
                    ))
            },
        ))
        .child(when(state.search_open, move || {
            view()
                .px_4()
                .py_1()
                .flex_row()
                .gap_2()
                .items_center()
                .child(
                    ui::field("查找响应文本", state.query, move |value| {
                        state.query.set(value.into());
                        state.selected.set(None);
                    })
                    .element_ref(&search_ref)
                    .auto_focus(true)
                    .placeholder("查找当前视图… Enter 下一处")
                    .max_length(1024)
                    .flex_1()
                    .submit_behavior(nsh_core::SubmitBehavior::Submit)
                    .on_submit(move |_: &str| navigate(false)),
                )
                .child(
                    checkbox()
                        .accessibility_label("区分大小写")
                        .child(ui::label("Aa"))
                        .h(px(30.0))
                        .px_2()
                        .rounded(px(4.0))
                        .justify_center()
                        .border_1()
                        .border_color(move || {
                            if state.case_sensitive.get() {
                                theme().primary
                            } else {
                                theme().border
                            }
                        })
                        .bg(move || {
                            if state.case_sensitive.get() {
                                theme().accent
                            } else {
                                theme().background
                            }
                        })
                        .checked(move || state.case_sensitive.get())
                        .on_change(move |checked: &bool| {
                            state.case_sensitive.set(*checked);
                            state.selected.set(None);
                        }),
                )
                .child(
                    ui::muted(move || {
                        matches.with(|m| {
                            if let Some(error) = &m.error {
                                return error.clone();
                            }
                            if state.query.with(String::is_empty) {
                                return "输入关键词".into();
                            }
                            let suffix = if m.limited { "+" } else { "" };
                            match state.selected.get().filter(|i| *i < m.ranges.len()) {
                                Some(i) => format!("{} / {}{}", i + 1, m.ranges.len(), suffix),
                                None => format!("{}{} 处匹配", m.ranges.len(), suffix),
                            }
                        })
                    })
                    .test_id("响应匹配计数"),
                )
                .child(
                    ui::action("↑", move || navigate(true))
                        .accessibility_label("上一处匹配")
                        .disabled(move || matches.with(|m| m.ranges.is_empty())),
                )
                .child(
                    ui::action("↓", move || navigate(false))
                        .accessibility_label("下一处匹配")
                        .disabled(move || matches.with(|m| m.ranges.is_empty())),
                )
                .child(
                    ui::action("×", move || state.search_open.set(false))
                        .accessibility_label("关闭响应查找"),
                )
                .child(when(
                    move || store.dialog.get() == Dialog::None,
                    move || {
                        for (key, backwards) in [("mod+g", false), ("shift+mod+g", true)] {
                            on_key_with(
                                key,
                                KeyOptions {
                                    when: nsh_core::keymap::When::Always,
                                    ..Default::default()
                                },
                                move |_| navigate(backwards),
                            );
                        }
                        view()
                    },
                ))
        }))
        .child(when(
            move || content.with(|c| c.truncated),
            move || {
                ui::muted(move || {
                    if state.pointer_open.get() && state.section.get() != 2 {
                        "仅显示并查找字段前 256 KiB；复制字段 JSON 可获得完整值"
                    } else {
                        "仅显示并查找前 256 KiB；复制响应可获得完整内容"
                    }
                })
                .px_4()
                .py_1()
                .text_size(px(11.0))
            },
        ))
        .child(
            view()
                .flex_1()
                .min_h(px(0.0))
                .px_4()
                .py_2()
                .child(dynamic(move || {
                    // A different view starts with fresh virtual rows and scroll position.
                    // NSH 0.4 can retain stale gutter rows when a long document becomes a short field.
                    state.section.get();
                    state.pointer_open.get();
                    state.pointer.get();
                    running.get();
                    cleanup(move || editor_focused.set(false));
                    editor()
                        .value(move || SharedString::from(content.with(|c| c.text.clone())))
                        .component_ref(&response_ref)
                        .on_focus(move || editor_focused.set(true))
                        .on_blur(move || editor_focused.set(false))
                        .accessibility_label("响应内容")
                        .read_only(true)
                        .soft_wrap(state.wrap)
                        .font_family(ui::MONO)
                        .font_size(px(13.0))
                        .line_height(px(20.0))
                        .folding(false)
                        .active_line(false)
                        .indent_guides(false)
                        .language(move || {
                            Some(SharedString::from(if state.section.get() == 0 {
                                "json"
                            } else {
                                "text"
                            }))
                        })
                        .flex_1()
                        .min_h(px(80.0))
                        .placeholder("发送请求后查看响应正文、状态码、耗时和大小")
                })),
        )
        .child(when(
            move || editor_focused.get() && store.dialog.get() == Dialog::None,
            move || {
                // The editor has its own keymap. Keep workbench search/cancel keys available while it owns focus.
                crate::workbench::register_request_keys(store);
                on_key_with(
                    "escape",
                    KeyOptions {
                        when: nsh_core::keymap::When::Always,
                        ..Default::default()
                    },
                    move |_| {
                        if state.search_open.get_untracked() {
                            state.search_open.set(false);
                        } else {
                            store.cancel(tab);
                        }
                    },
                );
                when(state.search_open, move || {
                    for (key, backwards) in [("mod+g", false), ("shift+mod+g", true)] {
                        on_key_with(
                            key,
                            KeyOptions {
                                when: nsh_core::keymap::When::Always,
                                ..Default::default()
                            },
                            move |_| navigate(backwards),
                        );
                    }
                    view()
                })
            },
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_unicode_matches_use_original_utf16_offsets() {
        assert_eq!(
            find_matches("İ 😀中文 Σ σ ς K k", "中文", false).ranges,
            vec![4..6]
        );
        assert_eq!(
            find_matches("İ 😀中文 Σ σ ς K k", "σ", false).ranges,
            vec![7..8, 9..10, 11..12]
        );
        assert_eq!(
            find_matches("İ 😀中文 Σ σ ς K k", "k", false).ranges,
            vec![13..14, 15..16]
        );
        assert_eq!(find_matches("a.* A.*", ".*", true).ranges, vec![1..3, 5..7]);
        assert_eq!(find_matches("Ab aB", "Ab", true).ranges, vec![0..2]);
        assert!(find_matches("text", "", false).ranges.is_empty());
        assert_eq!(editor_position("a\nİ 😀中文", 7), EditorPosition::new(1, 5));
    }

    #[test]
    fn search_bounds_and_navigation_are_explicit() {
        let found = find_matches(&"x".repeat(MATCH_LIMIT + 1), "x", false);
        assert_eq!(found.ranges.len(), MATCH_LIMIT);
        assert!(found.limited);
        assert!(!find_matches(&"x".repeat(MATCH_LIMIT), "x", false).limited);
        assert_eq!(next_match(None, 3, false), Some(0));
        assert_eq!(next_match(None, 3, true), Some(2));
        assert_eq!(next_match(Some(2), 3, false), Some(0));
        assert_eq!(next_match(Some(0), 3, true), Some(2));
        assert_eq!(next_match(Some(9), 2, false), Some(0));
        assert_eq!(next_match(Some(0), 0, false), None);
    }

    #[test]
    fn display_is_bounded_without_changing_the_response() {
        let body = format!("{}中文TAIL", "a".repeat(DISPLAY_LIMIT - 1));
        let response = Response {
            body: body.clone(),
            headers: "A: 中文\r\nB: 😀\r\n".into(),
            ..Default::default()
        };
        let display = display_content(&response, 1);
        assert!(display.truncated);
        assert_eq!(display.text.len(), DISPLAY_LIMIT - 1);
        assert!(find_matches(&display.text, "TAIL", false).ranges.is_empty());
        assert_eq!(response.body, body);
        assert_eq!(display_content(&response, 2).text, "A: 中文\nB: 😀\n");
        let response = Response {
            body: "{\"a\":1}".into(),
            ..Default::default()
        };
        assert_eq!(display_content(&response, 0).text, "{\n  \"a\": 1\n}");
        assert_eq!(display_content(&response, 1).text, "{\"a\":1}");
    }

    #[test]
    fn json_paths_handle_escaped_keys_arrays_null_and_missing_values() {
        let value = serde_json::json!({"data": [{"a/b": {"~name": "中文😀"}}, null], "": 42});
        assert_eq!(json_field(&value, "/data/0/a~1b/~0name").unwrap(), "中文😀");
        assert!(json_field(&value, "/data/1").unwrap().is_null());
        assert_eq!(json_field(&value, "/").unwrap(), 42);
        assert_eq!(json_field(&value, "").unwrap(), &value);
        for path in [
            "data",
            "/missing",
            "/data/02",
            "/data/-",
            "/data/0/a~2b",
            "/data/0/~",
        ] {
            assert!(json_field(&value, path).is_err(), "{path}");
        }
        let value =
            serde_json::json!({"padding": "x".repeat(DISPLAY_LIMIT), "tail": "完整响应中的字段"});
        assert_eq!(json_field(&value, "/tail").unwrap(), "完整响应中的字段");
    }
}
