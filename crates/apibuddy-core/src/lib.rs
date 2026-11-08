//! Shared ApiBuddy domain: templates, services, storage, HTTP, and CLI helpers.

#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
pub mod curl;
pub mod environments;
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod http;
#[cfg(not(target_arch = "wasm32"))]
pub mod paths;
pub mod resolve;
pub mod services;
#[cfg(not(target_arch = "wasm32"))]
pub mod store;
pub mod templates;

pub use environments::{
    Environment, Environments, apply_environment, empty_environments, select_environment,
};
pub use error::{Error, Result};
pub mod protocol;
pub use protocol::{Event, Outcome};
#[cfg(not(target_arch = "wasm32"))]
pub mod host;
#[cfg(not(target_arch = "wasm32"))]
pub use http::{Engine, MAX_RESPONSE_BYTES, Operation};
#[cfg(not(target_arch = "wasm32"))]
pub use paths::data_directory;
pub use resolve::resolve_request;
pub use services::{
    AgentBundle, AgentCommand, CliConfig, Service, ServiceCatalog, ServiceInput, default_service,
};
#[cfg(not(target_arch = "wasm32"))]
pub use store::{EnvironmentStore, ServiceStore, TemplateStore};
pub use templates::{
    Authentication, METHODS, Method, Pair, RequestDraft, ResolvedRequest, SavedTemplate,
    TemplateInput, Variable, new_template, referenced_variables,
};
