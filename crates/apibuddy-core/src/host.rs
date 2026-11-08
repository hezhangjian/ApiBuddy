//! Local capability owner. The browser never executes HTTP or chooses file paths.
use crate::protocol::{Catalog, Command, Events, Library};
use crate::{
    Engine, EnvironmentStore, Error, Event, Operation, Result, ServiceStore, apply_environment,
    resolve_request, select_environment,
};
use serde_json::{Value, to_value};
use std::{collections::HashMap, path::PathBuf, sync::mpsc::Receiver};

pub struct Host {
    desktop: crate::store::DesktopStore,
    services: ServiceStore,
    engine: Engine,
    next: u64,
    running: HashMap<u64, (Operation, Receiver<Event>)>,
}

impl Host {
    pub fn new(root: PathBuf) -> Self {
        Self {
            desktop: crate::store::DesktopStore::new(&root),
            services: ServiceStore::with_root(root),
            engine: Engine::new(),
            next: 0,
            running: HashMap::new(),
        }
    }

    pub fn call(&mut self, command: Command) -> Result<Value> {
        Ok(match command {
            Command::History => to_value(self.desktop.history()?)?,
            Command::AppendHistory { entry } => to_value(self.desktop.append_history(entry)?)?,
            Command::Workspace { client } => to_value(self.desktop.workspace(&client)?)?,
            Command::SaveWorkspace { client, data } => {
                to_value(self.desktop.save_workspace(&client, data)?)?
            }
            Command::Catalog => {
                let c = self.services.list()?;
                to_value(Catalog {
                    services: c.services,
                    warnings: c.warnings,
                })?
            }
            Command::Load { service } => {
                let l = self.services.templates(&service)?.list()?;
                let environments =
                    EnvironmentStore::for_service(&self.services, &service)?.read()?;
                to_value(Library {
                    templates: l.templates,
                    warnings: l.warnings,
                    environments,
                })?
            }
            Command::Create { input } => to_value(self.services.create(&input)?)?,
            Command::Save {
                service,
                input,
                id,
                revision,
            } => to_value(self.services.templates(&service)?.save(
                &input,
                id.as_deref(),
                revision,
            )?)?,
            Command::SaveEnvironments { service, data } => {
                to_value(EnvironmentStore::for_service(&self.services, &service)?.write(&data)?)?
            }
            Command::Resolve {
                service,
                input,
                environment,
            } => to_value(self.resolve(&service, &input, environment.as_deref())?)?,
            Command::Start {
                service,
                input,
                environment,
            } => {
                if self.running.len() >= 8 {
                    return Err(Error::msg("最多同时发送 8 个请求"));
                }
                let request = self.resolve(&service, &input, environment.as_deref())?;
                let operation = self.engine.start(request)?;
                self.next += 1;
                self.running.insert(self.next, operation);
                to_value(self.next)?
            }
            Command::Cancel { operation, discard } => {
                if let Some((op, _)) = self.running.get_mut(&operation) {
                    op.cancel();
                }
                if discard {
                    self.running.remove(&operation);
                }
                Value::Null
            }
            Command::Poll { operations } => {
                let mut batches: Events = Vec::new();
                let mut completed = Vec::new();
                for (id, (_, receiver)) in &self.running {
                    if !operations.contains(id) {
                        continue;
                    }
                    // Bound each turn so streaming requests cannot monopolize the UI bridge.
                    let events: Vec<_> = receiver.try_iter().take(32).collect();
                    if events.iter().any(|e| matches!(e, Event::End(_))) {
                        completed.push(*id);
                    }
                    if !events.is_empty() {
                        batches.push((*id, events));
                    }
                }
                for id in completed {
                    self.running.remove(&id);
                }
                to_value(batches)?
            }
        })
    }

    fn resolve(
        &self,
        service: &str,
        input: &crate::TemplateInput,
        environment: Option<&str>,
    ) -> Result<crate::ResolvedRequest> {
        let data = EnvironmentStore::for_service(&self.services, service)?.read()?;
        let profile = select_environment(&data, environment)?;
        resolve_request(&apply_environment(&input.request, profile)?, true)
    }
}
