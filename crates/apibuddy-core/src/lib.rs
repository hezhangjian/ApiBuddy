//! Shared ApiBuddy domain: templates, services, storage, HTTP, and CLI helpers.

pub mod cli;
pub mod curl;
pub mod environments;
pub mod error;
pub mod http;
pub mod paths;
pub mod resolve;
pub mod services;
pub mod store;
pub mod templates;

pub use environments::{
    Environment, Environments, apply_environment, empty_environments, select_environment,
};
pub use error::{Error, Result};
pub mod protocol;
pub use protocol::{Event, Outcome};
pub mod host;
pub use http::{Engine, MAX_RESPONSE_BYTES, Operation};
pub use paths::data_directory;
pub use resolve::resolve_request;
pub use services::{
    AgentBundle, AgentCommand, CliConfig, Service, ServiceCatalog, ServiceInput, default_service,
};
pub use store::{EnvironmentStore, ServiceStore, TemplateStore};
pub use templates::{
    Authentication, METHODS, Method, Pair, RequestDraft, ResolvedRequest, SavedTemplate,
    TemplateInput, Variable, new_template, referenced_variables,
};
