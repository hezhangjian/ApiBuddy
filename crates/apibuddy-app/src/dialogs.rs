use crate::{
    state::{Dialog, Workbench},
    ui,
};
use apibuddy_core::{Environment, ServiceInput, Variable, curl};
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
    })
}

fn panel(store: Workbench, title: &'static str, content: impl IntoElement) -> impl IntoElement {
    dialog()
        .on_cancel(move || store.dialog.set(Dialog::None))
        .child(
            view()
                .w(px(650.0))
                .max_h(px(720.0))
                .p_5()
                .gap_4()
                .rounded(px(10.0))
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
                            ui::action("×", move || store.dialog.set(Dialog::None))
                                .accessibility_label("关闭对话框"),
                        ),
                )
                .child(content)
                .child(ui::muted(move || store.notice.get()).number_of_lines(3)),
        )
}

fn service(store: Workbench) -> impl IntoElement {
    let name = signal(String::new());
    let slug = signal(String::new());
    let base = signal(String::new());
    panel(
        store,
        "新建服务",
        view()
            .gap_3()
            .child(ui::label("名称"))
            .child(ui::field("服务名称", name, move |v| name.set(v.into())).placeholder("订单服务"))
            .child(ui::label("服务 ID"))
            .child(ui::field("服务 ID", slug, move |v| slug.set(v.into())).placeholder("orders"))
            .child(ui::muted("用于区分服务的唯一标识，例如 orders。"))
            .child(ui::label("基础地址"))
            .child(
                ui::field("服务地址", base, move |v| base.set(v.into()))
                    .placeholder("http://127.0.0.1:42427"),
            )
            .child(ui::primary("创建服务", move || {
                if store.create_service(ServiceInput {
                    name: name.get_untracked(),
                    slug: slug.get_untracked(),
                    base_url: base.get_untracked(),
                    description: String::new(),
                }) {
                    store.dialog.set(Dialog::None);
                }
            })),
    )
}

fn import(store: Workbench) -> impl IntoElement {
    let source = signal(String::new());
    let preview = computed(move || {
        if source.with(String::is_empty) {
            return "粘贴一条 cURL 命令，检查后导入为新标签。".into();
        }
        match curl::import(&source.get()) {
            Ok(input) => format!(
                "{} {} · {} 个请求头 · {} B 正文",
                input.request.method,
                input.request.url,
                input.request.headers.len(),
                input.request.body.len()
            ),
            Err(error) => error.to_string(),
        }
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
            .child(ui::label(move || preview.get()).number_of_lines(4))
            .child(ui::muted(
                "支持 URL、-X、-H、-d / --data-raw、-I、--max-time。导入不会发送请求。",
            ))
            .child(ui::primary("导入请求", move || {
                match curl::import(&source.get_untracked()) {
                    Ok(input) => {
                        store.open(store.service(), input, None, None);
                        store.dialog.set(Dialog::None);
                    }
                    Err(error) => store.notice.set(error.to_string()),
                }
            })),
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
                if store.save_environments(data.get_untracked()) {
                    store.dialog.set(Dialog::None);
                }
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
                            if store.save(tab, false) {
                                store.close(id);
                            }
                        }
                    })),
            ),
    )
}
