use crate::{
    state::{Dialog, Workbench},
    ui,
};
use apibuddy_core::{Authentication, Environment, ServiceInput, Variable, curl};
use nsh_base::{SelectItem, dialog, theme};
use nsh_core::prelude::*;

pub fn dialogs(store: Workbench) -> impl IntoElement {
    dynamic(move || match store.dialog.get() {
        Dialog::None => view().into_element(),
        Dialog::Service => service(store).into_element(),
        Dialog::Environment => environments(store).into_element(),
        Dialog::Import => import(store).into_element(),
        #[cfg(feature = "cli-integration")]
        Dialog::Agent => agent(store).into_element(),
        Dialog::Close(id) => close(store, id).into_element(),
        Dialog::Save { id, copy, close } => save(store, id, copy, close).into_element(),
    })
}

fn save(store: Workbench, id: u64, copy: bool, close: bool) -> impl IntoElement {
    let tab = store
        .tabs
        .with_untracked(|tabs| tabs.iter().find(|t| t.id == id).copied());
    let name = signal(
        tab.map(|t| t.draft.get_untracked().name)
            .unwrap_or_default(),
    );
    let selected = signal(
        tab.map(|t| {
            if t.saved.get_untracked().is_some() {
                t.service.get_untracked().id
            } else {
                store.selected_service.get_untracked()
            }
        })
        .unwrap_or_default(),
    );
    let new_name = signal(String::new());
    panel(
        store,
        "保存到集合",
        view()
            .gap_3()
            .child(ui::label("请求名称（修改名称后保存即可重命名）"))
            .child(ui::field("保存名称", name, move |v| name.set(v.into())))
            .child(ui::label("选择集合"))
            .child(ui::choice(
                "保存集合",
                move || {
                    let mut items = store.catalog.with(|c| {
                        c.iter()
                            .map(|s| SelectItem::new(s.id.clone(), s.name.clone()))
                            .collect::<Vec<_>>()
                    });
                    items.push(SelectItem::new("new", "+ 新建集合"));
                    items
                },
                move || selected.get(),
                move || {
                    if selected.get() == "new" {
                        "+ 新建集合".into()
                    } else {
                        store.catalog.with(|c| {
                            c.iter()
                                .find(|s| s.id == selected.get())
                                .map(|s| s.name.clone())
                                .unwrap_or_default()
                        })
                    }
                },
                move |id| selected.set(id),
            ))
            .child(when(
                move || selected.get() == "new",
                move || {
                    ui::field("新集合名称", new_name, move |v| new_name.set(v.into()))
                        .placeholder("例如：本地调试")
                },
            ))
            .child(ui::primary("保存到集合", move || {
                if let Some(tab) = tab {
                    if selected.get_untracked() == "new" {
                        store.save_new_collection(
                            tab,
                            new_name.get_untracked(),
                            name.get_untracked(),
                            close,
                            selected,
                        );
                    } else if let Some(destination) = store.catalog.with_untracked(|c| {
                        c.iter().find(|s| s.id == selected.get_untracked()).cloned()
                    }) {
                        store.save_to(tab, destination, name.get_untracked(), copy, close);
                    }
                }
            })),
    )
}

fn panel(store: Workbench, title: &'static str, content: impl IntoElement) -> impl IntoElement {
    dialog()
        .on_cancel(move || store.dialog.set(Dialog::None))
        .child(
            view()
                .flex_1()
                .bg(rgba(0x111b304d))
                .items_center()
                .justify_center()
                .p_4()
                .child(
                    view()
                        .w(px(650.0))
                        .max_w(percentage(1.0))
                        .max_h(px(720.0))
                        .p_5()
                        .gap_4()
                        .rounded(px(14.0))
                        .shadow_lg()
                        .bg(|| theme().background)
                        .border_1()
                        .border_color(|| theme().border)
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .child(
                                    ui::label(title)
                                        .text_size(px(18.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .flex_1(),
                                )
                                .child(
                                    ui::ghost("×", move || store.dialog.set(Dialog::None))
                                        .accessibility_label("关闭对话框"),
                                ),
                        )
                        .child(content)
                        .child(ui::muted(move || store.notice.get()).number_of_lines(3)),
                ),
        )
}

fn service(store: Workbench) -> impl IntoElement {
    let name = signal(String::new());
    let slug = signal(String::new());
    let base = signal(String::new());
    panel(
        store,
        "新建集合",
        view()
            .gap_3()
            .child(ui::label("名称"))
            .child(ui::field("服务名称", name, move |v| name.set(v.into())).placeholder("订单服务"))
            .child(ui::label("标识（可选，留空自动生成）"))
            .child(ui::field("服务 ID", slug, move |v| slug.set(v.into())).placeholder("orders"))
            .child(ui::label("基础地址（可选，用于 {{base_url}}）"))
            .child(
                ui::field("服务地址", base, move |v| base.set(v.into()))
                    .placeholder("http://127.0.0.1:42427"),
            )
            .child(ui::primary("创建集合", move || {
                store.create_service(ServiceInput {
                    name: name.get_untracked(),
                    slug: slug.get_untracked(),
                    base_url: base.get_untracked(),
                    description: String::new(),
                });
            })),
    )
}

fn import(store: Workbench) -> impl IntoElement {
    let source = signal(String::new());
    let parsed = computed(move || curl::import(&source.get()).map_err(|error| error.to_string()));
    let preview = computed(move || {
        if source.with(String::is_empty) {
            return "粘贴一条 cURL 命令，检查后导入为新标签。".into();
        }
        parsed.with(|result| match result {
            Ok(input) => {
                let request = &input.request;
                let auth = if matches!(request.auth, Authentication::Basic { .. }) {
                    "Basic 认证"
                } else if request
                    .headers
                    .iter()
                    .any(|h| h.key.eq_ignore_ascii_case("authorization"))
                {
                    "请求头认证"
                } else {
                    "无认证"
                };
                format!(
                    "{} {}\n{} 个请求头 · {} B 正文 · {}\n超时 {} 秒 · 导入后手动发送",
                    request.method,
                    request.url,
                    request.headers.len(),
                    request.body.len(),
                    auth,
                    request.timeout_ms as f64 / 1000.0
                )
            }
            Err(error) => error.clone(),
        })
    });
    panel(
        store,
        "导入 cURL",
        view()
            .gap_3()
            .child(
                ui::editor("cURL 命令", source, move |v| source.set(v.into()))
                    .h(px(230.0))
                    .flex_none()
                    .placeholder("curl 'https://api.example.com/users'"),
            )
            .child(ui::label(move || preview.get()).number_of_lines(5))
            .child(ui::muted(
                "支持 -X、-H、-d、--json、--data-urlencode、-G、-u、-I、--max-time；可用 -sS、-XPOST 和 --header=值。",
            ).number_of_lines(3))
            .child(ui::primary("导入请求", move || {
                match parsed.get_untracked() {
                    Ok(input) => {
                        store.open(store.service(), input, None, None);
                        store.dialog.set(Dialog::None);
                    }
                    Err(error) => store.notice.set(error),
                }
            }).disabled(move || parsed.with(Result::is_err))),
    )
}

fn environments(store: Workbench) -> impl IntoElement {
    let data = signal(store.environments.get_untracked());
    let selected = signal(0usize);
    let name = signal(data.with_untracked(|e| {
        e.profiles
            .first()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "local".into())
    }));
    let values = signal(data.with_untracked(|e| {
        e.profiles
            .first()
            .map(|p| serialize_vars(&p.variables))
            .unwrap_or_else(|| "base_url=http://127.0.0.1:42427".into())
    }));
    let stash = move || -> Result<(), String> {
        let variables = parse_vars(&values.get_untracked())?;
        let index = selected.get_untracked();
        data.update(|e| {
            if let Some(profile) = e.profiles.get_mut(index) {
                profile.name = name.get_untracked();
                profile.variables = variables;
            } else {
                e.profiles.push(Environment {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.get_untracked(),
                    variables,
                });
            }
        });
        Ok(())
    };
    panel(
        store,
        "管理环境",
        view()
            .gap_3()
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        ui::choice(
                            "编辑环境",
                            move || {
                                data.with(|e| {
                                    e.profiles
                                        .iter()
                                        .enumerate()
                                        .map(|(i, p)| {
                                            SelectItem::new(i.to_string(), p.name.clone())
                                        })
                                        .collect::<Vec<_>>()
                                })
                            },
                            move || selected.get().to_string(),
                            move || {
                                data.with(|e| {
                                    e.profiles
                                        .get(selected.get())
                                        .map(|p| p.name.clone())
                                        .unwrap_or_else(|| "新环境".into())
                                })
                            },
                            move |index| {
                                if let Err(error) = stash() {
                                    store.notice.set(error);
                                    return;
                                }
                                if let Ok(index) = index.parse::<usize>() {
                                    selected.set(index);
                                    if let Some(profile) =
                                        data.with_untracked(|e| e.profiles.get(index).cloned())
                                    {
                                        name.set(profile.name);
                                        values.set(serialize_vars(&profile.variables));
                                    }
                                }
                            },
                        )
                        .flex_1(),
                    )
                    .child(ui::action("+ 新环境", move || {
                        if let Err(error) = stash() {
                            store.notice.set(error);
                            return;
                        }
                        if data.with_untracked(|e| e.profiles.len()) >= 30 {
                            store.notice.set("每个服务最多 30 个环境".into());
                            return;
                        }
                        selected.set(data.with_untracked(|e| e.profiles.len()));
                        name.set(String::new());
                        values.set(String::new());
                    })),
            )
            .child(
                ui::field("环境名称", name, move |v| name.set(v.into()))
                    .placeholder("local / staging / production"),
            )
            .child(ui::muted(
                "每行 name=value；同名值覆盖模板默认值，值中可包含 =。",
            ))
            .child(
                ui::editor("环境变量", values, move |v| values.set(v.into()))
                    .h(px(230.0))
                    .flex_none(),
            )
            .child(ui::muted("环境值以明文保存在本机。"))
            .child(ui::primary("保存环境", move || {
                if let Err(error) = stash() {
                    store.notice.set(error);
                    return;
                }
                store.save_environments(data.get_untracked());
            })),
    )
}

fn serialize_vars(variables: &[Variable]) -> String {
    variables
        .iter()
        .map(|v| format!("{}={}", v.key, v.value))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_vars(source: &str) -> Result<Vec<Variable>, String> {
    source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| "每行环境变量需为 name=value".to_string())?;
            Ok(Variable {
                key: key.trim().into(),
                value: value.into(),
            })
        })
        .collect()
}

#[cfg(feature = "cli-integration")]
fn agent(store: Workbench) -> impl IntoElement {
    panel(
        store,
        "CLI 与 AI Agent",
        view()
            .gap_3()
            .child(
                ui::editor("Agent Markdown", store.agent_text, |_| {})
                    .readonly(true)
                    .h(px(360.0))
                    .flex_none(),
            )
            .child(ui::primary("复制给 AI Agent", move || {
                set_clipboard(store.agent_text.get_untracked());
                store.notice.set("Agent Markdown 已复制".into());
            })),
    )
}

fn close(store: Workbench, id: u64) -> impl IntoElement {
    panel(
        store,
        "保存请求修改？",
        view()
            .gap_4()
            .child(ui::label(
                "该标签包含未保存的请求，关闭前可保存到接口集合。",
            ))
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(ui::action("继续编辑", move || {
                        store.dialog.set(Dialog::None)
                    }))
                    .child(ui::action("放弃并关闭", move || store.close(id)))
                    .child(ui::primary("保存并关闭", move || {
                        let tab = store
                            .tabs
                            .with_untracked(|tabs| tabs.iter().find(|t| t.id == id).copied());
                        if let Some(tab) = tab {
                            store.save_and_close(tab);
                        }
                    })),
            ),
    )
}
