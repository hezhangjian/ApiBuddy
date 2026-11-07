#[cfg(feature = "cli-integration")]
use apibuddy_core::cli;
use apibuddy_core::{
    Engine, EnvironmentStore, Environments, Event, Operation, Outcome, SavedTemplate, Service,
    ServiceInput, ServiceStore, TemplateInput, apply_environment, curl, default_service,
    empty_environments, new_template, resolve_request, select_environment,
};
use nsh_core::prelude::*;
use std::{collections::HashMap, path::PathBuf, sync::mpsc::Receiver, time::Instant};

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
    operation: Operation,
    receiver: Receiver<Event>,
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
    engine: Stored<Engine>,
    running: Stored<HashMap<u64, Running>>,
    owner: Owner,
}

impl Workbench {
    pub fn new(root: PathBuf) -> Self {
        let this = Self {
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
            engine: stored(Engine::new()),
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

    pub fn reload(self) {
        match self.services.with(|services| services.list()) {
            Ok(catalog) => {
                if !catalog.warnings.is_empty() {
                    self.notice.set(catalog.warnings.join("；"));
                }
                self.catalog.set(catalog.services);
            }
            Err(error) => self.notice.set(error.to_string()),
        }
        self.reload_service();
    }

    pub fn reload_service(self) {
        let id = self.selected_service.get_untracked();
        match self
            .services
            .with(|services| services.templates(&id)?.list())
        {
            Ok(library) => {
                if !library.warnings.is_empty() {
                    self.notice.set(library.warnings.join("；"));
                }
                self.library.set(library.templates);
            }
            Err(error) => {
                self.library.set(Vec::new());
                self.notice.set(error.to_string());
            }
        }
        match self
            .services
            .with(|services| EnvironmentStore::for_service(services, &id)?.read())
        {
            Ok(data) => self.environments.set(data),
            Err(error) => {
                self.environments.set(empty_environments());
                self.notice.set(error.to_string());
            }
        }
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

    pub fn save(self, tab: RequestTab, copy: bool) -> bool {
        let saved = if copy {
            None
        } else {
            tab.saved.get_untracked()
        };
        let service_id = tab.service.with(|s| s.id.clone());
        let result = self.services.with(|services| {
            services.templates(&service_id)?.save(
                &tab.draft.get_untracked(),
                saved.as_ref().map(|(id, _)| id.as_str()),
                saved.as_ref().map(|(_, revision)| *revision),
            )
        });
        match result {
            Ok(saved) => {
                let input = TemplateInput {
                    name: saved.name,
                    description: saved.description,
                    request: saved.request,
                };
                tab.draft.set(input.clone());
                tab.baseline.set(input);
                tab.saved.set(Some((saved.id, saved.revision)));
                self.notice.set(format!("已保存 · 修订 {}", saved.revision));
                if service_id == self.selected_service.get_untracked() {
                    self.reload_service();
                }
                true
            }
            Err(error) => {
                self.notice.set(error.to_string());
                false
            }
        }
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
        self.running.with(|running| {
            running.remove(&id);
        });
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

    pub fn resolved(
        self,
        tab: RequestTab,
    ) -> apibuddy_core::Result<apibuddy_core::ResolvedRequest> {
        let service_id = tab.service.with(|s| s.id.clone());
        let data = self
            .services
            .with(|services| EnvironmentStore::for_service(services, &service_id)?.read())?;
        let id = tab.environment.get_untracked();
        let profile = select_environment(&data, id.as_deref())?;
        let request = apply_environment(&tab.draft.get_untracked().request, profile)?;
        resolve_request(&request, true)
    }

    pub fn send(self, tab: RequestTab) {
        if tab.response.with_untracked(|r| r.running) {
            self.cancel(tab);
            return;
        }
        if self.running.with(|running| running.len()) >= 8 {
            self.notice.set("最多同时发送 8 个请求".into());
            return;
        }
        let result = self
            .resolved(tab)
            .and_then(|request| self.engine.with(|engine| engine.start(request)));
        match result {
            Ok((operation, receiver)) => {
                tab.response.set(Response {
                    running: true,
                    message: "正在发送…".into(),
                    ..Default::default()
                });
                tab.content_version.update(|v| *v += 1);
                let history = HistoryEntry {
                    id: self.id(),
                    service: tab.service.with(|s| s.clone()),
                    input: tab.draft.get_untracked(),
                    environment: tab.environment.get_untracked(),
                    summary: String::new(),
                };
                self.running.with(|running| {
                    running.insert(
                        tab.id,
                        Running {
                            operation,
                            receiver,
                            started: Instant::now(),
                            tab,
                            history,
                        },
                    );
                });
                self.notice.set("请求已发送".into());
            }
            Err(error) => self.notice.set(error.to_string()),
        }
    }

    pub fn cancel(self, tab: RequestTab) {
        self.running.with(|running| {
            if let Some(request) = running.get_mut(&tab.id) {
                request.operation.cancel();
            }
        });
    }

    fn poll(self) {
        let mut completed = Vec::new();
        self.running.with(|running| {
            for (id, request) in running.iter_mut() {
                let events: Vec<_> = request.receiver.try_iter().collect();
                let changed = !events.is_empty();
                let mut end = false;
                request.tab.response.update(|response| {
                    response.elapsed = request.started.elapsed().as_millis();
                    for event in events {
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
                                end = true;
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
                if end {
                    completed.push(*id);
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

    pub fn create_service(self, input: ServiceInput) -> bool {
        match self.services.with(|services| services.create(&input)) {
            Ok(service) => {
                self.catalog.update(|catalog| catalog.push(service.clone()));
                self.select_service(service.id);
                self.new_request();
                self.notice.set("服务已创建".into());
                true
            }
            Err(error) => {
                self.notice.set(error.to_string());
                false
            }
        }
    }

    pub fn save_environments(self, data: Environments) -> bool {
        let id = self.selected_service.get_untracked();
        match self
            .services
            .with(|services| EnvironmentStore::for_service(services, &id)?.write(&data))
        {
            Ok(data) => {
                self.environments.set(data);
                self.notice.set("环境已保存".into());
                true
            }
            Err(error) => {
                self.notice.set(error.to_string());
                false
            }
        }
    }

    pub fn copy_curl(self, tab: RequestTab) {
        match self.resolved(tab) {
            Ok(request) => {
                set_clipboard(curl::export(&request));
                self.notice
                    .set("已复制当前环境下的 cURL（含请求头与实际变量值）".into());
            }
            Err(error) => self.notice.set(error.to_string()),
        }
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
