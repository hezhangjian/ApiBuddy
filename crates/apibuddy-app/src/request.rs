use crate::{
    state::{RequestTab, Workbench},
    ui,
};
use apibuddy_core::{Authentication, METHODS, Pair, Variable};
use nsh_base::editor::{Editor, EditorApi, EditorPosition, EditorSelection, editor};
use nsh_base::{SelectItem, button, menu, menu_item, theme};
use nsh_core::prelude::*;

pub fn request_editor(
    store: Workbench,
    tab: RequestTab,
    show_collection: impl Fn() + 'static,
) -> impl IntoElement {
    let timeout =
        signal((tab.draft.with_untracked(|d| d.request.timeout_ms) as f64 / 1000.0).to_string());
    view()
        .flex_1()
        .min_h(px(170.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
                .child(
                    button()
                        .h(px(30.0))
                        .px_2()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .rounded(px(6.0))
                        .hover(|s| s.bg(|| theme().secondary))
                        .accessibility_label(move || {
                            Some(SharedString::from(if tab.saved.get().is_some() {
                                "查看所属集合"
                            } else {
                                "选择保存集合"
                            }))
                        })
                        .on_press(move || {
                            if tab.saved.get_untracked().is_some() {
                                show_collection();
                            } else {
                                store.save(tab, false);
                            }
                        })
                        .child(ui::icon(ui::Glyph::Folder))
                        .child(
                            ui::muted(move || {
                                if tab.saved.get().is_some() {
                                    tab.service.with(|s| s.name.clone())
                                } else {
                                    "临时请求".into()
                                }
                            })
                            .max_w(px(150.0))
                            .number_of_lines(1)
                            .text_size(px(12.0)),
                        ),
                )
                .child(ui::caption("/"))
                .child(
                    ui::field(
                        "请求名称",
                        move || tab.draft.with(|d| d.name.clone()),
                        move |value| tab.draft.update(|d| d.name = value.into()),
                    )
                    .flex_1()
                    .min_w(px(100.0))
                    .max_w(px(300.0))
                    .h(px(30.0))
                    .font_weight(FontWeight::MEDIUM)
                    .border_color(|| AdaptiveColor::TRANSPARENT)
                    .hover(|s| s.border_color(|| nsh_base::theme().input)),
                )
                .child(
                    ui::muted(move || {
                        if tab.saved.get().is_none() {
                            "未保存"
                        } else if Workbench::dirty(tab) {
                            "已修改"
                        } else {
                            "已保存"
                        }
                    })
                    .text_size(px(11.0))
                    .number_of_lines(1),
                )
                .child(view().flex_1())
                .child(
                    ui::choice(
                        "环境",
                        move || {
                            let mut items = vec![SelectItem::new("", "无环境")];
                            items.extend(store.environments.with(|e| {
                                e.profiles
                                    .iter()
                                    .map(|p| SelectItem::new(p.id.clone(), p.name.clone()))
                                    .collect::<Vec<_>>()
                            }));
                            items
                        },
                        move || tab.environment.get().unwrap_or_default(),
                        move || {
                            let id = tab.environment.get();
                            store.environments.with(|e| {
                                e.profiles
                                    .iter()
                                    .find(|p| Some(&p.id) == id.as_ref())
                                    .map(|p| p.name.clone())
                                    .unwrap_or_else(|| "无环境".into())
                            })
                        },
                        move |id| {
                            tab.environment
                                .set(if id.is_empty() { None } else { Some(id) })
                        },
                    )
                    .w(px(145.0)),
                )
                .child(ui::icon_button(
                    ui::Glyph::Sliders,
                    "管理环境",
                    move || {
                        store.dialog.set(crate::state::Dialog::Environment);
                    },
                ))
                .child(
                    ui::icon_action(ui::Glyph::Folder, "保存", move || {
                        store.save(tab, false);
                    })
                    .accessibility_label("保存请求"),
                )
                .child(request_menu(store, tab)),
        )
        .child(
            view()
                .flex_row()
                .gap_2()
                .px_4()
                .pb_3()
                .child(
                    ui::choice(
                        "请求方法",
                        move || {
                            METHODS
                                .iter()
                                .map(|m| SelectItem::new(*m, *m))
                                .collect::<Vec<_>>()
                        },
                        move || tab.draft.with(|d| d.request.method.clone()),
                        move || tab.draft.with(|d| d.request.method.clone()),
                        move |method| tab.draft.update(|d| d.request.method = method),
                    )
                    .w(px(110.0))
                    .trigger(
                        view()
                            .h(px(40.0))
                            .px_3()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(|| nsh_base::theme().input)
                            .bg(|| nsh_base::theme().secondary)
                            .child(
                                ui::label(move || tab.draft.with(|d| d.request.method.clone()))
                                    .font_family(ui::MONO)
                                    .font_weight(FontWeight::BOLD)
                                    .flex_1()
                                    .text_color(move || {
                                        tab.draft.with(|d| ui::method_color(&d.request.method))
                                    }),
                            )
                            .child(ui::caption("⌄")),
                    ),
                )
                .child(
                    ui::field(
                        "请求 URL",
                        move || tab.draft.with(|d| d.request.url.clone()),
                        move |value| tab.draft.update(|d| d.request.url = value.into()),
                    )
                    .font_family(ui::MONO)
                    .h(px(40.0))
                    .rounded(px(8.0))
                    .bg(|| nsh_base::theme().secondary)
                    .placeholder("https://api.example.com/path")
                    .auto_focus(
                        tab.saved.get_untracked().is_none()
                            && tab.draft.with_untracked(|d| d.request.url.is_empty()),
                    )
                    .on_submit(move |_: &str| {
                        if !tab.response.with_untracked(|r| r.running) {
                            store.send(tab);
                        }
                    })
                    .flex_1(),
                )
                .child(
                    ui::primary(
                        move || {
                            if tab.response.with(|r| r.running) {
                                "取消"
                            } else {
                                "发送"
                            }
                        },
                        move || store.send(tab),
                    )
                    .w(px(96.0))
                    .h(px(40.0))
                    .accessibility_label("发送或取消请求"),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .px_4()
                .border_b_1()
                .border_color(|| nsh_base::theme().border)
                .child(view().flex_1().child(ui::tabs_with_counts(
                    &["Params", "Headers", "Body", "Authorization", "变量", "设置"],
                    tab.section,
                    move |index| {
                        tab.draft.with(|d| match index {
                            0 => d
                                .request
                                .query
                                .iter()
                                .filter(|p| p.enabled && !p.key.is_empty())
                                .count(),
                            1 => d
                                .request
                                .headers
                                .iter()
                                .filter(|p| p.enabled && !p.key.is_empty())
                                .count(),
                            4 => d.request.variables.len(),
                            _ => 0,
                        })
                    },
                ))),
        )
        .child(dynamic(move || match tab.section.get() {
            0 => pairs(tab, 0).into_element(),
            1 => pairs(tab, 1).into_element(),
            2 => body(store, tab).into_element(),
            3 => auth(tab).into_element(),
            4 => variables(tab).into_element(),
            _ => view()
                .p_4()
                .gap_3()
                .child(ui::label("请求超时（1–600 秒）"))
                .child(
                    ui::field("超时秒数", timeout, move |value| {
                        timeout.set(value.to_string());
                        let seconds = value.parse::<f64>().unwrap_or(0.0);
                        tab.draft
                            .update(|d| d.request.timeout_ms = (seconds * 1000.0).round() as u64);
                    })
                    .w(px(160.0)),
                )
                .child(ui::muted("TLS 证书校验开启 · 重定向作为原始 3xx 响应显示"))
                .child(
                    ui::field(
                        "请求描述",
                        move || tab.draft.with(|d| d.description.clone()),
                        move |value| tab.draft.update(|d| d.description = value.into()),
                    )
                    .placeholder("描述接口用途"),
                )
                .into_element(),
        }))
}

fn request_menu(store: Workbench, tab: RequestTab) -> impl IntoElement {
    let item = |title: &'static str, action: usize| {
        menu_item()
            .accessibility_label(title)
            .h(px(34.0))
            .px_3()
            .flex_row()
            .items_center()
            .gap_2()
            .rounded(px(5.0))
            .focus(|s| s.bg(|| theme().accent))
            .hover(|s| s.bg(|| theme().accent))
            .on_select(move || match action {
                0 => store.save(tab, true),
                1 => store.duplicate(tab),
                _ => store.copy_curl(tab),
            })
            .child(ui::icon(match action {
                0 => ui::Glyph::Folder,
                1 => ui::Glyph::Copy,
                _ => ui::Glyph::Code,
            }))
            .child(ui::label(title).text_size(px(12.0)))
    };
    menu()
        .align(Align::End)
        .offset(px(4.0))
        .popup_class("w-44 p-1 bg-popover border border-border rounded-lg shadow-lg")
        .trigger(
            view()
                .h(px(30.0))
                .px_2()
                .rounded(px(6.0))
                .items_center()
                .justify_center()
                .role(AccessibilityRole::Button)
                .label("更多请求操作")
                .hover(|s| s.bg(|| theme().secondary))
                .child(ui::muted("•••").text_size(px(17.0))),
        )
        .child(item("另存到集合…", 0))
        .child(item("复制为临时请求", 1))
        .child(item("复制 cURL", 2))
}

fn pair_rows(d: &apibuddy_core::RequestDraft, kind: usize) -> &[Pair] {
    match kind {
        1 => &d.headers,
        2 => d.form.as_deref().unwrap_or_default(),
        _ => &d.query,
    }
}
fn pair_rows_mut(d: &mut apibuddy_core::RequestDraft, kind: usize) -> &mut Vec<Pair> {
    match kind {
        1 => &mut d.headers,
        2 => d.form.get_or_insert_default(),
        _ => &mut d.query,
    }
}

fn pairs(tab: RequestTab, kind: usize) -> impl IntoElement {
    let rows = computed(move || {
        tab.draft
            .with(|d| (0..pair_rows(&d.request, kind).len()).collect::<Vec<_>>())
    });
    let title = ["查询参数", "请求头", "Form 字段"][kind];
    view()
        .flex_1()
        .min_h(px(0.0))
        .px_4()
        .py_2()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    ui::muted(if kind == 1 {
                        "Name / Value · 支持 {{变量}}"
                    } else {
                        if kind == 2 {
                            "URL-encoded Form · 字段自动编码，支持重复字段"
                        } else {
                            "参数会编码后追加到 URL，支持重复参数"
                        }
                    })
                    .text_size(px(11.0))
                    .flex_1(),
                )
                .child(
                    ui::ghost("+ 添加", move || {
                        tab.draft.update(|d| {
                            let rows = pair_rows_mut(&mut d.request, kind);
                            if rows.len() < 100 {
                                rows.push(Pair {
                                    key: String::new(),
                                    value: String::new(),
                                    enabled: true,
                                });
                            }
                        })
                    })
                    .accessibility_label(format!("添加{title}")),
                ),
        )
        .child(
            view()
                .h(px(26.0))
                .flex_row()
                .gap_2()
                .items_center()
                .px_2()
                .bg(|| nsh_base::theme().secondary)
                .rounded(px(5.0))
                .child(ui::caption("启用").w(px(38.0)))
                .child(ui::caption("KEY").flex_1())
                .child(ui::caption("VALUE").flex_1())
                .child(view().w(px(30.0))),
        )
        .child(when(
            move || rows.with(Vec::is_empty),
            move || ui::caption("暂无字段 · 点击右上角添加").px_2().py_3(),
        ))
        .child(scroll().flex_1().child(view().gap_1().child(each(
            rows,
            |i| *i,
            move |index, _| {
                let i = index.get_untracked();
                view()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(
                        ui::action(
                            move || {
                                tab.draft.with(|d| {
                                    let rows = pair_rows(&d.request, kind);
                                    if rows.get(i).is_some_and(|r| r.enabled) {
                                        "☑"
                                    } else {
                                        "☐"
                                    }
                                })
                            },
                            move || {
                                tab.draft.update(|d| {
                                    let rows = pair_rows_mut(&mut d.request, kind);
                                    if let Some(row) = rows.get_mut(i) {
                                        row.enabled = !row.enabled;
                                    }
                                })
                            },
                        )
                        .accessibility_label(format!("启用{title} {}", i + 1)),
                    )
                    .child(
                        ui::field(
                            if kind == 1 {
                                "请求头名称"
                            } else {
                                if kind == 2 {
                                    "Form 名称"
                                } else {
                                    "参数名称"
                                }
                            },
                            move || {
                                tab.draft.with(|d| {
                                    let rows = pair_rows(&d.request, kind);
                                    rows.get(i).map(|r| r.key.clone()).unwrap_or_default()
                                })
                            },
                            move |value| {
                                tab.draft.update(|d| {
                                    let rows = pair_rows_mut(&mut d.request, kind);
                                    if let Some(row) = rows.get_mut(i) {
                                        row.key = value.into();
                                    }
                                })
                            },
                        )
                        .placeholder("Key")
                        .flex_1(),
                    )
                    .child(
                        ui::field(
                            if kind == 1 {
                                "请求头值"
                            } else {
                                if kind == 2 { "Form 值" } else { "参数值" }
                            },
                            move || {
                                tab.draft.with(|d| {
                                    let rows = pair_rows(&d.request, kind);
                                    rows.get(i).map(|r| r.value.clone()).unwrap_or_default()
                                })
                            },
                            move |value| {
                                tab.draft.update(|d| {
                                    let rows = pair_rows_mut(&mut d.request, kind);
                                    if let Some(row) = rows.get_mut(i) {
                                        row.value = value.into();
                                    }
                                })
                            },
                        )
                        .placeholder("Value")
                        .flex_1(),
                    )
                    .child(
                        ui::action("×", move || {
                            tab.draft.update(|d| {
                                let rows = pair_rows_mut(&mut d.request, kind);
                                if i < rows.len() {
                                    rows.remove(i);
                                }
                            })
                        })
                        .accessibility_label(format!("移除{title} {}", i + 1)),
                    )
            },
        ))))
}

fn body(store: Workbench, tab: RequestTab) -> impl IntoElement {
    let form_mode = computed(move || tab.draft.with(|d| d.request.form.is_some()));
    let code = component_ref::<Editor>();
    let focused = signal(false);
    let json_mode = computed(move || {
        tab.draft.with(|d| {
            d.request.headers.iter().any(|h| {
                h.enabled
                    && h.key.eq_ignore_ascii_case("content-type")
                    && h.value.to_ascii_lowercase().contains("json")
            })
        })
    });
    let format = move |compact| {
        let source = tab.draft.with_untracked(|d| d.request.body.clone());
        match serde_json::from_str::<serde_json::Value>(&source) {
            Ok(json) => {
                let value = if compact {
                    serde_json::to_string(&json)
                } else {
                    serde_json::to_string_pretty(&json)
                }
                .expect("JSON value serializes");
                if let Some(editor) = code.api::<EditorApi>() {
                    batch(|| {
                        // NSH 0.4 set_value retains old byte offsets. Collapse first so
                        // a shorter/differently indented Unicode document has valid selections.
                        editor.set_selections(vec![EditorSelection::caret(EditorPosition::new(
                            0, 0,
                        ))]);
                        editor.set_value(value);
                        editor.focus();
                    });
                }
                store.notice.set(
                    if compact {
                        "JSON 已压缩，可撤销"
                    } else {
                        "JSON 已格式化，可撤销"
                    }
                    .into(),
                );
            }
            Err(error) => {
                store.notice.set(format!(
                    "JSON 格式错误：第 {} 行，第 {} 列 · {}",
                    error.line(),
                    error.column(),
                    error
                ));
                if let Some(editor) = code.api::<EditorApi>() {
                    let line = error.line().saturating_sub(1);
                    let text = source.lines().nth(line).unwrap_or_default();
                    let mut offset = error.column().saturating_sub(1).min(text.len());
                    while !text.is_char_boundary(offset) {
                        offset -= 1;
                    }
                    let position = EditorPosition::new(
                        line as u32,
                        text[..offset].encode_utf16().count() as u32,
                    );
                    editor.set_selections(vec![EditorSelection::caret(position)]);
                    editor.reveal_position(position, true);
                    editor.focus();
                }
            }
        }
    };
    view()
        .flex_1()
        .min_h(px(0.0))
        .px_4()
        .py_2()
        .gap_2()
        .child(
            view()
                .flex_row()
                .gap_2()
                .items_center()
                .child(
                    ui::muted(move || {
                        tab.draft.with(|d| {
                            if matches!(d.request.method.as_str(), "GET" | "HEAD") {
                                "GET / HEAD 不发送正文"
                            } else if d.request.form.is_some() {
                                "Form 字段 · 切换模式保留草稿"
                            } else {
                                "原始正文 · 支持 {{变量}}"
                            }
                        })
                    })
                    .text_size(px(11.0))
                    .number_of_lines(1)
                    .flex_1(),
                )
                .child(
                    ui::ghost("JSON", move || set_body_type(tab, "application/json"))
                        .selected(move || !form_mode.get() && json_mode.get())
                        .bg(move || {
                            if !form_mode.get() && json_mode.get() {
                                nsh_base::theme().accent
                            } else {
                                AdaptiveColor::TRANSPARENT
                            }
                        }),
                )
                .child(
                    ui::ghost("Text", move || set_body_type(tab, "text/plain"))
                        .selected(move || !form_mode.get() && !json_mode.get())
                        .bg(move || {
                            if !form_mode.get() && !json_mode.get() {
                                nsh_base::theme().accent
                            } else {
                                AdaptiveColor::TRANSPARENT
                            }
                        }),
                )
                .child(
                    ui::ghost("Form", move || {
                        set_body_type(tab, "application/x-www-form-urlencoded")
                    })
                    .selected(move || form_mode.get())
                    .bg(move || {
                        if form_mode.get() {
                            nsh_base::theme().accent
                        } else {
                            AdaptiveColor::TRANSPARENT
                        }
                    }),
                )
                .child(ui::ghost("格式化", move || format(false)).disabled(move || form_mode.get()))
                .child(ui::ghost("压缩", move || format(true)).disabled(move || form_mode.get()))
                .child(
                    ui::action(
                        move || {
                            if tab.body_wrap.get() {
                                "换行 ✓"
                            } else {
                                "换行"
                            }
                        },
                        move || tab.body_wrap.update(|v| *v = !*v),
                    )
                    .accessibility_label("切换正文自动换行")
                    .disabled(move || form_mode.get()),
                ),
        )
        .child(dynamic(move || {
            if form_mode.get() {
                pairs(tab, 2).into_element()
            } else {
                cleanup(move || focused.set(false));
                view()
                    .test_id("请求正文")
                    .flex_1()
                    .min_h(px(80.0))
                    .child(
                        editor()
                            .component_ref(&code)
                            .value(move || {
                                SharedString::from(tab.draft.with(|d| d.request.body.clone()))
                            })
                            .on_change(move |value: &SharedString| {
                                tab.draft.update(|d| d.request.body = value.to_string())
                            })
                            .on_focus(move || focused.set(true))
                            .on_blur(move || focused.set(false))
                            .language(move || {
                                Some(SharedString::from(if json_mode.get() {
                                    "json"
                                } else {
                                    "text"
                                }))
                            })
                            .accessibility_label("请求正文")
                            .soft_wrap(tab.body_wrap)
                            .font_family(ui::MONO)
                            .font_size(px(13.0))
                            .line_height(px(20.0))
                            .tab_size(2u32)
                            .placeholder("输入 JSON 或文本正文")
                            .flex_1()
                            .min_h(px(80.0)),
                    )
                    .into_element()
            }
        }))
        .child(when(
            move || focused.get() && store.dialog.get() == crate::state::Dialog::None,
            move || {
                crate::workbench::register_request_keys(store);
                view()
            },
        ))
}

pub fn set_header(tab: RequestTab, name: &str, value: &str) {
    tab.draft.update(|d| {
        d.request
            .headers
            .retain(|row| !row.key.eq_ignore_ascii_case(name));
        if !value.is_empty() {
            d.request.headers.push(Pair {
                key: name.into(),
                value: value.into(),
                enabled: true,
            });
        }
    });
}

fn auth(tab: RequestTab) -> impl IntoElement {
    let mode = move || {
        tab.draft.with(|d| {
            match d.request.auth {
                Authentication::Headers => "headers",
                Authentication::None => "none",
                Authentication::Bearer { .. } => "bearer",
                Authentication::Basic { .. } => "basic",
                Authentication::ApiKey { .. } => "key",
            }
            .to_string()
        })
    };
    let auth_kind = computed(move || {
        tab.draft.with(|d| match d.request.auth {
            Authentication::Bearer { .. } => 1,
            Authentication::Basic { .. } => 2,
            Authentication::ApiKey { .. } => 3,
            _ => 0,
        })
    });
    scroll().flex_1().child(
        view()
            .p_4()
            .gap_3()
            .child(ui::choice(
                "认证方式",
                move || {
                    vec![
                        SelectItem::new("headers", "手动 Headers / 导入的认证"),
                        SelectItem::new("none", "No Auth"),
                        SelectItem::new("bearer", "Bearer Token"),
                        SelectItem::new("basic", "Basic Auth"),
                        SelectItem::new("key", "API Key"),
                    ]
                },
                mode,
                move || {
                    match mode().as_str() {
                        "none" => "No Auth",
                        "bearer" => "Bearer Token",
                        "basic" => "Basic Auth",
                        "key" => "API Key",
                        _ => "手动 Headers / 导入的认证",
                    }
                    .into()
                },
                move |mode| {
                    tab.draft.update(|d| {
                        d.request.auth = match mode.as_str() {
                            "none" => Authentication::None,
                            "bearer" => Authentication::Bearer {
                                token: String::new(),
                            },
                            "basic" => Authentication::Basic {
                                username: String::new(),
                                password: String::new(),
                            },
                            "key" => Authentication::ApiKey {
                                key: "X-API-Key".into(),
                                value: String::new(),
                                query: false,
                            },
                            _ => Authentication::Headers,
                        }
                    })
                },
            ))
            .child(dynamic(move || {
                // Depend only on the variant so typing does not reconstruct the focused input.
                match auth_kind.get() {
                    1 => view()
                        .gap_2()
                        .child(ui::label("Bearer Token"))
                        .child(auth_field(tab, "Bearer Token", 0))
                        .into_element(),
                    2 => view()
                        .gap_2()
                        .child(ui::label("用户名"))
                        .child(auth_field(tab, "Basic 用户名", 0))
                        .child(ui::label("密码"))
                        .child(auth_field(tab, "Basic 密码", 1))
                        .into_element(),
                    3 => view()
                        .gap_2()
                        .child(ui::label("Key / Value"))
                        .child(auth_field(tab, "API Key 名称", 0))
                        .child(auth_field(tab, "API Key 值", 1))
                        .child(ui::choice(
                            "API Key 位置",
                            move || {
                                vec![
                                    SelectItem::new("header", "Header"),
                                    SelectItem::new("query", "Query Params"),
                                ]
                            },
                            move || {
                                tab.draft.with(|d| {
                                    if matches!(
                                        d.request.auth,
                                        Authentication::ApiKey { query: true, .. }
                                    ) {
                                        "query"
                                    } else {
                                        "header"
                                    }
                                    .into()
                                })
                            },
                            move || {
                                tab.draft.with(|d| {
                                    if matches!(
                                        d.request.auth,
                                        Authentication::ApiKey { query: true, .. }
                                    ) {
                                        "Query Params"
                                    } else {
                                        "Header"
                                    }
                                    .into()
                                })
                            },
                            move |value| {
                                tab.draft.update(|d| {
                                    if let Authentication::ApiKey { query, .. } =
                                        &mut d.request.auth
                                    {
                                        *query = value == "query";
                                    }
                                })
                            },
                        ))
                        .into_element(),
                    _ => ui::muted(
                        "手动模式使用 Headers 中的认证；No Auth 发送时移除 Authorization。",
                    )
                    .into_element(),
                }
            }))
            .child(ui::muted(
                "支持 {{变量}}，发送时解析。认证信息随请求明文保存在本机。",
            )),
    )
}

fn auth_field(tab: RequestTab, name: &'static str, index: usize) -> impl IntoElement {
    ui::field(
        name,
        move || {
            tab.draft.with(|d| {
                d.request
                    .auth
                    .fields()
                    .get(index)
                    .copied()
                    .unwrap_or_default()
                    .to_string()
            })
        },
        move |value| {
            tab.draft.update(|d| match &mut d.request.auth {
                Authentication::Bearer { token } => *token = value.into(),
                Authentication::Basic { username, password } => {
                    *if index == 0 { username } else { password } = value.into()
                }
                Authentication::ApiKey { key, value: v, .. } => {
                    *if index == 0 { key } else { v } = value.into()
                }
                _ => {}
            })
        },
    )
    .masked(name == "Basic 密码")
    .placeholder("支持 {{变量}}")
}

fn variables(tab: RequestTab) -> impl IntoElement {
    let rows = computed(move || {
        tab.draft
            .with(|d| (0..d.request.variables.len()).collect::<Vec<_>>())
    });
    view()
        .flex_1()
        .min_h(px(0.0))
        .px_4()
        .py_2()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .child(ui::muted("模板默认值；所选环境中的同名变量优先").flex_1())
                .child(ui::action("+ 变量", move || {
                    tab.draft.update(|d| {
                        if d.request.variables.len() < 100 {
                            d.request.variables.push(Variable {
                                key: String::new(),
                                value: String::new(),
                            });
                        }
                    })
                })),
        )
        .child(scroll().flex_1().child(view().gap_1().child(each(
            rows,
            |i| *i,
            move |index, _| {
                let i = index.get_untracked();
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        ui::field(
                            "变量名",
                            move || {
                                tab.draft.with(|d| {
                                    d.request
                                        .variables
                                        .get(i)
                                        .map(|v| v.key.clone())
                                        .unwrap_or_default()
                                })
                            },
                            move |value| {
                                tab.draft.update(|d| {
                                    if let Some(row) = d.request.variables.get_mut(i) {
                                        row.key = value.into();
                                    }
                                })
                            },
                        )
                        .placeholder("name")
                        .flex_1(),
                    )
                    .child(
                        ui::field(
                            "变量默认值",
                            move || {
                                tab.draft.with(|d| {
                                    d.request
                                        .variables
                                        .get(i)
                                        .map(|v| v.value.clone())
                                        .unwrap_or_default()
                                })
                            },
                            move |value| {
                                tab.draft.update(|d| {
                                    if let Some(row) = d.request.variables.get_mut(i) {
                                        row.value = value.into();
                                    }
                                })
                            },
                        )
                        .placeholder("value")
                        .flex_1(),
                    )
                    .child(
                        ui::action("×", move || {
                            tab.draft.update(|d| {
                                if i < d.request.variables.len() {
                                    d.request.variables.remove(i);
                                }
                            })
                        })
                        .accessibility_label(format!("移除变量 {}", i + 1)),
                    )
            },
        ))))
}

fn set_body_type(tab: RequestTab, content_type: &str) {
    tab.draft.update(|d| {
        if content_type == "application/x-www-form-urlencoded" {
            if d.request.form.is_none() {
                let imported_form = d.request.headers.iter().any(|row| {
                    row.enabled
                        && row.key.eq_ignore_ascii_case("content-type")
                        && row
                            .value
                            .split(';')
                            .next()
                            .unwrap_or_default()
                            .trim()
                            .eq_ignore_ascii_case("application/x-www-form-urlencoded")
                });
                d.request.form = Some(d.request.form_draft.take().unwrap_or_else(|| {
                    if !imported_form {
                        return vec![];
                    }
                    url::form_urlencoded::parse(d.request.body.as_bytes())
                        .map(|(key, value)| Pair {
                            key: key.into_owned(),
                            value: value.into_owned(),
                            enabled: true,
                        })
                        .collect()
                }));
            }
        } else if let Some(form) = d.request.form.take() {
            d.request.form_draft = Some(form);
        }
    });
    set_header(tab, "Content-Type", content_type);
}
