use crate::backend::Backend;
#[cfg(feature = "cli-integration")]
use apibuddy_core::ServiceStore;
#[cfg(feature = "cli-integration")]
use apibuddy_core::cli;
use apibuddy_core::{
    Environments, Event, Outcome, SavedTemplate, Service, ServiceInput, TemplateInput, curl,
    default_service, empty_environments, new_template,
    protocol::{Catalog, Command, DraftTab, Events, HistoryEntry, Library, Workspace},
};
use nsh_core::{prelude::*, time::Instant};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[derive(Clone, Copy)]
pub struct RequestTab {
    pub id: u64,
    pub service: Signal<Service>,
    pub draft: Signal<TemplateInput>,
    pub baseline: Signal<TemplateInput>,
    pub saved: Signal<Option<(String, u64)>>,
    pub environment: Signal<Option<String>>,
    pub response: Signal<Response>,
    pub response_view: crate::response::ResponseView,
    pub section: Signal<usize>,
    pub content_version: Signal<u64>,
    pub body_wrap: Signal<bool>,
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
    Save {
        id: u64,
        copy: bool,
        close: bool,
    },
}

#[derive(Clone, Copy)]
pub struct Workbench {
    backend: Stored<Backend>,
    #[cfg(feature = "cli-integration")]
    pub services: Stored<ServiceStore>,
    pub catalog: Signal<Vec<Service>>,
    pub selected_service: Signal<String>,
    pub library: Signal<Vec<SavedTemplate>>,
    pub library_loading: Signal<bool>,
    pub environments: Signal<Environments>,
    pub tabs: Signal<Vec<RequestTab>>,
    pub active: Signal<u64>,
    pub history: Signal<Vec<HistoryEntry>>,
    pub notice: Signal<String>,
    pub dialog: Signal<Dialog>,
    #[cfg(feature = "cli-integration")]
    pub agent_text: Signal<String>,
    pub recovery_status: Signal<String>,
    recovery_ready: Signal<bool>,
    workspace_pending: Stored<Option<Workspace>>,
    workspace_last: Stored<Workspace>,
    workspace_busy: Stored<bool>,
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
            library_loading: signal(false),
            environments: signal(empty_environments()),
            tabs: signal(Vec::new()),
            active: signal(0),
            history: signal(Vec::new()),
            notice: signal("就绪".to_string()),
            dialog: signal(Dialog::None),
            #[cfg(feature = "cli-integration")]
            agent_text: signal(String::new()),
            recovery_status: signal("正在恢复草稿…".into()),
            recovery_ready: signal(false),
            workspace_pending: stored(None),
            workspace_last: stored(Workspace::default()),
            workspace_busy: stored(false),
            next_id: stored(0),
            polling: stored(false),
            saving: stored(HashSet::new()),
            mutation: stored(false),
            load_generation: stored(0),
            running: stored(HashMap::new()),
            owner: Owner::current(),
        };
        this.reload();
        this.restore();
        effect(move || {
            if this.recovery_ready.get() {
                let data = this.snapshot();
                let changed = this.workspace_last.with(|last| *last != data);
                if changed {
                    this.workspace_pending.with(|p| *p = Some(data));
                    this.recovery_status.set("草稿待保存…".into());
                }
            }
        });
        set_interval(std::time::Duration::from_millis(500), move || {
            this.persist_workspace()
        });
        set_interval(std::time::Duration::from_millis(50), move || this.poll());
        cleanup(move || this.running.with(|running| running.clear()));
        this
    }

    fn client() -> String {
        "native".into()
    }

    fn snapshot(self) -> Workspace {
        let tabs = self.tabs.get();
        Workspace {
            schema_version: 1,
            revision: self.workspace_last.with(|w| w.revision),
            active: tabs
                .iter()
                .position(|t| t.id == self.active.get())
                .unwrap_or(0),
            selected_service: self.selected_service.get(),
            tabs: tabs
                .iter()
                .map(|t| DraftTab {
                    service: t.service.get(),
                    input: t.draft.get(),
                    baseline: t.baseline.get(),
                    saved: t.saved.get(),
                    environment: t.environment.get(),
                    section: t.section.get(),
                })
                .collect(),
        }
    }

    fn restore(self) {
        self.task(async move {
            match self
                .backend()
                .call::<Vec<HistoryEntry>>(Command::History)
                .await
            {
                Ok(entries) => self.history.set(entries),
                Err(e) => self.notice.set(format!("历史读取失败：{e}")),
            }
            let loaded = self
                .backend()
                .call::<Workspace>(Command::Workspace {
                    client: Self::client(),
                })
                .await;
            let can_save = loaded.is_ok();
            match loaded {
                Ok(data) => {
                    for draft in &data.tabs {
                        self.open(
                            draft.service.clone(),
                            draft.input.clone(),
                            draft.saved.clone(),
                            draft.environment.clone(),
                        );
                        if let Some(tab) = self.current() {
                            tab.baseline.set(draft.baseline.clone());
                            tab.section.set(draft.section.min(5));
                        }
                    }
                    if let Some(tab) = self
                        .tabs
                        .with_untracked(|tabs| tabs.get(data.active).copied())
                    {
                        self.activate(tab);
                    }
                    self.select_service(data.selected_service.clone());
                    self.workspace_last.with(|last| *last = data);
                    self.recovery_status.set("草稿已恢复".into());
                }
                Err(e) => self
                    .recovery_status
                    .set(format!("草稿恢复失败，原文件保留：{e}")),
            }
            if self.tabs.with_untracked(Vec::is_empty) {
                self.new_request();
            }
            self.recovery_ready.set(can_save);
        });
    }

    fn persist_workspace(self) {
        if self.workspace_busy.with(|b| *b) || !self.recovery_ready.get_untracked() {
            return;
        }
        let Some(mut data) = self.workspace_pending.with(Option::take) else {
            return;
        };
        data.revision = self.workspace_last.with(|w| w.revision);
        self.workspace_busy.with(|b| *b = true);
        self.task(async move {
            let result = self
                .backend()
                .call::<Workspace>(Command::SaveWorkspace {
                    client: Self::client(),
                    data,
                })
                .await;
            self.workspace_busy.with(|b| *b = false);
            match result {
                Ok(data) => {
                    self.workspace_last.with(|last| *last = data);
                    let current = untracked(|| self.snapshot());
                    if self.workspace_last.with(|last| *last != current) {
                        self.workspace_pending.with(|p| *p = Some(current));
                        self.recovery_status.set("草稿待保存…".into());
                    } else {
                        self.workspace_pending.with(|p| *p = None);
                        self.recovery_status.set("草稿已保存在本机".into());
                    }
                }
                Err(e) => {
                    self.recovery_ready.set(false);
                    self.recovery_status.set(format!("草稿自动保存已停止：{e}"));
                }
            }
        });
    }

    fn record_history(self, entry: HistoryEntry) {
        self.history.update(|h| {
            h.insert(0, entry.clone());
            h.truncate(50);
        });
        self.task(async move {
            match self
                .backend()
                .call::<Vec<HistoryEntry>>(Command::AppendHistory { entry })
                .await
            {
                Ok(entries) => self.history.set(entries),
                Err(e) => self.notice.set(format!("历史保存失败：{e}")),
            }
        });
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
        self.library_loading.set(true);
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
            self.library_loading.set(false);
            match result {
                Ok(library) => {
                    if !library.warnings.is_empty() {
                        self.notice.set(library.warnings.join("；"));
                    }
                    self.library.set(library.templates);
                    if self.current().is_none_or(|t| {
                        t.service.get_untracked().id == self.selected_service.get_untracked()
                    }) {
                        self.environments.set(library.environments);
                    }
                }
                Err(e) => {
                    self.library.set(Vec::new());
                    if self.current().is_none_or(|t| {
                        t.service.get_untracked().id == self.selected_service.get_untracked()
                    }) {
                        self.environments.set(empty_environments());
                    }
                    self.notice.set(e.to_string());
                }
            }
        });
    }

    pub fn select_service(self, id: String) {
        // Never expose the previous collection's rows under the new collection ID.
        self.library.set(Vec::new());
        self.selected_service.set(id);
        self.reload_service();
    }

    pub fn activate(self, tab: RequestTab) {
        let service = tab.service.with(|s| s.id.clone());
        if self.selected_service.get_untracked() != service {
            self.select_service(service);
        }
        self.active.set(tab.id);
        self.reload_environments(tab);
    }

    fn reload_environments(self, tab: RequestTab) {
        let service = tab.service.get_untracked().id;
        self.environments.set(empty_environments());
        self.task(async move {
            match self
                .backend()
                .call::<Library>(Command::Load {
                    service: service.clone(),
                })
                .await
            {
                Ok(library)
                    if self.current().is_some_and(|t| {
                        t.id == tab.id && t.service.get_untracked().id == service
                    }) =>
                {
                    self.environments.set(library.environments)
                }
                Err(e) => self.notice.set(e.to_string()),
                _ => {}
            }
        });
    }

    pub fn open(
        self,
        service: Service,
        input: TemplateInput,
        saved: Option<(String, u64)>,
        environment: Option<String>,
    ) -> Option<RequestTab> {
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
                return Some(tab);
            }
        }
        if self.tabs.with_untracked(Vec::len) >= 20 {
            self.notice
                .set("最多打开 20 个请求，请先关闭一个标签".into());
            return None;
        }
        // Signals belong to the workbench owner, even when invoked from a temporary dialog.
        let tab = self.owner.run(|| RequestTab {
            id: self.id(),
            service: signal(service),
            draft: signal(input.clone()),
            baseline: signal(input),
            saved: signal(saved),
            environment: signal(environment),
            response: signal(Response::default()),
            response_view: crate::response::ResponseView::new(),
            section: signal(0),
            content_version: signal(0),
            body_wrap: signal(true),
        });
        self.tabs.update(|tabs| tabs.push(tab));
        self.activate(tab);
        Some(tab)
    }

    pub fn duplicate(self, source: RequestTab) {
        let mut input = source.draft.get_untracked();
        let original_name = input.name.clone();
        for number in 1.. {
            let suffix = if number == 1 {
                " 副本".into()
            } else {
                format!(" 副本 {number}")
            };
            let mut end = original_name.len().min(120 - suffix.len());
            while !original_name.is_char_boundary(end) {
                end -= 1;
            }
            input.name = format!("{}{suffix}", &original_name[..end]);
            if !self.tabs.with_untracked(|tabs| {
                tabs.iter()
                    .any(|t| t.draft.with_untracked(|d| d.name == input.name))
            }) {
                break;
            }
        }
        if let Some(tab) = self.open(
            source.service.get_untracked(),
            input,
            None,
            source.environment.get_untracked(),
        ) {
            tab.section.set(source.section.get_untracked());
            tab.body_wrap.set(source.body_wrap.get_untracked());
            self.notice.set("已复制为独立草稿，保存时创建新接口".into());
        }
    }

    pub fn switch_tab(self, direction: isize) {
        let selected = self.tabs.with_untracked(|tabs| {
            let at = tabs
                .iter()
                .position(|t| t.id == self.active.get_untracked())?;
            tabs.get((at as isize + direction).rem_euclid(tabs.len() as isize) as usize)
                .copied()
        });
        if let Some(tab) = selected {
            self.activate(tab);
        }
    }

    pub fn select_tab(self, index: usize) {
        let selected = self.tabs.with_untracked(|tabs| {
            if index == 8 {
                tabs.last().copied()
            } else {
                tabs.get(index).copied()
            }
        });
        if let Some(tab) = selected {
            self.activate(tab);
        }
    }

    pub fn new_request(self) {
        let service = self.service();
        let mut input = new_template();
        input.name = "未命名请求".into();
        input.request.url.clear();
        input.request.variables.clear();
        if !service.base_url.is_empty() && service.id != "default" {
            input.request.variables.push(apibuddy_core::Variable {
                key: "base_url".into(),
                value: service.base_url.clone(),
            });
        }
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
        if copy || tab.saved.get_untracked().is_none() {
            self.dialog.set(Dialog::Save {
                id: tab.id,
                copy,
                close: false,
            });
        } else {
            self.save_to(
                tab,
                tab.service.get_untracked(),
                tab.draft.get_untracked().name,
                false,
                false,
            );
        }
    }

    pub fn save_and_close(self, tab: RequestTab) {
        if tab.saved.get_untracked().is_none() {
            self.dialog.set(Dialog::Save {
                id: tab.id,
                copy: false,
                close: true,
            });
        } else {
            self.save_to(
                tab,
                tab.service.get_untracked(),
                tab.draft.get_untracked().name,
                false,
                true,
            );
        }
    }

    pub fn save_to(
        self,
        tab: RequestTab,
        destination: Service,
        name: String,
        copy: bool,
        close: bool,
    ) {
        if !self.saving.with(|ids| ids.insert(tab.id)) {
            return;
        }
        let saved = if copy || destination.id != tab.service.get_untracked().id {
            None
        } else {
            tab.saved.get_untracked()
        };
        let service = destination.id.clone();
        tab.draft.update(|d| d.name = name);
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
            if !self.tabs.with_untracked(|tabs| tabs.iter().any(|t| t.id == tab.id)) {
                if service == self.selected_service.get_untracked() { self.reload_service(); }
                if let Err(error) = result { self.notice.set(format!("已关闭请求的保存失败：{error}")); }
                return;
            }
            match result {
                Ok(saved) => {
                    // An asynchronous save must not overwrite edits made while it was pending.
                    tab.baseline.set(TemplateInput {
                        name: saved.name,
                        description: saved.description,
                        request: saved.request,
                    });
                    tab.saved.set(Some((saved.id, saved.revision)));
                    if tab.service.get_untracked().id != destination.id { tab.environment.set(None); }
                    tab.service.set(destination);
                    if self.active.get_untracked() == tab.id { self.activate(tab); }
                    if matches!(self.dialog.get_untracked(), Dialog::Save { id, .. } if id == tab.id) { self.dialog.set(Dialog::None); }
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
        let index = self
            .tabs
            .with_untracked(|tabs| tabs.iter().position(|t| t.id == id));
        let closed = self
            .tabs
            .with_untracked(|tabs| tabs.iter().find(|t| t.id == id).copied());
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
            let neighbor = self.tabs.with_untracked(|tabs| {
                tabs.get(index.unwrap_or(0).min(tabs.len().saturating_sub(1)))
                    .copied()
            });
            if let Some(neighbor) = neighbor {
                self.activate(neighbor);
            } else {
                self.new_request();
            }
        }
        self.dialog.set(Dialog::None);
        // Signals belong to the workbench lifetime. Release large buffers once their view
        // is gone, so repeatedly closing 8 MiB responses cannot retain them until quit.
        if let Some(tab) = closed {
            tab.response.set(Response::default());
            tab.draft.set(new_template());
            tab.baseline.set(new_template());
            tab.response_view.clear_inputs();
        }
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
            id: uuid::Uuid::new_v4().to_string(),
            completed_at: 0,
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
        tab.response_view.selected.set(None);
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
                    if self.running.with(|r| r.contains_key(&tab.id)) {
                        self.notice.set("请求已发送".into());
                    }
                }
                Err(e) => {
                    let failed = self.running.with(|r| r.remove(&tab.id));
                    let Some(mut failed) = failed else {
                        return;
                    };
                    failed.history.summary = e.to_string();
                    self.record_history(failed.history);
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
            let result = self.backend().poll(operations).await;
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
                    self.record_history(request.history);
                }
            }
        });
    }

    pub fn create_service(self, mut input: ServiceInput) {
        if input.slug.trim().is_empty() {
            input.slug = format!("collection-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        }
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
                    if self.dialog.get_untracked() == Dialog::Service {
                        self.dialog.set(Dialog::None);
                    }
                    self.notice.set("集合已创建".into());
                }
                Err(e) => self.notice.set(e.to_string()),
            }
        });
    }

    pub fn save_new_collection(
        self,
        tab: RequestTab,
        collection: String,
        name: String,
        close: bool,
        selection: Signal<String>,
    ) {
        let mut input = tab.draft.get_untracked();
        input.name = name.clone();
        if let Err(e) = apibuddy_core::templates::validate_template(&input) {
            self.notice.set(e.to_string());
            return;
        }
        if self.mutation.with(|v| std::mem::replace(v, true)) {
            return;
        }
        self.task(async move {
            let result = self.backend().call::<Service>(Command::Create { input: ServiceInput {
                name: collection, slug: format!("collection-{}", &uuid::Uuid::new_v4().to_string()[..8]), base_url: String::new(), description: String::new(),
            }}).await;
            self.mutation.with(|v| *v = false);
            match result {
                Ok(service) => {
                    self.catalog.update(|c| c.push(service.clone()));
                    // Retain the new destination if saving the request needs correction/retry.
                    if matches!(self.dialog.get_untracked(), Dialog::Save { id, .. } if id == tab.id) { selection.set(service.id.clone()); }
                    self.save_to(tab, service, name, true, close);
                }
                Err(e) => self.notice.set(e.to_string()),
            }
        });
    }

    pub fn save_environments(self, data: Environments) {
        if self.mutation.with(|v| std::mem::replace(v, true)) {
            return;
        }
        let service = self
            .current()
            .map(|t| t.service.get_untracked().id)
            .unwrap_or_else(|| self.selected_service.get_untracked());
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
                    if self
                        .current()
                        .is_some_and(|t| t.service.get_untracked().id == service)
                    {
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
        self.notice.set("正在解析当前请求…".into());
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
