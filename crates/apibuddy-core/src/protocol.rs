//! Typed boundary shared by the native UI and the WebView UI.
use crate::{Environments, SavedTemplate, Service, ServiceInput, TemplateInput};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Catalog,
    Load {
        service: String,
    },
    Create {
        input: ServiceInput,
    },
    Save {
        service: String,
        input: TemplateInput,
        id: Option<String>,
        revision: Option<u64>,
    },
    SaveEnvironments {
        service: String,
        data: Environments,
    },
    Resolve {
        service: String,
        input: TemplateInput,
        environment: Option<String>,
    },
    Start {
        service: String,
        input: TemplateInput,
        environment: Option<String>,
    },
    Cancel {
        operation: u64,
        discard: bool,
    },
    Poll {
        operations: Vec<u64>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub services: Vec<Service>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub templates: Vec<SavedTemplate>,
    pub environments: Environments,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub command: Command,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Reply {
    pub id: u64,
    pub result: Result<serde_json::Value, String>,
}
pub type Events = Vec<(u64, Vec<Event>)>;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Event {
    Headers {
        status: u16,
        status_text: String,
        headers: String,
        url: String,
    },
    Text {
        text: String,
        bytes: usize,
    },
    End(Outcome),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Outcome {
    Completed,
    Cancelled,
    TimedOut,
    Failed(String),
}
