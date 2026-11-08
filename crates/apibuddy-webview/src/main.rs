#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod server;

use apibuddy_core::{host::Host, protocol::Command};
use nsh_core::prelude::*;
#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;
use std::{path::PathBuf, sync::OnceLock};

struct Launch {
    url: String,
    token: String,
    root: PathBuf,
}
static LAUNCH: OnceLock<Launch> = OnceLock::new();

#[component]
fn Shell() -> impl IntoElement {
    let launch = LAUNCH.get().expect("initialized launch");
    let host = stored(Host::new(launch.root.clone()));
    let web = element_ref();
    let error = signal(String::new());
    view()
        .flex_1()
        .child(
            web_view(launch.url.clone())
                .flex_1()
                .element_ref(&web)
                .on_load_start(move |_: &WebNavigation| {
                    // Reloading the page ends all operations owned by its previous UI.
                    host.with(|h| *h = Host::new(launch.root.clone()));
                    error.set(String::new());
                })
                .on_error(move |message: &str| error.set(format!("WebView 加载失败：{message}")))
                .on_message(move |message: &str| {
                    if message.len() > 64 * 1024 * 1024 {
                        return;
                    }
                    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(message) else {
                        return;
                    };
                    if value["token"].as_str() != Some(launch.token.as_str()) {
                        return;
                    }
                    if let Some(text) = value["clipboard"].as_str() {
                        set_clipboard(text.to_owned());
                        return;
                    }
                    let Some(id) = value["id"].as_u64() else {
                        return;
                    };
                    let result = serde_json::from_value::<Command>(value["command"].take())
                        .map_err(|_| "无效的本地操作".to_owned())
                        .and_then(|command| {
                            host.with(|host| host.call(command).map_err(|e| e.to_string()))
                        });
                    let reply =
                        serde_json::json!({ "token": launch.token, "id": id, "result": result });
                    web.post_message(reply.to_string());
                }),
        )
        .child(dynamic(move || {
            if error.get().is_empty() {
                view().into_element()
            } else {
                text(error.get()).p_3().into_element()
            }
        }))
}

fn main() -> std::process::ExitCode {
    match launch() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ApiBuddy WebView: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn launch() -> Result<std::process::ExitCode, Box<dyn std::error::Error>> {
    let root = apibuddy_core::data_directory()?;
    let assets = assets_directory()?;
    let server = server::Assets::start(assets)?;
    let token = uuid::Uuid::new_v4().to_string();
    let url = format!("{}index.html#{token}", server.url);
    LAUNCH
        .set(Launch { url, token, root })
        .map_err(|_| "already started")?;
    platform::set_ui_sets(vec![nsh_base::registry()]);
    let result = platform::run_app(
        platform::AppSource::Rust(
            platform::Root::of::<Shell>().app_id("io.github.hezhangjian.apibuddy.webview"),
        ),
        platform::AppOptions {
            title: "ApiBuddy · WebView".into(),
            app_id: "io.github.hezhangjian.apibuddy.webview".into(),
            size: (1360, 900),
            ..Default::default()
        },
    );
    drop(server);
    Ok(result)
}

fn assets_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("APIBUDDY_WEB_ASSETS") {
        return Ok(PathBuf::from(path));
    }
    let exe = std::env::current_exe()?;
    let parent = exe.parent().ok_or("missing executable directory")?;
    for path in [
        parent.join("../Resources/web"),
        parent.join("web"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dist/web"),
    ] {
        if path.join("index.html").is_file() {
            return Ok(path);
        }
    }
    Err("缺少 Web UI 资源，请先运行 python3 scripts/build-webview.py".into())
}
