use crate::{
    dialogs, request, response,
    state::{Dialog, Workbench},
    ui,
};
use nsh_base::{SelectItem, button, theme};
use nsh_core::prelude::*;

pub fn workbench(store: Workbench) -> impl IntoElement {
    let sidebar = signal(true);
    let search = signal(String::new());
    let navigation = signal(0);
    let search_ref = element_ref();
    register_request_keys(store);
    for (key, action) in [("mod+b", 0), ("mod+k", 1)] {
        on_key_with(
            key,
            KeyOptions {
                when: nsh_core::keymap::When::Always,
                ..Default::default()
            },
            move |_| {
                if store.dialog.get_untracked() != Dialog::None {
                    return;
                }
                if action == 0 {
                    sidebar.update(|v| *v = !*v);
                } else {
                    sidebar.set(true);
                    search_ref.focus();
                }
            },
        );
    }
    let filtered = computed(move || {
        let query = search.get().to_lowercase();
        store.library.with(|items| {
            items
                .iter()
                .filter(|t| {
                    t.name.to_lowercase().contains(&query)
                        || t.request.url.to_lowercase().contains(&query)
                        || t.request.method.to_lowercase().contains(&query)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let filtered_history = computed(move || {
        let query = search.get().to_lowercase();
        store.history.with(|entries| {
            entries
                .iter()
                .filter(|e| {
                    e.input.name.to_lowercase().contains(&query)
                        || e.input.request.url.to_lowercase().contains(&query)
                        || e.input.request.method.to_lowercase().contains(&query)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    view()
        .flex_1()
        .min_h(px(0.0))
        .bg(|| theme().background)
        .child(
            view()
                .h(px(42.0))
                .flex_row()
                .items_center()
                .gap_3()
                .px_4()
                .border_b_1()
                .border_color(|| theme().border)
                .child(ui::icon_button(ui::Glyph::Sidebar, "切换侧栏", move || sidebar.update(|v| *v = !*v)))
                .child(view().size(px(24.0)).rounded(px(7.0)).bg(|| theme().primary)
                    .items_center().justify_center()
                    .child(ui::label("A").font_weight(FontWeight::BOLD).text_color(|| theme().primary_foreground)))
                .child(ui::label("ApiBuddy").font_weight(FontWeight::SEMIBOLD).text_size(px(14.0)))
                .child(ui::caption("/"))
                .child(ui::muted("我的工作区").text_size(px(12.0)).flex_1())
                .child(ui::caption("LOCAL").text_size(px(10.0)).px_2().py_1().rounded(px(5.0)).bg(|| theme().secondary))
                .child(ui::icon_action(ui::Glyph::Import, "导入 cURL", move || store.dialog.set(Dialog::Import)))
                .child(ui::icon_action(ui::Glyph::Sliders, "环境", move || store.dialog.set(Dialog::Environment))
                    .accessibility_label("管理环境"))
                .child({
                    #[cfg(feature = "cli-integration")]
                    { Some(ui::action("CLI 与 AI", move || store.install_cli()).into_element()) }
                    #[cfg(not(feature = "cli-integration"))]
                    { None::<AnyElement> }
                }),
        )
        .child(
            view()
                .flex_1()
                .min_h(px(0.0))
                .flex_row()
                .child(when(sidebar, move || {
                    view()
                        .w(px(232.0))
                        .min_h(px(0.0))
                        .bg(|| theme().sidebar)
                        .border_r_1()
                        .border_color(|| theme().border)
                        .child(
                            view()
                                .p_3()
                                .gap_2()
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap_2()
                                        .child(ui::icon(ui::Glyph::Folder))
                                        .child(ui::caption("工作区集合").flex_1())
                                        .child(ui::icon_button(ui::Glyph::Plus, "新建集合", move || store.dialog.set(Dialog::Service))),
                                )
                                .child(ui::choice(
                                    "当前集合",
                                    move || {
                                        store.catalog.with(|items| {
                                            items
                                                .iter()
                                                .map(|s| {
                                                    SelectItem::new(s.id.clone(), s.name.clone())
                                                })
                                                .collect::<Vec<_>>()
                                        })
                                    },
                                    move || store.selected_service.get(),
                                    move || store.service().name,
                                    move |id| {
                                        store.select_service(id);
                                    },
                                ))
                                .child(
                                    ui::field("搜索接口", search, move |value| {
                                        search.set(value.into())
                                    })
                                    .element_ref(&search_ref)
                                    .placeholder(if crate::backend::mac_shortcuts() {
                                        "搜索接口…  ⌘K"
                                    } else {
                                        "搜索接口…  Ctrl+K"
                                    }),
                                )
                                .child(ui::segments(&["Collections", "History"], navigation)),
                        )
                        .child(dynamic(move || {
                            if navigation.get() == 0 {
                                view()
                                    .flex_1()
                                    .min_h(px(0.0))
                                    .child(
                                        virtual_list(
                                            move || filtered.with(Vec::len),
                                            move |at| {
                                                button()
                                                    .h(px(52.0))
                                                    .px_3()
                                                    .justify_center()
                                                    .gap_1()
                                                    .mx_2().rounded(px(8.0))
                                                    .bg(move || {
                                                        let current = store.tabs.with(|tabs| tabs.iter().find(|t| t.id == store.active.get()).and_then(|t| t.saved.get()).map(|s| s.0));
                                                        let item = filtered.with(|items| items.get(at.index()).map(|t| t.id.clone()));
                                                        if current.is_some() && current == item { theme().accent } else { AdaptiveColor::TRANSPARENT }
                                                    })
                                                    .hover(|s| s.bg(|| theme().accent))
                                                    .on_press(move || {
                                                        if let Some(saved) = filtered
                                                            .with_untracked(|items| {
                                                                items.get(at.index()).cloned()
                                                            })
                                                        {
                                                            store.open_saved(saved);
                                                        }
                                                    })
                                                    .child(
                                                        view()
                                                            .flex_row()
                                                            .gap_2()
                                                            .items_center()
                                                            .child(
                                                                ui::label(move || {
                                                                    filtered.with(|items| {
                                                                        items
                                                                            .get(at.index())
                                                                            .map(|t| {
                                                                                t.request
                                                                                    .method
                                                                                    .clone()
                                                                            })
                                                                            .unwrap_or_default()
                                                                    })
                                                                })
                                                                .w(px(51.0))
                                                                .text_size(px(11.0))
                                                                .font_weight(FontWeight::BOLD)
                                                                .text_color(move || filtered.with(|items| items.get(at.index()).map(|t| ui::method_color(&t.request.method)).unwrap_or(theme().primary))),
                                                            )
                                                            .child(
                                                                ui::label(move || {
                                                                    filtered.with(|items| {
                                                                        items
                                                                            .get(at.index())
                                                                            .map(|t| t.name.clone())
                                                                            .unwrap_or_default()
                                                                    })
                                                                })
                                                                .flex_1()
                                                                .number_of_lines(1),
                                                            ),
                                                    )
                                                    .child(
                                                        ui::muted(move || {
                                                            filtered.with(|items| {
                                                                items
                                                                    .get(at.index())
                                                                    .map(|t| t.request.url.clone())
                                                                    .unwrap_or_default()
                                                            })
                                                        })
                                                        .text_size(px(11.0))
                                                        .number_of_lines(1),
                                                    )
                                            },
                                        )
                                        .estimated_item_size(px(52.0))
                                        .flex_1()
                                        .empty(
                                            view().px_4().py_5().gap_2()
                                                .child(ui::caption("还没有匹配的接口"))
                                                .child(ui::muted("保存请求，建立你的接口集合。").text_size(px(12.0)).number_of_lines(2)),
                                        ),
                                    )
                                    .into_element()
                            } else {
                                virtual_list(
                                    move || filtered_history.with(Vec::len),
                                    move |at| {
                                        button()
                                            .h(px(66.0))
                                            .px_3()
                                            .gap_1()
                                            .justify_center()
                                            .hover(|s| s.bg(|| theme().accent))
                                            .on_press(move || {
                                                if let Some(entry) = filtered_history
                                                    .with_untracked(|h| h.get(at.index()).cloned())
                                                {
                                                    store.open(
                                                        entry.service,
                                                        entry.input,
                                                        None,
                                                        entry.environment,
                                                    );
                                                }
                                            })
                                            .child(
                                                ui::label(move || {
                                                    filtered_history.with(|h| {
                                                        h.get(at.index())
                                                            .map(|e| {
                                                                format!(
                                                                    "{} {}",
                                                                    e.input.request.method,
                                                                    e.input.request.url
                                                                )
                                                            })
                                                            .unwrap_or_default()
                                                    })
                                                })
                                                .number_of_lines(1),
                                            )
                                            .child(
                                                ui::muted(move || {
                                                    filtered_history.with(|h| {
                                                        h.get(at.index())
                                                            .map(|e| e.summary.clone())
                                                            .unwrap_or_default()
                                                    })
                                                })
                                                .text_size(px(11.0))
                                                .number_of_lines(2),
                                            )
                                    },
                                )
                                .estimated_item_size(px(66.0))
                                .flex_1()
                                .empty(
                                    view().p_4().child(ui::muted(
                                        "本机最近 50 次请求；点击可重新编辑。",
                                    )),
                                )
                                .into_element()
                            }
                        }))
                        .child(
                            view()
                                .p_3()
                                .gap_2().border_t_1().border_color(|| theme().border)
                                .child(ui::icon_action(ui::Glyph::Plus, "新建请求", move || store.new_request())
                                    .bg(|| theme().background).border_color(|| theme().input))
                                .child(ui::icon_action(ui::Glyph::Refresh, "刷新集合", move || store.reload())),
                        )
                }))
                .child(
                    view()
                        .flex_1()
                        .min_w(px(0.0))
                        .min_h(px(0.0))
                        .child(
                            view()
                                .h(px(40.0))
                                .flex_row().items_center().px_2().gap_1().bg(|| theme().sidebar)
                                .border_b_1()
                                .border_color(|| theme().border)
                                .child(scroll().horizontal().scrollbar(false).flex_1().child(
                                    view().flex_row().gap_1().child(each(
                                        store.tabs,
                                        |tab| tab.id,
                                        move |value, _| {
                                            let tab = value.get_untracked();
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .h(px(32.0)).rounded(px(7.0)).pr_1()
                                                .border_1().border_color(move || {
                                                    if store.active.get() == tab.id { theme().input } else { AdaptiveColor::TRANSPARENT }
                                                })
                                                .bg(move || {
                                                    if store.active.get() == tab.id { theme().background } else { AdaptiveColor::TRANSPARENT }
                                                })
                                                .child(
                                                    button()
                                                        .h(px(30.0))
                                                        .px_3()
                                                        .justify_center()
                                                        .on_press(move || store.activate(tab))
                                                        .child(
                                                            ui::label(move || {
                                                                tab.draft.with(|d| {
                                                                    format!(
                                                                        "{}  {}{}",
                                                                        d.request.method,
                                                                        d.name,
                                                                        if Workbench::dirty(tab) {
                                                                            " •"
                                                                        } else {
                                                                            ""
                                                                        }
                                                                    )
                                                                })
                                                            })
                                                            .text_size(px(12.0)).font_weight(FontWeight::MEDIUM)
                                                            .text_color(move || if store.active.get() == tab.id { theme().foreground } else { theme().muted_foreground })
                                                            .max_w(px(190.0))
                                                            .number_of_lines(1),
                                                        ),
                                                )
                                                .child(
                                                    ui::ghost("×", move || {
                                                        store.request_close(tab)
                                                    })
                                                    .size(px(24.0)).px_1()
                                                    .accessibility_label(move || {
                                                        tab.draft
                                                            .with(|d| Some(SharedString::from(format!("关闭 {}", d.name))))
                                                    }),
                                                )
                                        },
                                    )),
                                ))
                                .child(
                                    ui::icon_button(ui::Glyph::Plus, "新建请求标签", move || store.new_request()),
                                ),
                        )
                        .child(dynamic(move || {
                            let id = store.active.get();
                            let tab = store
                                .tabs
                                .with_untracked(|tabs| tabs.iter().find(|t| t.id == id).copied());
                            match tab {
                                Some(tab) => nsh_base::v_resizable()
                                    .flex_1().min_h(px(0.0))
                                    .handle_class("h-px bg-border group-hover:bg-primary group-active:bg-primary")
                                    .handle_thumb_class("h-5 w-[3px] rounded-full bg-muted-foreground opacity-0 group-hover:opacity-40")
                                    .child(nsh_base::resizable_panel().min_size(px(260.0)).child(request::request_editor(store, tab)))
                                    .child(nsh_base::resizable_panel().min_size(px(280.0)).child(response::response_view(store, tab)))
                                    .into_element(),
                                None => view().into_element(),
                            }
                        })),
                ),
        )
        .child(
            view()
                .h(px(27.0))
                .px_3()
                .flex_row()
                .items_center()
                .gap_3()
                .border_t_1()
                .border_color(|| theme().border)
                .bg(|| theme().sidebar)
                .child(
                    ui::muted(move || store.notice.get())
                        .flex_1()
                        .number_of_lines(1)
                        .text_size(px(11.0)),
                )
                .child(ui::muted(move || store.recovery_status.get()).max_w(px(320.0)).number_of_lines(1).text_size(px(11.0)))
                .child(ui::muted(if crate::backend::mac_shortcuts() {
                    "⌘↵ 发送   ⌘S 保存   ⌘T 新建"
                } else {
                    "Ctrl+Enter 发送   Ctrl+S 保存   Ctrl+T 新建"
                }).text_size(px(11.0))),
        )
        .child(dialogs::dialogs(store))
}

// Bind after a focused code editor so its editing keymap cannot consume workbench commands.
pub fn register_request_keys(store: Workbench) {
    for (key, action) in [
        ("mod+enter", 0),
        ("mod+s", 1),
        ("mod+t", 2),
        ("mod+w", 3),
        ("escape", 4),
        ("mod+shift+d", 5),
        ("ctrl+tab", 6),
        ("ctrl+shift+tab", 7),
    ] {
        on_key_with(
            key,
            KeyOptions {
                when: nsh_core::keymap::When::Always,
                ..Default::default()
            },
            move |_| {
                if store.dialog.get_untracked() != Dialog::None {
                    return;
                }
                match action {
                    2 => store.new_request(),
                    6 => store.switch_tab(1),
                    7 => store.switch_tab(-1),
                    _ => {
                        if let Some(tab) = store.current() {
                            match action {
                                0 => store.send(tab),
                                1 => store.save(tab, false),
                                3 => store.request_close(tab),
                                4 => {
                                    if tab.response_view.search_open.get_untracked() {
                                        tab.response_view.search_open.set(false);
                                    } else {
                                        store.cancel(tab);
                                    }
                                }
                                5 => store.duplicate(tab),
                                _ => unreachable!(),
                            }
                        }
                    }
                }
            },
        );
    }
    for index in 0..9 {
        on_key_with(
            &format!("mod+{}", index + 1),
            KeyOptions {
                when: nsh_core::keymap::When::Always,
                ..Default::default()
            },
            move |_| {
                if store.dialog.get_untracked() == Dialog::None {
                    store.select_tab(index);
                }
            },
        );
    }
}
