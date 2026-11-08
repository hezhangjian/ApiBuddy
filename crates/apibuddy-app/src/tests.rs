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
        app.wait_for("id:请求名称").unwrap();
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

fn copy_curl(app: &mut Headless) {
    app.press("复制 cURL").unwrap();
    app.wait_for("已复制当前环境下的 cURL（含请求头与实际变量值）")
        .unwrap();
}

fn body_text(app: &mut Headless) -> String {
    copy_curl(app);
    apibuddy_core::curl::import(app.screen().clipboard().unwrap())
        .unwrap()
        .request
        .body
}

fn replace_body(app: &mut Headless, text: &str) {
    app.type_text("id:请求正文", "").unwrap();
    app.press_key("mod+a").unwrap();
    app.paste("id:请求正文", &nsh_core::PasteEvent::new(text))
        .unwrap();
}

#[test]
fn save_reopen_and_keep_independent_tab_drafts() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    fill(&mut app, "id:请求名称", "查询订单");
    fill(&mut app, "id:请求 URL", "http://127.0.0.1:42427/orders");
    app.press("label:保存请求").unwrap();
    app.press("保存到集合").unwrap();
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
    app.wait_for("草稿已保存在本机").unwrap();
    let persisted = apibuddy_core::store::DesktopStore::new(&sandbox.0)
        .workspace("native")
        .unwrap();
    assert_eq!(persisted.tabs[persisted.active].input.name, "草稿二");
    drop(app);
    let mut restarted = sandbox.start();
    restarted.wait_for("草稿已恢复").unwrap();
    assert_eq!(
        restarted.screen().input_text("id:请求名称").unwrap(),
        Some("草稿二")
    );
    assert_eq!(
        restarted.screen().input_text("id:请求 URL").unwrap(),
        Some("")
    );
    drop(restarted);
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
    app.type_text("id:cURL 命令", "curl -sS 'http://127.0.0.1:42427/echo' --json '{\"name\":\"中文\"}' -usample:test-only -m1.5").unwrap();
    app.press("导入请求").unwrap();
    app.wait_for("POST  导入的请求 •").unwrap();
    app.press("Body").unwrap();
    copy_curl(&mut app);
    assert_eq!(
        apibuddy_core::curl::import(app.screen().clipboard().unwrap())
            .unwrap()
            .request
            .body,
        "{\"name\":\"中文\"}"
    );
    app.press("Authorization").unwrap();
    assert_eq!(
        app.screen().input_text("id:Basic 用户名").unwrap(),
        Some("sample")
    );
    assert_eq!(
        app.screen().input_text("id:Basic 密码").unwrap(),
        Some("test-only")
    );
    copy_curl(&mut app);
    let clipboard = app.screen().clipboard().unwrap();
    let input = apibuddy_core::curl::import(clipboard).unwrap();
    assert_eq!(input.request.method, "POST");
    assert_eq!(input.request.body, "{\"name\":\"中文\"}");
    assert_eq!(input.request.timeout_ms, 1500);
    assert!(
        input
            .request
            .headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("authorization")
                && h.value == "Basic c2FtcGxlOnRlc3Qtb25seQ==")
    );
    app.press("label:保存请求").unwrap();
    app.press("保存到集合").unwrap();
    app.wait_for("已保存 · 修订 1").unwrap();
}

#[test]
fn import_preview_rejects_lost_body_and_recovers_after_correction() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    app.press("导入 cURL").unwrap();
    fill(
        &mut app,
        "id:cURL 命令",
        "curl https://example.com -XGET -d q=value",
    );
    app.wait_for("GET / HEAD 不支持正文；查询参数请使用 -G / --get")
        .unwrap();
    assert!(app.press("导入请求").unwrap_err().contains("disabled"));
    // A rejected import keeps the dialog and the original request untouched.
    assert!(app.screen().input_text("id:cURL 命令").unwrap().is_some());
    fill(
        &mut app,
        "id:cURL 命令",
        "curl 'https://example.com/echo?fixed=1#part' -G --data-urlencode 'q=中文 &+'",
    );
    app.press("导入请求").unwrap();
    app.wait_for("GET  导入的请求 •").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求 URL").unwrap(),
        Some("https://example.com/echo?fixed=1&q=%E4%B8%AD%E6%96%87+%26%2B#part")
    );
    copy_curl(&mut app);
    let input = apibuddy_core::curl::import(app.screen().clipboard().unwrap()).unwrap();
    assert!(input.request.body.is_empty());
    assert_eq!(input.request.method, "GET");
}

#[test]
fn service_and_environment_created_through_ui_survive_dialog_close() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    app.press("label:新建集合").unwrap();
    app.type_text("id:服务名称", "订单服务").unwrap();
    app.type_text("id:服务 ID", "orders").unwrap();
    app.type_text("id:服务地址", "http://127.0.0.1:42427")
        .unwrap();
    app.press("创建集合").unwrap();
    app.wait_for("集合已创建").unwrap();
    app.press("label:新建请求标签").unwrap();
    fill(&mut app, "id:请求名称", "get-order");
    app.press("label:保存请求").unwrap();
    app.press("保存到集合").unwrap();
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

#[test]
fn body_modes_preserve_raw_and_disabled_form_fields_across_restart() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    app.press("导入 cURL").unwrap();
    app.type_text("id:cURL 命令", "curl http://localhost/echo -m 1.5 -H 'Content-Type: application/json' -d '{\"name\":\"中文\"}'").unwrap();
    app.press("导入请求").unwrap();
    app.wait_for("POST  导入的请求 •").unwrap();
    app.press("设置").unwrap();
    assert_eq!(app.screen().input_text("id:超时秒数").unwrap(), Some("1.5"));
    app.press("Body").unwrap();
    app.press("Form").unwrap();
    app.press("label:添加Form 字段").unwrap();
    fill(&mut app, "id:Form 名称", "name");
    fill(&mut app, "id:Form 值", "中文 & + {{unused}}");
    app.press("label:启用Form 字段 1").unwrap();
    app.press("JSON").unwrap();
    copy_curl(&mut app);
    assert_eq!(
        apibuddy_core::curl::import(app.screen().clipboard().unwrap())
            .unwrap()
            .request
            .body,
        "{\"name\":\"中文\"}"
    );
    copy_curl(&mut app);
    let input = apibuddy_core::curl::import(app.screen().clipboard().unwrap()).unwrap();
    assert_eq!(input.request.body, "{\"name\":\"中文\"}");
    app.press("label:保存请求").unwrap();
    app.press("保存到集合").unwrap();
    app.wait_for("已保存 · 修订 1").unwrap();
    app.wait_for("草稿已保存在本机").unwrap();
    drop(app);
    let mut app = sandbox.start();
    app.wait_for("草稿已恢复").unwrap();
    app.press("Body").unwrap();
    app.press("Form").unwrap();
    assert_eq!(
        app.screen().input_text("id:Form 值").unwrap(),
        Some("中文 & + {{unused}}")
    );
    copy_curl(&mut app);
    let input = apibuddy_core::curl::import(app.screen().clipboard().unwrap()).unwrap();
    assert_eq!(input.request.body, "");
    fill(&mut app, "id:Form 值", "中文 & +");
    app.press("label:启用Form 字段 1").unwrap();
    copy_curl(&mut app);
    let input = apibuddy_core::curl::import(app.screen().clipboard().unwrap()).unwrap();
    assert_eq!(input.request.body, "name=%E4%B8%AD%E6%96%87+%26+%2B");
    let saved = apibuddy_core::TemplateStore::new(&sandbox.0)
        .list()
        .unwrap()
        .templates;
    assert_eq!(
        saved[0].request.form_draft.as_ref().unwrap()[0].enabled,
        false
    );
    assert_eq!(saved[0].request.body, "{\"name\":\"中文\"}");
}

#[test]
fn response_search_and_json_fields_work_across_views_and_tabs() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/echo", listener.local_addr().unwrap());
    // The field is beyond the preview boundary; extraction and copy must still use the full body.
    let body = format!(
        r#"{{"message":"İ 😀中文 Ab aB 中文","padding":"{}","data":[null,{{"a/b":"尾部字段"}}]}}"#,
        "x".repeat(270000)
    );
    let reply_body = body.clone();
    let server = std::thread::spawn(move || {
        let body = reply_body;
        let (mut connection, _) = listener.accept().unwrap();
        connection
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut buffer = [0; 4096];
        connection.read(&mut buffer).unwrap();
        write!(connection, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Search: 中文\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    fill(&mut app, "id:请求名称", "查找测试");
    fill(&mut app, "id:请求 URL", &url);
    app.press_key("mod+enter").unwrap();
    app.wait_for("200 OK").unwrap();
    app.press("Raw").unwrap();
    app.press_key("mod+f").unwrap();
    app.wait_for("id:查找响应文本").unwrap();
    fill(&mut app, "id:查找响应文本", "中文");
    app.wait_for("2 处匹配").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for("1 / 2").unwrap();
    app.press_key("mod+g").unwrap();
    app.wait_for("2 / 2").unwrap();
    app.press_key("mod+g").unwrap();
    app.wait_for("1 / 2").unwrap();
    app.press_key("shift+mod+g").unwrap();
    app.wait_for("2 / 2").unwrap();
    fill(&mut app, "id:查找响应文本", "Ab");
    app.wait_for("2 处匹配").unwrap();
    app.press("label:区分大小写").unwrap();
    app.wait_for("1 处匹配").unwrap();
    app.press("响应头").unwrap();
    fill(&mut app, "id:查找响应文本", "Content-Type");
    app.wait_for("0 处匹配").unwrap();
    app.press("label:区分大小写").unwrap();
    app.wait_for("1 处匹配").unwrap();
    app.press("Raw").unwrap();
    app.press("JSON 字段").unwrap();
    fill(&mut app, "id:JSON 字段路径", "/data/0");
    app.press("复制字段 JSON").unwrap();
    assert_eq!(app.screen().clipboard().unwrap(), "null");
    fill(&mut app, "id:JSON 字段路径", "/data/3");
    app.wait_for("未找到此 JSON 字段").unwrap();
    fill(&mut app, "id:JSON 字段路径", "/data/1/a~1b");
    app.press("复制字段 JSON").unwrap();
    assert_eq!(app.screen().clipboard().unwrap(), "\"尾部字段\"");
    fill(&mut app, "id:查找响应文本", "尾部");
    app.press("label:下一处匹配").unwrap();
    app.wait_for("1 / 1").unwrap();
    app.press("label:新建请求标签").unwrap();
    app.wait_for("等待第一条响应").unwrap();
    app.press("GET  查找测试 •").unwrap();
    assert_eq!(
        app.screen().input_text("id:JSON 字段路径").unwrap(),
        Some("/data/1/a~1b")
    );
    assert_eq!(
        app.screen().input_text("id:查找响应文本").unwrap(),
        Some("尾部")
    );
    app.wait_for("1 / 1").unwrap();
    app.press("复制响应").unwrap();
    assert_eq!(app.screen().clipboard().unwrap(), body);
    app.press_key("escape").unwrap();
    app.press("label:关闭 JSON 字段").unwrap();
    app.press("复制响应").unwrap();
    assert_eq!(app.screen().clipboard().unwrap(), body);
    server.join().unwrap();
}

#[test]
fn native_body_editor_formats_undoes_and_keeps_workbench_shortcuts() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    app.press("导入 cURL").unwrap();
    app.type_text(
        "id:cURL 命令",
        "curl http://localhost/echo --json '{\"name\":\"中文😀\",\"count\":1}'",
    )
    .unwrap();
    app.press("导入请求").unwrap();
    app.press("Body").unwrap();
    let original = body_text(&mut app);
    app.press("格式化").unwrap();
    let formatted = body_text(&mut app);
    assert!(formatted.contains('\n'));
    app.type_text("id:请求正文", "").unwrap();
    app.press_key("mod+z").unwrap();
    assert_eq!(body_text(&mut app), original);
    app.type_text("id:请求正文", "").unwrap();
    app.press_key("mod+shift+z").unwrap();
    assert_eq!(body_text(&mut app), formatted);
    app.press("压缩").unwrap();
    let compact = body_text(&mut app);
    assert!(!compact.contains('\n'));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&compact).unwrap()["name"],
        "中文😀"
    );
    replace_body(&mut app, "{\n  \"name\": \"中文\",\n}");
    app.press("格式化").unwrap();
    app.wait_for("JSON 格式错误：第 3 行，第 1 列 · trailing comma at line 3 column 1")
        .unwrap();
    assert_eq!(body_text(&mut app), "{\n  \"name\": \"中文\",\n}");
    replace_body(&mut app, &original);
    app.press_key("mod+s").unwrap();
    app.wait_for("保存到集合").unwrap();
    app.press("保存到集合").unwrap();
    app.wait_for("已保存 · 修订 1").unwrap();
    fill(&mut app, "id:请求 URL", "invalid-url");
    app.type_text("id:请求正文", "").unwrap();
    app.press_key("mod+enter").unwrap();
    app.wait_for("请输入有效的绝对 URL").unwrap();
}

#[test]
fn duplicate_and_keyboard_navigation_keep_independent_drafts_and_close_the_neighbor() {
    let sandbox = Sandbox::new();
    let mut app = sandbox.start();
    fill(&mut app, "id:请求名称", "原请求");
    fill(&mut app, "id:请求 URL", "http://localhost/original");
    app.press("label:保存请求").unwrap();
    app.press("保存到集合").unwrap();
    app.wait_for("已保存 · 修订 1").unwrap();
    app.press_key("mod+shift+d").unwrap();
    app.wait_for("GET  原请求 副本 •").unwrap();
    fill(&mut app, "id:请求 URL", "http://localhost/copy");
    app.press_key("ctrl+shift+tab").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求 URL").unwrap(),
        Some("http://localhost/original")
    );
    app.press_key("ctrl+tab").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求 URL").unwrap(),
        Some("http://localhost/copy")
    );
    app.press("label:保存请求").unwrap();
    app.press("保存到集合").unwrap();
    app.wait_for("已保存 · 修订 1").unwrap();
    app.press_key("mod+t").unwrap();
    fill(&mut app, "id:请求名称", "邻居");
    app.press_key("mod+t").unwrap();
    fill(&mut app, "id:请求名称", "末尾");
    app.press_key("mod+2").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求名称").unwrap(),
        Some("原请求 副本")
    );
    app.press_key("mod+w").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求名称").unwrap(),
        Some("邻居")
    );
    app.press_key("mod+9").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求名称").unwrap(),
        Some("末尾")
    );
    app.wait_for("草稿已保存在本机").unwrap();
    drop(app);
    let templates = apibuddy_core::TemplateStore::new(&sandbox.0)
        .list()
        .unwrap()
        .templates;
    assert_eq!(templates.len(), 2);
    assert!(
        templates
            .iter()
            .any(|t| t.name == "原请求" && t.request.url.ends_with("/original"))
    );
    assert!(
        templates
            .iter()
            .any(|t| t.name == "原请求 副本" && t.request.url.ends_with("/copy"))
    );
    let mut app = sandbox.start();
    app.wait_for("草稿已恢复").unwrap();
    assert_eq!(
        app.screen().input_text("id:请求名称").unwrap(),
        Some("末尾")
    );
}
