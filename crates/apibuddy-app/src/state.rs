use crate::backend::Backend;
#[cfg(feature = "cli-integration")]
use apibuddy_core::ServiceStore;
#[cfg(feature = "cli-integration")]
use apibuddy_core::cli;
use apibuddy_core::{
    Environments, Event, Outcome, SavedTemplate, Service, ServiceInput, TemplateInput, curl,
    default_service, empty_environments, new_template,
    protocol::{Catalog, Command, Events, Library},
};
use nsh_core::{prelude::*, time::Instant};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[derive(Clone, Copy)]
pub struct RequestTab {
    pub id: u64,
    pub service: Stored<Service>,
    pub draft: Signal<TemplateInput>,
    pub baseline: Signal<TemplateInput>,
    pub saved: Signal<Option<(String, u64)>>,
    pub environment: Signal<Option<String>>,
    pub response: Signal<Response>,
    pub section: Signal<usize>,
    pub content_version: Signal<u64>,
}

impl PartialEq for RequestTab {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct Response {
    pub status: Option<u16>,
    pub headers: String,
    pub body: String,
    pub bytes: usize,
    pub elapsed: u128,
    pub running: bool,
    pub message: String,
}

#[derive(Clone, PartialEq)]
pub struct HistoryEntry {
    pub id: u64,
    pub service: Service,
    pub input: TemplateInput,
    pub environment: Option<String>,
    pub summary: String,
}

struct Running {
    operation: Option<u64>,
    cancelled: bool,
    started: Instant,
    tab: RequestTab,
    history: HistoryEntry,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Dialog {
    #[default]
    None,
    Service,
    Environment,
    Import,
    #[cfg(feature = "cli-integration")]
    Agent,
    Close(u64),
}

#[derive(Clone, Copy)]
pub struct Workbench {
    backend: Stored<Backend>,
    #[cfg(feature = "cli-integration")]
    pub services: Stored<ServiceStore>,
    pub catalog: Signal<Vec<Service>>,
    pub selected_service: Signal<String>,
    pub library: Signal<Vec<SavedTemplate>>,
    pub environments: Signal<Environments>,
    pub tabs: Signal<Vec<RequestTab>>,
    pub active: Signal<u64>,
    pub history: Signal<Vec<HistoryEntry>>,
    pub notice: Signal<String>,
    pub dialog: Signal<Dialog>,
    #[cfg(feature = "cli-integration")]
    pub agent_text: Signal<String>,
    next_id: Stored<u64>,
    polling: Stored<bool>,
    saving: Stored<HashSet<u64>>,
    mutation: Stored<bool>,
    load_generation: Stored<u64>,
    running: Stored<HashMap<u64, Running>>,
    owner: Owner,
}

impl Workbench {
    pub fn new(root: PathBuf) -> Self {
        let this = Self {
            backend: stored(Backend::new(root.clone())),
            #[cfg(feature = "cli-integration")]
            services: stored(ServiceStore::with_root(root)),
            catalog: signal(vec![default_service()]),
            selected_service: signal("default".to_string()),
            library: signal(Vec::new()),
            environments: signal(empty_environments()),
            tabs: signal(Vec::new()),
            active: signal(0),
            history: signal(Vec::new()),
            notice: signal("就绪".to_string()),
            dialog: signal(Dialog::None),
            #[cfg(feature = "cli-integration")]
            agent_text: signal(String::new()),
            next_id: stored(0),
            polling: stored(false),
            saving: stored(HashSet::new()),
            mutation: stored(false),
            load_generation: stored(0),
            running: stored(HashMap::new()),
            owner: Owner::current(),
        };
        this.reload();
        this.new_request();
        set_interval(std::time::Duration::from_millis(50), move || this.poll());
        cleanup(move || this.running.with(|running| running.clear()));
        this
    }

    fn id(self) -> u64 {
        self.next_id.with(|n| {
            *n += 1;
            *n
        })
    }

    pub fn current(self) -> Option<RequestTab> {
        let id = self.active.get();
        self.tabs
            .with(|tabs| tabs.iter().find(|t| t.id == id).copied())
    }

    pub fn service(self) -> Service {
        self.catalog
            .with(|items| {
                items
                    .iter()
                    .find(|s| s.id == self.selected_service.get())
                    .cloned()
            })
            .unwrap_or_else(default_service)
    }

    fn task(self, future: impl std::future::Future<Output = ()> + 'static) {
        self.owner.run(|| {
            spawn(future);
        });
    }

    fn backend(self) -> Backend {
        self.backend.with(|b| b.clone())
    }

    pub fn reload(self) {
        self.task(async move {
            match self.backend().call::<Catalog>(Command::Catalog).await {
                Ok(catalog) => {
                    if !catalog.warnings.is_empty() {
                        self.notice.set(catalog.warnings.join("；"));
                    }
                    self.catalog.set(catalog.services);
                    self.reload_service();
                }
                Err(e) => self.notice.set(e.to_string()),
            }
        });
    }

    pub fn reload_service(self) {
        let service = self.selected_service.get_untracked();
        let generation = self.load_generation.with(|v| {
            *v += 1;
            *v
        });
        self.task(async move {
            let result = self
                .backend()
                .call::<Library>(Command::Load { service })
                .await;
            if self.load_generation.with(|v| *v) != generation {
                return;
            }
            match result {
                Ok(library) => {
                    if !library.warnings.is_empty() {
                        self.notice.set(library.warnings.join("；"));
                    }
                    self.library.set(library.templates);
                    self.environments.set(library.environments);
                }
                Err(e) => {
                    self.library.set(Vec::new());
                    self.environments.set(empty_environments());
                    self.notice.set(e.to_string());
                }
            }
        });
    }

    pub fn select_service(self, id: String) {
        self.selected_service.set(id);
        self.reload_service();
    }

    pub fn activate(self, tab: RequestTab) {
        let service = tab.service.with(|s| s.id.clone());
        if self.selected_service.get_untracked() != service {
            self.select_service(service);
        }
        self.active.set(tab.id);
    }

    pub fn open(
        self,
        service: Service,
        input: TemplateInput,
        saved: Option<(String, u64)>,
        environment: Option<String>,
    ) {
        if let Some((ref template_id, _)) = saved {
            let existing = self.tabs.with_untracked(|tabs| {
                tabs.iter()
                    .find(|t| {
                        t.service.with(|s| s.id == service.id)
                            && t.saved.with_untracked(|s| {
                                s.as_ref().is_some_and(|(id, _)| id == template_id)
                            })
                    })
                    .copied()
            });
            if let Some(tab) = existing {
                self.activate(tab);
                return;
            }
        }
        if self.tabs.with_untracked(Vec::len) >= 20 {
            self.notice
                .set("最多打开 20 个请求，请先关闭一个标签".into());
            return;
        }
        // Signals belong to the workbench owner, even when invoked from a temporary dialog.
        let tab = self.owner.run(|| RequestTab {
            id: self.id(),
            service: stored(service),
            draft: signal(input.clone()),
            baseline: signal(input),
            saved: signal(saved),
            environment: signal(environment),
            response: signal(Response::default()),
            section: signal(0),
            content_version: signal(0),
        });
        self.tabs.update(|tabs| tabs.push(tab));
        self.activate(tab);
    }

    pub fn new_request(self) {
        let service = self.service();
        let mut input = new_template();
        input.name = "未命名请求".into();
        input.request.url = "{{base_url}}/get".into();
        input.request.variables[0].value = service.base_url.clone();
        self.open(service, input, None, None);
    }

    pub fn open_saved(self, saved: SavedTemplate) {
        self.open(
            self.service(),
            TemplateInput {
                name: saved.name,
                description: saved.description,
                request: saved.request,
            },
            Some((saved.id, saved.revision)),
            None,
        );
    }

    pub fn save(self, tab: RequestTab, copy: bool) {
        self.save_then(tab, copy, false);
    }

    pub fn save_and_close(self, tab: RequestTab) {
        self.save_then(tab, false, true);
    }

    fn save_then(self, tab: RequestTab, copy: bool, close: bool) {
        if !self.saving.with(|ids| ids.insert(tab.id)) {
            return;
        }
        let saved = if copy {
            None
        } else {
            tab.saved.get_untracked()
        };
        let service = tab.service.with(|s| s.id.clone());
        let input = tab.draft.get_untracked();
        self.task(async move {
            let result = self
                .backend()
                .call::<SavedTemplate>(Command::Save {
                    service: service.clone(),
                    input: input.clone(),
                    id: saved.as_ref().map(|(id, _)| id.clone()),
                    revision: saved.map(|(_, rev)| rev),
                })
                .await;
            self.saving.with(|ids| {
                ids.remove(&tab.id);
            });
            match result {
                Ok(saved) => {
                    // An asynchronous save must not overwrite edits made while it was pending.
                    tab.baseline.set(TemplateInput {
                        name: saved.name,
                        description: saved.description,
                        request: saved.request,
                    });
                    tab.saved.set(Some((saved.id, saved.revision)));
                    self.notice.set(format!("已保存 · 修订 {}", saved.revision));
                    if service == self.selected_service.get_untracked() {
                        self.reload_service();
                    }
                    if close && !Self::dirty(tab) {
                        self.close(tab.id);
                    }
                }
                Err(e) => self.notice.set(e.to_string()),
            }
        });
    }

    pub fn dirty(tab: RequestTab) -> bool {
        tab.saved.with(Option::is_none) || tab.draft.get() != tab.baseline.get()
    }

    pub fn request_close(self, tab: RequestTab) {
        if Self::dirty(tab) {
            self.dialog.set(Dialog::Close(tab.id));
        } else {
            self.close(tab.id);
        }
    }

    pub fn close(self, id: u64) {
        let operation = self
            .running
            .with(|running| running.remove(&id).and_then(|r| r.operation));
        if let Some(operation) = operation {
            self.task(async move {
                let _ = self
                    .backend()
                    .call::<()>(Command::Cancel {
                        operation,
                        discard: true,
                    })
                    .await;
            });
        }
        self.tabs.update(|tabs| tabs.retain(|tab| tab.id != id));
        if self.active.get_untracked() == id {
            let last = self.tabs.with_untracked(|tabs| tabs.last().copied());
            if let Some(last) = last {
                self.activate(last);
            } else {
                self.new_request();
            }
        }
        self.dialog.set(Dialog::None);
    }

    pub fn send(self, tab: RequestTab) {
        if tab.response.with_untracked(|r| r.running) {
            self.cancel(tab);
            return;
        }
        if self.running.with(|r| r.len()) >= 8 {
            self.notice.set("最多同时发送 8 个请求".into());
            return;
        }
        let input = tab.draft.get_untracked();
        let environment = tab.environment.get_untracked();
        let service = tab.service.with(|s| s.clone());
        let history = HistoryEntry {
            id: self.id(),
            service: service.clone(),
            input: input.clone(),
            environment: environment.clone(),
            summary: String::new(),
        };
        tab.response.set(Response {
            running: true,
            message: "正在发送…".into(),
            ..Default::default()
        });
        tab.content_version.update(|v| *v += 1);
        self.running.with(|r| {
            r.insert(
                tab.id,
                Running {
                    operation: None,
                    cancelled: false,
                    started: Instant::now(),
                    tab,
                    history,
                },
            );
        });
        self.task(async move {
            match self
                .backend()
                .call::<u64>(Command::Start {
                    service: service.id,
                    input,
                    environment,
                })
                .await
            {
                Ok(operation) => {
                    let cancel = self.running.with(|running| match running.get_mut(&tab.id) {
                        Some(r) => {
                            r.operation = Some(operation);
                            r.cancelled
                        }
                        None => true,
                    });
                    if cancel {
                        let discard = self.running.with(|r| !r.contains_key(&tab.id));
                        let _ = self
                            .backend()
                            .call::<()>(Command::Cancel { operation, discard })
                            .await;
                    }
                    self.notice.set("请求已发送".into());
                }
                Err(e) => {
                    self.running.with(|r| {
                        r.remove(&tab.id);
                    });
                    tab.response.update(|r| {
                        r.running = false;
                        r.message = e.to_string();
                    });
                    self.notice.set(e.to_string());
                }
            }
        });
    }

    pub fn cancel(self, tab: RequestTab) {
        let operation = self.running.with(|running| {
            running.get_mut(&tab.id).and_then(|r| {
                r.cancelled = true;
                r.operation
            })
        });
        if let Some(operation) = operation {
            self.task(async move {
                if let Err(e) = self
                    .backend()
                    .call::<()>(Command::Cancel {
                        operation,
                        discard: false,
                    })
                    .await
                {
                    self.notice.set(e.to_string());
                }
            });
        }
    }

    fn poll(self) {
        if self.running.with(|r| r.is_empty()) || self.polling.with(|p| *p) {
            return;
        }
        self.polling.with(|p| *p = true);
        self.task(async move {
            let operations = self
                .running
                .with(|r| r.values().filter_map(|r| r.operation).collect());
            let result = self
                .backend()
                .call::<Events>(Command::Poll { operations })
                .await;
            self.polling.with(|p| *p = false);
            match result {
                Ok(events) => self.apply_events(events),
                Err(e) => {
                    self.notice.set(e.to_string());
                    self.running.with(|r| {
                        for request in r.values() {
                            request.tab.response.update(|s| {
                                s.running = false;
                                s.message = e.to_string();
                            });
                        }
                        r.clear();
                    });
                }
            }
        });
    }

    fn apply_events(self, batches: Events) {
        let mut events: HashMap<u64, Vec<Event>> = batches.into_iter().collect();
        self.running.with(|running| {
            let mut completed = Vec::new();
            for (id, request) in running.iter_mut() {
                let batch = request
                    .operation
                    .and_then(|op| events.remove(&op))
                    .unwrap_or_default();
                let changed = !batch.is_empty();
                request.tab.response.update(|response| {
                    response.elapsed = request.started.elapsed().as_millis();
                    for event in batch {
                        match event {
                            Event::Headers {
                                status,
                                status_text,
                                headers,
                                ..
                            } => {
                                response.status = Some(status);
                                response.headers = headers;
                                response.message = format!("{status} {status_text}");
                            }
                            Event::Text { text, bytes } => {
                                response.body.push_str(&text);
                                response.bytes = bytes;
                            }
                            Event::End(outcome) => {
                                response.running = false;
                                completed.push(*id);
                                match outcome {
                                    Outcome::Completed => {
                                        if response.body.is_empty() {
                                            response.message.push_str(" · 响应正文为空");
                                        }
                                    }
                                    Outcome::Cancelled => response.message = "已取消".into(),
                                    Outcome::TimedOut => response.message = "请求已超时".into(),
                                    Outcome::Failed(error) => response.message = error,
                                }
                                request.history.summary =
                                    format!("{} · {} ms", response.message, response.elapsed);
                            }
                        }
                    }
                });
                if changed {
                    request.tab.content_version.update(|v| *v += 1);
                }
            }
            for id in completed {
                if let Some(request) = running.remove(&id) {
                    self.history.update(|history| {
                        history.insert(0, request.history);
                        history.truncate(50);
                    });
                }
            }
        });
    }

    pub fn create_service(self, input: ServiceInput) {
        if self.mutation.with(|v| std::mem::replace(v, true)) {
            return;
        }
        self.task(async move {
            let result = self
                .backend()
                .call::<Service>(Command::Create { input })
                .await;
            self.mutation.with(|v| *v = false);
            match result {
                Ok(service) => {
                    self.catalog.update(|catalog| catalog.push(service.clone()));
                    self.select_service(service.id);
                    self.new_request();
                    if self.dialog.get_untracked() == Dialog::Service {
                        self.dialog.set(Dialog::None);
                    }
                    self.notice.set("服务已创建".into());
                }
                Err(e) => self.notice.set(e.to_string()),
            }
        });
    }

    pub fn save_environments(self, data: Environments) {
        if self.mutation.with(|v| std::mem::replace(v, true)) {
            return;
        }
        let service = self.selected_service.get_untracked();
        self.task(async move {
            let result = self
                .backend()
                .call::<Environments>(Command::SaveEnvironments {
                    service: service.clone(),
                    data,
                })
                .await;
            self.mutation.with(|v| *v = false);
            match result {
                Ok(data) => {
                    if service == self.selected_service.get_untracked() {
                        self.environments.set(data);
                    }
                    if self.dialog.get_untracked() == Dialog::Environment {
                        self.dialog.set(Dialog::None);
                    }
                    self.notice.set("环境已保存".into());
                }
                Err(e) => self.notice.set(e.to_string()),
            }
        });
    }

    pub fn copy_curl(self, tab: RequestTab) {
        let service = tab.service.with(|s| s.id.clone());
        let input = tab.draft.get_untracked();
        let environment = tab.environment.get_untracked();
        self.task(async move {
            match self
                .backend()
                .call::<apibuddy_core::ResolvedRequest>(Command::Resolve {
                    service,
                    input,
                    environment,
                })
                .await
            {
                Ok(request) => {
                    crate::backend::copy_text(curl::export(&request));
                    self.notice
                        .set("已复制当前环境下的 cURL（含请求头与实际变量值）".into());
                }
                Err(e) => self.notice.set(e.to_string()),
            }
        });
    }

    #[cfg(feature = "cli-integration")]
    pub fn install_cli(self) {
        let id = self.selected_service.get_untracked();
        let environment_id = self
            .current()
            .filter(|tab| tab.service.with(|s| s.id == id))
            .and_then(|tab| tab.environment.get_untracked());
        match self.services.with(|services| {
            cli::install_cli_with_environment(services, &id, environment_id.as_deref())
        }) {
            Ok(bundle) => {
                self.agent_text.set(bundle.markdown);
                self.dialog.set(Dialog::Agent);
                self.notice.set("CLI 已生成".into());
            }
            Err(error) => self.notice.set(error.to_string()),
        }
    }
}
