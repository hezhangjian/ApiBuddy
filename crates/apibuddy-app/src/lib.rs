mod backend;
mod dialogs;
mod request;
mod response;
mod state;
mod ui;
mod workbench;

use nsh_core::prelude::*;

#[native_module("apibuddy/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

/// The native Rust desktop workbench.
#[component]
pub fn App() -> impl IntoElement {
    let directory = apibuddy_core::data_directory();
    match directory {
        Ok(root) => workbench::workbench(state::Workbench::new(root)).into_element(),
        Err(error) => view()
            .p_4()
            .child(text(format!("无法打开数据目录：{error}")))
            .into_element(),
    }
}

#[cfg(test)]
mod tests;
