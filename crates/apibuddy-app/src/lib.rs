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

/// The shared Rust workbench, rendered natively or in WebAssembly.
#[component]
pub fn App() -> impl IntoElement {
    #[cfg(target_arch = "wasm32")]
    let directory: apibuddy_core::Result<std::path::PathBuf> = Ok(Default::default());
    #[cfg(not(target_arch = "wasm32"))]
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
