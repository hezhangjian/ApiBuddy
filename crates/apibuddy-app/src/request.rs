use crate::{
    state::{RequestTab, Workbench},
    ui,
};
use apibuddy_core::{METHODS, Pair, Variable};
use nsh_base::{SelectItem, theme};
use nsh_core::prelude::*;

pub fn request_editor(store: Workbench, tab: RequestTab) -> impl IntoElement {
    let timeout = signal((tab.draft.with_untracked(|d| d.request.timeout_ms) / 1000).to_string());
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
                    ui::field(
                        "请求名称",
                        move || tab.draft.with(|d| d.name.clone()),
                        move |value| tab.draft.update(|d| d.name = value.into()),
                    )
                    .w(px(230.0)),
                )
                .child(
                    ui::muted(move || tab.service.with(|s| format!("/ {}", s.name)))
                        .flex_1()
                        .number_of_lines(1),
                )
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
                    .w(px(155.0)),
                )
                .child(
                    ui::action("保存", move || {
                        store.save(tab, false);
                    })
                    .accessibility_label("保存请求"),
                )
                .child(ui::action("另存", move || {
                    store.save(tab, true);
                })),
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
                    .w(px(112.0)),
                )
                .child(
                    ui::field(
                        "请求 URL",
                        move || tab.draft.with(|d| d.request.url.clone()),
                        move |value| tab.draft.update(|d| d.request.url = value.into()),
                    )
                    .font_family(ui::MONO)
                    .placeholder("https://api.example.com/path")
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
                    .w(px(90.0))
                    .accessibility_label("发送或取消请求"),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .px_4()
                .child(view().flex_1().child(ui::tabs(
                    &["Params", "Headers", "Body", "认证", "变量", "设置"],
                    tab.section,
                )))
                .child(ui::action("复制 cURL", move || store.copy_curl(tab))),
        )
        .child(dynamic(move || match tab.section.get() {
            0 => pairs(tab, false).into_element(),
            1 => pairs(tab, true).into_element(),
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
                        let seconds = value.parse::<u64>().unwrap_or(0);
                        tab.draft
                            .update(|d| d.request.timeout_ms = seconds.saturating_mul(1000));
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

fn pairs(tab: RequestTab, headers: bool) -> impl IntoElement {
    let rows = computed(move || {
        tab.draft.with(|d| {
            (0..if headers {
                d.request.headers.len()
            } else {
                d.request.query.len()
            })
                .collect::<Vec<_>>()
        })
    });
    let title = if headers { "请求头" } else { "查询参数" };
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
                    ui::muted(if headers {
                        "Name / Value · 支持 {{变量}}"
                    } else {
                        "参数会编码后追加到 URL，支持重复参数"
                    })
                    .flex_1(),
                )
                .child(
                    ui::action("+ 添加", move || {
                        tab.draft.update(|d| {
                            let rows = if headers {
                                &mut d.request.headers
                            } else {
                                &mut d.request.query
                            };
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
                                    let rows = if headers {
                                        &d.request.headers
                                    } else {
                                        &d.request.query
                                    };
                                    if rows.get(i).is_some_and(|r| r.enabled) {
                                        "☑"
                                    } else {
                                        "☐"
                                    }
                                })
                            },
                            move || {
                                tab.draft.update(|d| {
                                    let rows = if headers {
                                        &mut d.request.headers
                                    } else {
                                        &mut d.request.query
                                    };
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
                            if headers {
                                "请求头名称"
                            } else {
                                "参数名称"
                            },
                            move || {
                                tab.draft.with(|d| {
                                    let rows = if headers {
                                        &d.request.headers
                                    } else {
                                        &d.request.query
                                    };
                                    rows.get(i).map(|r| r.key.clone()).unwrap_or_default()
                                })
                            },
                            move |value| {
                                tab.draft.update(|d| {
                                    let rows = if headers {
                                        &mut d.request.headers
                                    } else {
                                        &mut d.request.query
                                    };
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
                            if headers { "请求头值" } else { "参数值" },
                            move || {
                                tab.draft.with(|d| {
                                    let rows = if headers {
                                        &d.request.headers
                                    } else {
                                        &d.request.query
                                    };
                                    rows.get(i).map(|r| r.value.clone()).unwrap_or_default()
                                })
                            },
                            move |value| {
                                tab.draft.update(|d| {
                                    let rows = if headers {
                                        &mut d.request.headers
                                    } else {
                                        &mut d.request.query
                                    };
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
                                let rows = if headers {
                                    &mut d.request.headers
                                } else {
                                    &mut d.request.query
                                };
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
                            } else {
                                "原始正文 · 支持 {{变量}}"
                            }
                        })
                    })
                    .flex_1(),
                )
                .child(ui::action("JSON", move || {
                    set_header(tab, "Content-Type", "application/json")
                }))
                .child(ui::action("Text", move || {
                    set_header(tab, "Content-Type", "text/plain")
                }))
                .child(ui::action("Form", move || {
                    set_header(tab, "Content-Type", "application/x-www-form-urlencoded")
                }))
                .child(ui::action(
                    "格式化",
                    move || match serde_json::from_str::<serde_json::Value>(
                        &tab.draft.with_untracked(|d| d.request.body.clone()),
                    ) {
                        Ok(json) => tab.draft.update(|d| {
                            d.request.body = serde_json::to_string_pretty(&json).unwrap_or_default()
                        }),
                        Err(error) => store.notice.set(format!("JSON 格式错误：{error}")),
                    },
                )),
        )
        .child(
            ui::editor(
                "请求正文",
                move || tab.draft.with(|d| d.request.body.clone()),
                move |value| tab.draft.update(|d| d.request.body = value.into()),
            )
            .placeholder("{\n  \"name\": \"ApiBuddy\"\n}"),
        )
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
    view()
        .flex_1()
        .p_4()
        .gap_3()
        .child(ui::label("Bearer Token"))
        .child(
            ui::field(
                "Bearer Token",
                move || {
                    tab.draft.with(|d| {
                        d.request
                            .headers
                            .iter()
                            .find(|h| h.key.eq_ignore_ascii_case("authorization"))
                            .map(|h| {
                                h.value
                                    .strip_prefix("Bearer ")
                                    .unwrap_or(&h.value)
                                    .to_string()
                            })
                            .unwrap_or_default()
                    })
                },
                move |value| {
                    set_header(
                        tab,
                        "Authorization",
                        &if value.is_empty() {
                            String::new()
                        } else {
                            format!("Bearer {value}")
                        },
                    )
                },
            )
            .placeholder("{{token}}"),
        )
        .child(ui::muted(
            "可填写环境变量 {{token}}。Basic、API Key 等认证可在 Headers 中填写。",
        ))
        .child(view().flex_row().child(ui::action("移除认证", move || {
            set_header(tab, "Authorization", "")
        })))
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

pub fn response_view(store: Workbench, tab: RequestTab) -> impl IntoElement {
    let section = signal(0);
    let content = computed(move || {
        tab.content_version.get();
        let selected = section.get();
        tab.response.with_untracked(|r| {
            let value = match selected {
                2 => r.headers.clone(),
                0 if !r.running && r.body.len() <= 256 * 1024 => {
                    serde_json::from_str::<serde_json::Value>(&r.body)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                        .unwrap_or_else(|| r.body.clone())
                }
                _ => r.body.clone(),
            };
            if value.len() > 256 * 1024 {
                let end = value.floor_char_boundary(256 * 1024);
                format!(
                    "{}\n\n[界面显示前 256 KiB；复制响应可获得完整内容]",
                    &value[..end]
                )
            } else {
                value
            }
        })
    });
    view()
        .flex_1()
        .min_h(px(120.0))
        .border_t_1()
        .border_color(|| theme().border)
        .child(
            view()
                .h(px(40.0))
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
                    .number_of_lines(1),
                )
                .child(ui::muted(move || {
                    tab.response
                        .with(|r| format!("{} ms  ·  {} B", r.elapsed, r.bytes))
                }))
                .child(ui::action("复制响应", move || {
                    let value = tab.response.with_untracked(|r| {
                        if section.get_untracked() == 2 {
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
                .child(ui::tabs(&["Pretty", "Raw", "响应头"], section)),
        )
        .child(
            view().flex_1().min_h(px(0.0)).px_4().py_2().child(
                ui::editor("响应内容", move || content.get(), |_| {})
                    .readonly(true)
                    .placeholder("发送请求后查看响应正文、状态码、耗时和大小"),
            ),
        )
}
