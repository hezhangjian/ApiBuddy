use apibuddy_core::{Result, protocol::Command};
use serde::de::DeserializeOwned;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Backend {
    #[cfg(not(target_arch = "wasm32"))]
    host: std::rc::Rc<std::cell::RefCell<apibuddy_core::host::Host>>,
}

impl Backend {
    pub fn new(_root: PathBuf) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            host: std::rc::Rc::new(std::cell::RefCell::new(apibuddy_core::host::Host::new(
                _root,
            ))),
        }
    }

    pub async fn call<T: DeserializeOwned>(&self, command: Command) -> Result<T> {
        #[cfg(not(target_arch = "wasm32"))]
        let value = self.host.borrow_mut().call(command)?;
        #[cfg(target_arch = "wasm32")]
        let value = web::call(command).await?;
        Ok(serde_json::from_value(value)?)
    }
}

pub fn mac_shortcuts() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web::is_mac()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        cfg!(target_os = "macos")
    }
}

pub fn copy_text(text: String) {
    #[cfg(not(target_arch = "wasm32"))]
    nsh_core::prelude::set_clipboard(text);
    #[cfg(target_arch = "wasm32")]
    web::copy_text(&text);
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(
        inline_js = "export function apiCall(text) { return window.apibuddy.call(text); } export function apiCopy(text) { window.apibuddy.copy(text); } export function apiMac() { return /Mac/.test(navigator.platform); }"
    )]
    extern "C" {
        #[wasm_bindgen(js_name = apiCall, catch)]
        fn api_call(text: &str) -> std::result::Result<js_sys::Promise, JsValue>;
        #[wasm_bindgen(js_name = apiMac)]
        pub fn is_mac() -> bool;
        #[wasm_bindgen(js_name = apiCopy)]
        pub fn copy_text(text: &str);
    }

    pub async fn call(command: Command) -> Result<serde_json::Value> {
        let error = |v: JsValue| {
            apibuddy_core::Error::msg(v.as_string().unwrap_or_else(|| "本地后端连接失败".into()))
        };
        let promise = api_call(&serde_json::to_string(&command)?).map_err(error)?;
        let value = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(error)?;
        let text = value
            .as_string()
            .ok_or_else(|| apibuddy_core::Error::msg("本地后端返回无效数据"))?;
        let result: std::result::Result<serde_json::Value, String> = serde_json::from_str(&text)?;
        result.map_err(apibuddy_core::Error::msg)
    }
}
