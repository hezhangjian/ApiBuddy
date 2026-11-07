#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

fn main() -> impl std::process::Termination {
    platform::set_ui_sets(vec![nsh_base::registry(), apibuddy_app::registry()]);
    let theme =
        nsh_base::Theme::from_json(include_str!("../theme.json")).expect("valid bundled theme");
    platform::run_app(
        platform::AppSource::Rust(
            platform::Root::of::<apibuddy_app::App>()
                .tokens(theme.into_tokens())
                .app_id("io.github.hezhangjian.apibuddy"),
        ),
        platform::AppOptions {
            title: "ApiBuddy".into(),
            app_id: "io.github.hezhangjian.apibuddy".into(),
            size: (1360, 900),
            ..Default::default()
        },
    )
}
