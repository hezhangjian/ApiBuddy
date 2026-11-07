use super::*;
use nsh_core::{
    element::Root,
    platform::{Idiom, Os},
};
use nsh_engine::EngineConfig;
use nsh_testing::Headless;
use std::path::PathBuf;

#[derive(Clone, PartialEq, Props)]
struct HarnessProps {
    root: SharedString,
}

#[component]
fn Harness(props: HarnessProps) -> impl IntoElement {
    workbench::workbench(state::Workbench::new(PathBuf::from(props.root.as_str())))
}

struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("apibuddy-ui-{}", uuid::Uuid::new_v4())))
    }
    fn start(&self) -> Headless {
        let root = self.0.to_string_lossy().to_string();
        let theme = nsh_base::Theme::from_json(include_str!("../theme.json")).unwrap();
        let mut config = EngineConfig {
            viewport: size(px(1360.0), px(900.0)),
            ui_sets: vec![nsh_base::registry()],
            ..EngineConfig::new(
                Root::new::<Harness>(move || HarnessProps {
                    root: root.clone().into(),
                })
                .tokens(theme.into_tokens()),
            )
        };
        config.environment.os = Os::MacOs;
        config.environment.idiom = Idiom::Desktop;
        let mut app = Headless::start(config).unwrap();
        app.wait_for("ApiBuddy").unwrap();
        app
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fill(app: &mut Headless, name: &str, value: &str) {
    app.clear(name).unwrap();
    app.type_text(name, value).unwrap();
}

#[test]
fn save_reopen_and_keep_independent_tab_drafts() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    fill(&mut app, "id:请求名称", "查询订单");
    fill(&mut app, "id:请求 URL", "http://127.0.0.1:42427/orders");
    app.press("label:保存请求").unwrap();
    app.wait_for("已保存 · 修订 1").unwrap();
    app.press("label:新建请求标签").unwrap();
    fill(&mut app, "id:请求名称", "草稿二");
    app.press("GET  查询订单").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求 URL").unwrap(),
        Some("http://127.0.0.1:42427/orders")
    );
    app.press("GET  草稿二 •").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求名称").unwrap(),
        Some("草稿二")
    );
    app.press("label:关闭 草稿二").unwrap();
    app.wait_for("保存请求修改？").unwrap();
    app.press("继续编辑").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求名称").unwrap(),
        Some("草稿二")
    );
    drop(app);
    let templates = apibuddy_core::TemplateStore::new(&sandbox.0)
        .list()
        .unwrap()
        .templates;
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].name, "查询订单");
}

#[test]
fn import_dialog_owns_no_tab_state_and_clipboard_matches_request() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    app.press("导入 cURL").unwrap();
    app.type_text("id:cURL 命令", "curl 'http://127.0.0.1:42427/echo' -X POST -H 'Content-Type: application/json' --data-raw '{\"name\":\"中文\"}'").unwrap();
    app.press("导入请求").unwrap();
    app.wait_for("POST  导入的请求 •").unwrap();
    app.press("Body").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求正文").unwrap(),
        Some("{\"name\":\"中文\"}")
    );
    app.press("复制 cURL").unwrap();
    let clipboard = app.screen().clipboard().unwrap();
    let input = apibuddy_core::curl::import(clipboard).unwrap();
    assert_eq!(input.request.method, "POST");
    assert_eq!(input.request.body, "{\"name\":\"中文\"}");
    app.press("label:保存请求").unwrap();
    app.wait_for("已保存 · 修订 1").unwrap();
}

#[test]
fn service_and_environment_created_through_ui_survive_dialog_close() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    app.press("label:新建服务").unwrap();
    app.type_text("id:服务名称", "订单服务").unwrap();
    app.type_text("id:服务 ID", "orders").unwrap();
    app.type_text("id:服务地址", "http://127.0.0.1:42427")
        .unwrap();
    app.press("创建服务").unwrap();
    app.wait_for("服务已创建").unwrap();
    fill(&mut app, "id:请求名称", "get-order");
    app.press("label:保存请求").unwrap();
    app.press("label:管理环境").unwrap();
    app.press("保存环境").unwrap();
    app.wait_for("环境已保存").unwrap();
    let store = apibuddy_core::ServiceStore::with_root(&sandbox.0);
    let service = store
        .list()
        .unwrap()
        .services
        .into_iter()
        .find(|s| s.slug == "orders")
        .unwrap();
    let env = apibuddy_core::EnvironmentStore::for_service(&store, &service.id)
        .unwrap()
        .read()
        .unwrap();
    assert_eq!(env.profiles[0].name, "local");
    assert_eq!(
        store
            .templates(&service.id)
            .unwrap()
            .list()
            .unwrap()
            .templates[0]
            .name,
        "get-order"
    );
}

#[test]
fn send_shortcut_works_while_url_input_has_focus() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    fill(&mut app, "id:请求 URL", "invalid-url");
    app.press_key("mod+enter").unwrap();
    app.wait_for("请输入有效的绝对 URL").unwrap();
}
