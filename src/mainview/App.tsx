import { useEffect, useRef, useState, type SetStateAction } from "react";
import { METHODS, referencedVariables, resolveRequest, type Library, type Method, type RequestDraft, type SavedTemplate, type TemplateInput } from "../shared/templates";
import { DEFAULT_SERVICE, type Service } from "../shared/services";
import { templateForService } from "../shared/service-templates";
import { applyEnvironment, emptyEnvironments, selectEnvironment } from "../shared/environments";
import { MAX_TABS, type EditorTab } from "../shared/workspace";
import { exportCurl } from "../shared/curl";
import { AgentPanel } from "./AgentPanel";
import { InterfaceDialog } from "./InterfaceDialog";
import { ServiceDialog } from "./ServiceDialog";
import { EnvironmentDialog } from "./EnvironmentDialog";
import { CurlDialog } from "./CurlDialog";
import { SplitPane, useLayout } from "./SplitPane";
import { PairEditor } from "./PairEditor";
import { ResponsePanel } from "./ResponsePanel";
import { errorText, rpc, unwrap } from "./rpc";
import { emptyResponse, useRequests } from "./use-request";
import { freshTab, useWorkspace } from "./use-workspace";
import { useClipboardShortcuts } from "./clipboard";

const inputOf = ({ name, description, request }: TemplateInput): TemplateInput => ({ name, description, request });
const isDirty = (tab: EditorTab) => JSON.stringify(tab.draft) !== JSON.stringify(tab.baseline);
export default function App() {
  const { workspace, setWorkspace, updateTab, ready, status: draftStatus, error: workspaceError } = useWorkspace();
  const current = workspace.tabs.find((item) => item.id === workspace.activeTabId)!;
  const currentService = useRef(current.serviceId);
  currentService.current = current.serviceId;
  const [services, setServices] = useState<Service[]>([DEFAULT_SERVICE]);
  const service = services.find((item) => item.id === current.serviceId) ?? { ...DEFAULT_SERVICE, id: current.serviceId, name: "服务不可用", baseUrl: "" };
  const draft = current.draft, tab = current.editorTab, dirty = isDirty(current);
  const saved = current.templateId ? { id: current.templateId, revision: current.revision, name: current.baseline.name } : undefined;
  const setDraft = (value: SetStateAction<TemplateInput>) => updateTab(current.id, (item) => ({ ...item, draft: typeof value === "function" ? value(item.draft) : value }));
  const setTab = (value: string) => updateTab(current.id, (item) => ({ ...item, editorTab: value }));
  const [creatingService, setCreatingService] = useState(false), [creatingInterface, setCreatingInterface] = useState(false);
  const [agentOpen, setAgentOpen] = useState(false), [environmentOpen, setEnvironmentOpen] = useState(false), [curlOpen, setCurlOpen] = useState(false);
  const [environments, setEnvironments] = useState(emptyEnvironments);
  const [library, setLibrary] = useState<Library>({ directory: "正在读取存储目录…", templates: [], warnings: [] });
  const [search, setSearch] = useState(""), [notice, setNotice] = useState("");
  const [loading, setLoading] = useState(true), [saving, setBusy] = useState(false), [loadError, setLoadError] = useState(false);
  const busy = !ready || loading || saving;
  const [pending, setPending] = useState<{ title: string; run: () => void }>();
  const [deleting, setDeleting] = useState(false);
  const requests = useRequests();
  const response = requests.responses[current.id] ?? emptyResponse();
  const [layout, setLayout] = useLayout();
  const [focusPane, setFocusPane] = useState<"request" | "response">();
  const [narrowLayout, setNarrowLayout] = useState(false);
  const [descriptionOpen, setDescriptionOpen] = useState(false), [moreOpen, setMoreOpen] = useState(false);
  const menu = useRef<HTMLDivElement>(null);
  useClipboardShortcuts();
  useEffect(() => {
    setDescriptionOpen(false); setMoreOpen(false); setFocusPane(undefined);
    document.querySelector('[role="tab"][aria-selected="true"]')?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [current.id]);
  useEffect(() => {
    if (!moreOpen) return;
    const outside = (event: PointerEvent) => { if (!menu.current?.contains(event.target as Node)) setMoreOpen(false); };
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape") setMoreOpen(false); };
    document.addEventListener("pointerdown", outside); document.addEventListener("keydown", escape);
    return () => { document.removeEventListener("pointerdown", outside); document.removeEventListener("keydown", escape); };
  }, [moreOpen]);

  const reload = async (serviceId = current.serviceId) => {
    try {
      const [catalogResult, libraryResult, environmentResult] = await Promise.all([
        rpc.request["list-services"](undefined), rpc.request["list-templates"]({ serviceId }), rpc.request["load-environments"]({ serviceId }),
      ]);
      if (currentService.current !== serviceId) return;
      const catalog = unwrap(catalogResult);
      setServices(catalog.services);
      const next = unwrap(libraryResult), profiles = unwrap(environmentResult);
      setLibrary({ ...next, warnings: [...catalog.warnings, ...next.warnings] });
      setEnvironments(profiles); setLoadError(false);
    } catch (error) { if (currentService.current === serviceId) { setNotice(errorText(error)); setLoadError(true); } }
  };
  useEffect(() => {
    if (!ready) return;
    const id = current.serviceId;
    setLoading(true); setSearch(""); setLibrary({ directory: "正在读取存储目录…", templates: [], warnings: [] }); setEnvironments(emptyEnvironments());
    void reload(id).finally(() => { if (currentService.current === id) setLoading(false); });
  }, [current.serviceId, ready]);
  const effectiveRequest = () => {
    if (loadError) throw new Error("服务配置未加载，请刷新后重试");
    return applyEnvironment(draft.request, selectEnvironment(environments, current.environmentId));
  };
  const send = async () => { if (busy) return; try { await requests.send(current.id, current.serviceId, effectiveRequest()); } catch (error) { setNotice(errorText(error)); } };
  const cancel = () => requests.cancel(current.id);
  const updateRequest = (patch: Partial<RequestDraft>) => setDraft((value) => ({ ...value, request: { ...value.request, ...patch } }));
  const addTab = (next: EditorTab) => {
    if (workspace.tabs.length >= MAX_TABS) { setNotice(`最多打开 ${MAX_TABS} 个标签，请先关闭不需要的标签。`); return; }
    setWorkspace((value) => ({ ...value, tabs: [...value.tabs, next], activeTabId: next.id })); setNotice("");
  };
  const open = (template?: SavedTemplate) => {
    const existing = template && workspace.tabs.find((item) => item.serviceId === service.id && item.templateId === template.id);
    if (existing) { setWorkspace((value) => ({ ...value, activeTabId: existing.id })); return; }
    const next = template ? inputOf(template) : templateForService(service);
    addTab({ ...freshTab(service.id), draft: next, baseline: next, templateId: template?.id, revision: template?.revision,
      environmentId: current.environmentId, editorTab: template && !["GET", "HEAD"].includes(template.request.method) ? "body" : "query" });
  };
  const switchService = async (next: Service) => {
    const existing = workspace.tabs.find((item) => item.serviceId === next.id);
    if (existing) setWorkspace((value) => ({ ...value, activeTabId: existing.id }));
    else { const draft = templateForService(next); addTab({ ...freshTab(next.id), draft, baseline: draft }); }
  };
  const navigate = (_title: string, run: () => void) => { if (!busy) run(); };
  const createInterface = async (template: TemplateInput) => {
    if (workspace.tabs.length >= MAX_TABS) throw new Error(`请先关闭一个标签（上限 ${MAX_TABS}）`);
    setBusy(true);
    try {
      const next = unwrap(await rpc.request["save-template"]({ serviceId: service.id, template }));
      setLibrary((value) => ({ ...value, templates: [...value.templates, next].sort((a, b) => a.name.localeCompare(b.name)) }));
      setSearch(""); open(next);
    } finally { setBusy(false); }
  };
  const save = async (copy = false): Promise<boolean> => {
    if (busy) return false;
    setBusy(true); setNotice("");
    const snapshot = current;
    const input = copy ? { ...draft, name: `${draft.name} 副本` } : draft;
    try {
      const next = unwrap(await rpc.request["save-template"]({ serviceId: snapshot.serviceId, template: input, id: copy ? undefined : snapshot.templateId, revision: copy ? undefined : snapshot.revision }));
      updateTab(snapshot.id, (item) => ({ ...item, templateId: next.id, revision: next.revision, draft: inputOf(next), baseline: inputOf(next) }));
      await reload(snapshot.serviceId); return true;
    } catch (error) { setNotice(errorText(error)); return false; }
    finally { setBusy(false); }
  };
  const closeTab = (id: string) => {
    if (busy) return;
    const target = workspace.tabs.find((item) => item.id === id)!;
    const close = () => {
      requests.close(id);
      setWorkspace((value) => {
        let tabs = value.tabs.filter((item) => item.id !== id);
        if (!tabs.length) { const draft = templateForService(service); tabs = [{ ...freshTab(service.id), draft, baseline: draft }]; }
        const index = value.tabs.findIndex((item) => item.id === id);
        return { ...value, tabs, activeTabId: value.activeTabId === id ? tabs[Math.min(index, tabs.length - 1)].id : value.activeTabId };
      });
    };
    if (isDirty(target)) { setWorkspace((value) => ({ ...value, activeTabId: id })); setPending({ title: `关闭「${target.draft.name || "未命名请求"}」`, run: close }); }
    else close();
  };
  const remove = async () => {
    if (!current.templateId || busy) return;
    setBusy(true);
    try {
      unwrap(await rpc.request["delete-template"]({ serviceId: service.id, id: current.templateId, revision: current.revision! }));
      const draft = templateForService(service);
      updateTab(current.id, (item) => ({ ...item, draft, baseline: draft, templateId: undefined, revision: undefined }));
      requests.close(current.id); setDeleting(false); await reload();
    } catch (error) { setNotice(errorText(error)); setDeleting(false); }
    finally { setBusy(false); }
  };
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.isComposing || event.defaultPrevented || pending || deleting || creatingService || creatingInterface || agentOpen || environmentOpen || curlOpen) return;
      if (event.key === "Escape" && !moreOpen) { setFocusPane(undefined); return; }
      if (!(event.metaKey || event.ctrlKey) || busy) return;
      if (event.key.toLowerCase() === "b") { event.preventDefault(); setLayout((value) => ({ ...value, sidebar: !value.sidebar })); }
      if (event.key.toLowerCase() === "s") { event.preventDefault(); void save(); }
      if (event.key === "Enter") { event.preventDefault(); void send(); }
      if (event.key.toLowerCase() === "t") { event.preventDefault(); open(); }
      if (event.key.toLowerCase() === "w") { event.preventDefault(); closeTab(current.id); }
      if (event.key.toLowerCase() === "k") { event.preventDefault(); setLayout((value) => ({ ...value, sidebar: true })); requestAnimationFrame(() => document.querySelector<HTMLInputElement>('[aria-label="搜索模板"]')?.focus()); }
    };
    window.addEventListener("keydown", handler); return () => window.removeEventListener("keydown", handler);
  });
  const filtered = library.templates.filter((item) => `${item.name} ${item.description} ${item.request.method} ${item.request.url}`.toLowerCase().includes(search.toLowerCase()));
  const selectedEnvironment = environments.profiles.find((item) => item.id === current.environmentId);
  const missing = referencedVariables(draft.request).filter((key) => !draft.request.variables.some((row) => row.key.trim() === key) && !selectedEnvironment?.variables.some((row) => row.key === key));
  let preview = "", previewError = "";
  try {
    const resolved = resolveRequest(effectiveRequest());
    preview = `${resolved.method} ${resolved.url}\n${resolved.headers.map((h) => `${h.key}: ${h.value}`).join("\n")}${resolved.body ? `\n\n${resolved.body}` : ""}`;
  } catch (error) { previewError = errorText(error); }
  const tabLabels = [["query", `参数 ${draft.request.query.filter((row) => row.enabled).length}`], ["headers", `请求头 ${draft.request.headers.filter((row) => row.enabled).length}`], ["body", "请求体"], ["variables", `变量 ${draft.request.variables.length}`], ["preview", "预览"]];

  return <div className={`app-shell ${layout.sidebar ? "" : "sidebar-hidden"}`}>
    {layout.sidebar && <aside className="sidebar">
      <div className="brand"><span className="brand-mark" aria-hidden="true">ab</span><strong>ApiBuddy</strong><button className="icon-button" title="收起侧栏 · ⌘ B" aria-label="收起侧栏" onClick={() => setLayout((value) => ({ ...value, sidebar: false }))}>‹</button></div>
      <div className="service-picker"><select id="service-picker" aria-label="当前服务" disabled={busy} value={service.id} onChange={(e) => {
        const next = services.find((item) => item.id === e.target.value);
        if (next && next.id !== service.id) navigate(`切换到「${next.name}」`, () => void switchService(next));
      }}>{services.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select><button className="icon-button" title="新建服务" aria-label="新建服务" disabled={busy} onClick={() => navigate("新建服务", () => setCreatingService(true))}>＋</button></div>
      <div className="library-actions"><button disabled={busy} onClick={() => navigate("新建接口", () => setCreatingInterface(true))}>＋ 新建接口</button><button disabled={busy || workspace.tabs.length >= MAX_TABS} onClick={() => setCurlOpen(true)}>导入 cURL</button></div>
      <label className="search"><input aria-label="搜索模板" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="搜索接口 · ⌘ K" /></label>
      <div className="library-heading"><span>接口 <b>{library.templates.length}</b></span><button title="刷新模板库" aria-label="刷新模板库" disabled={busy} onClick={() => { setBusy(true); setNotice(""); void reload().finally(() => setBusy(false)); }}>↻</button></div>
      <nav className="template-list" aria-label="请求模板">
        {filtered.map((item) => <button key={item.id} disabled={busy} title={`${item.name} · ${item.request.url}`} className={`template-item ${saved?.id === item.id ? "selected" : ""}`} onClick={() => navigate(`打开「${item.name}」`, () => open(item))}>
          <span className={`method method-${item.request.method.toLowerCase()}`}>{item.request.method}</span><span className="template-label"><strong>{item.name}</strong></span>
        </button>)}
        {!filtered.length && <div className="library-empty">{search ? "没有匹配的接口" : "暂无接口"}</div>}
      </nav>
      <button className="agent-button" disabled={busy} onClick={() => setAgentOpen(true)}>⌘ CLI 与 AI 接入 <span>↗</span></button>
      <details className="storage"><summary>本地存储</summary><code>{library.directory}</code></details>
    </aside>}
    <main className="main-workspace">
      <div className="workspace-topbar">
        {!layout.sidebar && <button className="sidebar-toggle" title="展开侧栏 · ⌘ B" aria-label="展开侧栏" onClick={() => setLayout((value) => ({ ...value, sidebar: true }))}>☰</button>}
        <div className="request-tabs" role="tablist" aria-label="打开的请求">{workspace.tabs.map((item) => <div key={item.id} className={`request-tab ${item.id === current.id ? "active" : ""}`}><button role="tab" aria-selected={item.id === current.id} disabled={busy} title={`${services.find((s) => s.id === item.serviceId)?.name || "服务"} / ${item.draft.name}`} onClick={() => setWorkspace((value) => ({ ...value, activeTabId: item.id }))}><span className={`method method-${item.draft.request.method.toLowerCase()}`}>{item.draft.request.method}</span><span>{item.draft.name || "未命名请求"}</span>{requests.responses[item.id]?.phase === "running" ? <i className="running-dot" /> : isDirty(item) ? <i className="dirty-dot" /> : null}</button><button className="tab-close" disabled={busy} aria-label={`关闭标签 ${item.draft.name}`} onClick={() => closeTab(item.id)}>×</button></div>)}<button className="tab-new" disabled={busy || workspace.tabs.length >= MAX_TABS} title="新建草稿 ⌘ T" aria-label="新建草稿标签" onClick={() => open()}>＋</button></div>
        <button className="layout-toggle" disabled={narrowLayout} title={narrowLayout ? "窗口较窄，自动上下排列；展开窗口或收起侧栏可切换" : layout.stacked ? "切换为左右排列" : "切换为上下排列"} aria-label={narrowLayout ? "窄窗自动上下排列" : layout.stacked ? "切换为左右排列" : "切换为上下排列"} onClick={() => { setFocusPane(undefined); setLayout((value) => ({ ...value, stacked: !value.stacked })); }}>{narrowLayout ? "自动" : layout.stacked ? "左右" : "上下"}</button>
      </div>
      <div className="request-toolbar">
        <span className="service-context" title={service.name}>{service.name} /</span>
        <input disabled={busy} className="template-name" placeholder="接口名称" aria-label="接口名称" maxLength={120} value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} />
        <button className={`toolbar-button ${descriptionOpen ? "active" : ""}`} aria-expanded={descriptionOpen} title={draft.description || "编辑接口说明"} onClick={() => setDescriptionOpen((value) => !value)}>说明</button>
        <div className="environment-picker"><select aria-label="当前环境" disabled={busy || loadError} value={current.environmentId || ""} onChange={(e) => updateTab(current.id, (item) => ({ ...item, environmentId: e.target.value || undefined }))}><option value="">模板默认值</option>{current.environmentId && !selectedEnvironment && <option value={current.environmentId}>环境已删除，请重新选择</option>}{environments.profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}</select><button className="toolbar-button" title="管理环境" aria-label="管理环境" disabled={busy || loadError} onClick={() => setEnvironmentOpen(true)}>环境</button></div>
        <button className="save-button" disabled={busy} title="保存接口 ⌘ S" onClick={() => void save()}>{saving ? "保存中…" : dirty ? "保存 •" : "保存"}</button>
        <div className="request-menu" ref={menu}><button className="toolbar-button" disabled={busy} aria-label="更多请求操作" aria-expanded={moreOpen} onClick={() => setMoreOpen((value) => !value)}>•••</button>{moreOpen && <div className="request-menu-content">
          <button disabled={!!previewError} onClick={async () => { setMoreOpen(false); try { unwrap(await rpc.request["clipboard-write"]({ text: exportCurl(effectiveRequest()) })); setNotice("已复制 cURL，包含当前环境的实际值"); } catch (error) { setNotice(errorText(error)); } }}>复制 cURL</button>
          <button onClick={() => { setMoreOpen(false); void save(true); }}>另存副本</button>
          <button className="danger" disabled={!saved} onClick={() => { setMoreOpen(false); setDeleting(true); }}>删除接口</button>
        </div>}</div>
      </div>
      {descriptionOpen && <div className="description-row"><input disabled={busy} aria-label="模板描述" placeholder="接口说明（可选）" maxLength={2000} value={draft.description} onChange={(e) => setDraft({ ...draft, description: e.target.value })} /><button className="icon-button" aria-label="收起接口说明" onClick={() => setDescriptionOpen(false)}>×</button></div>}
      <fieldset className="request-line" disabled={busy}><select aria-label="HTTP 方法" value={draft.request.method} onChange={(e) => updateRequest({ method: e.target.value as Method })}>{METHODS.map((method) => <option key={method}>{method}</option>)}</select><input className="url-input" aria-label="请求 URL" placeholder="https://… 或 {{base_url}}/path" value={draft.request.url} onChange={(e) => updateRequest({ url: e.target.value })} onKeyDown={(e) => { if (e.key === "Enter" && !e.nativeEvent.isComposing) { e.preventDefault(); e.stopPropagation(); void send(); } }} />{response.phase === "running" ? <button className="send-button cancel-button" onClick={() => void cancel()}>停止</button> : <button className="send-button" title="发送请求 ⌘ Enter" onClick={() => void send()}>发送 ↗</button>}</fieldset>
      {workspaceError && <div className="error-message" role="alert">{workspaceError}</div>}
      {(notice || library.warnings.length > 0) && <div className="notice" role="status"><span>{notice || library.warnings.join(" ")}</span><button aria-label="关闭提示" onClick={() => { setNotice(""); setLibrary((value) => ({ ...value, warnings: [] })); }}>×</button></div>}
      <SplitPane layout={layout} focus={focusPane} onNarrowChange={setNarrowLayout} onResize={(direction, ratio) => setLayout((value) => ({ ...value, [direction]: ratio }))} request={
        <section className="request-panel" aria-label="请求编辑器">
          <fieldset disabled={busy}>
            <div className="tabs editor-tabs"><div>{tabLabels.map(([value, label]) => <button key={value} className={tab === value ? "active" : ""} onClick={() => setTab(value)}>{label}</button>)}</div><button className="panel-expand" aria-label={focusPane === "request" ? "恢复分栏" : "放大请求区"} title={focusPane === "request" ? "恢复分栏 · Esc" : "放大请求区"} aria-pressed={focusPane === "request"} onClick={() => setFocusPane(focusPane === "request" ? undefined : "request")}>{focusPane === "request" ? "↙" : "⛶"}</button></div>
            <div className={`editor-content ${tab === "body" ? "body-content" : ""}`}>
              {tab === "query" && <PairEditor label="参数" rows={draft.request.query} onChange={(query) => updateRequest({ query })} />}
              {tab === "headers" && <PairEditor label="请求头" rows={draft.request.headers} onChange={(headers) => updateRequest({ headers })} />}
              {tab === "body" && <><div className="body-heading"><span className="hint">{["GET", "HEAD"].includes(draft.request.method) ? `${draft.request.method} 不发送请求体` : "文本 / JSON"}</span><button className="text-button" onClick={() => { try { updateRequest({ body: JSON.stringify(JSON.parse(draft.request.body), null, 2) }); } catch { setNotice("请求体不是有效 JSON，未作修改"); } }}>格式化 JSON</button></div><textarea className="body-editor" aria-label="请求体" spellCheck={false} maxLength={1024 * 1024} placeholder={'{\n  "name": "{{name}}"\n}'} value={draft.request.body} onChange={(e) => updateRequest({ body: e.target.value })} /></>}
              {tab === "variables" && <>
                {selectedEnvironment && <div className="environment-active-note"><span>{selectedEnvironment.name} 覆盖同名默认值</span><button className="text-button" onClick={() => setEnvironmentOpen(true)}>查看</button></div>}<div className="variable-labels"><span>变量名</span><span>默认值</span><span /></div>
                {draft.request.variables.map((row, index) => <div className="variable-row" key={index}><input maxLength={100} aria-label={`变量 ${index + 1} 名称`} placeholder="base_url" value={row.key} onChange={(e) => updateRequest({ variables: draft.request.variables.map((v, i) => i === index ? { ...v, key: e.target.value } : v) })} /><input maxLength={16384} aria-label={`变量 ${index + 1} 值`} placeholder="https://api.example.com" value={row.value} onChange={(e) => updateRequest({ variables: draft.request.variables.map((v, i) => i === index ? { ...v, value: e.target.value } : v) })} /><button className="icon-button" aria-label={`移除变量 ${index + 1}`} onClick={() => updateRequest({ variables: draft.request.variables.filter((_, i) => i !== index) })}>×</button></div>)}
                <button disabled={draft.request.variables.length >= 100} className="text-button add-row" onClick={() => updateRequest({ variables: [...draft.request.variables, { key: "", value: "" }] })}>＋ 添加变量</button>
                {missing.length > 0 && <button className="missing-variables" onClick={() => updateRequest({ variables: [...draft.request.variables, ...missing.map((key) => ({ key, value: "" }))] })}>补齐未定义变量：{missing.join("、")}</button>}
              </>}
              {tab === "preview" && (previewError ? <p className="error-message" role="alert">{previewError}</p> : <pre className="request-preview">{preview}</pre>)}
            </div>
            <div className="request-options"><label><input type="checkbox" checked={draft.request.stream} onChange={(e) => updateRequest({ stream: e.target.checked })} />流式显示</label><label>超时 <input type="number" aria-label="超时秒数" min={1} max={600} value={draft.request.timeoutMs / 1000} onChange={(e) => updateRequest({ timeoutMs: Math.max(1000, Math.min(600000, Number(e.target.value) * 1000 || 1000)) })} /> 秒</label></div>
          </fieldset>
        </section>
      } response={<ResponsePanel key={current.id} response={response} expanded={focusPane === "response"}
        onExpand={() => setFocusPane(focusPane === "response" ? undefined : "response")} onReveal={() => setFocusPane((value) => value === "request" ? undefined : value)}
        shortcutsDisabled={!!pending || deleting || creatingService || creatingInterface || agentOpen || environmentOpen || curlOpen} />} />
      <footer className="workspace-status"><span className={`save-indicator ${dirty ? "unsaved" : ""}`}>{dirty ? "接口未保存" : saved ? `已保存 · 修订 ${saved.revision}` : "新草稿"}</span><span title={workspaceError || draftStatus}>{draftStatus}</span><span className="shortcut-hint">⌘ Enter 发送 · ⌘ S 保存</span></footer>
    </main>
    {creatingService && <ServiceDialog onClose={() => setCreatingService(false)} onCreated={async (next) => {
      setServices((current) => [...current, next]); await switchService(next);
    }} />}
    {creatingInterface && <InterfaceDialog service={service} onClose={() => setCreatingInterface(false)} onCreate={createInterface} />}
    {environmentOpen && <EnvironmentDialog serviceId={service.id} initial={environments} onSaved={setEnvironments} onClose={() => setEnvironmentOpen(false)} />}
    {curlOpen && <CurlDialog onClose={() => setCurlOpen(false)} onImport={(draft) => { const baseline = templateForService(service); addTab({ ...freshTab(service.id), draft, baseline, editorTab: ["GET", "HEAD"].includes(draft.request.method) ? "query" : "body" }); }} />}
    {agentOpen && <AgentPanel service={service} dirty={dirty} environmentId={current.environmentId} onClose={() => setAgentOpen(false)} />}
    {(pending || deleting) && <div className="modal-backdrop"><div className="modal" role="dialog" aria-modal="true" aria-label={deleting ? "删除模板" : "未保存的修改"}>
      <h2>{deleting ? "删除这个模板？" : "先处理未保存的修改"}</h2><p>{deleting ? `「${saved?.name}」将从本地模板库删除，当前编辑内容也会丢弃。` : `继续${pending?.title}之前，可以先保存当前模板。`}</p>
      <div className="modal-actions"><button disabled={busy} autoFocus onClick={() => { setPending(undefined); setDeleting(false); }}>取消</button>{deleting ? <button className="danger-button" disabled={busy} onClick={() => void remove()}>确认删除</button> : <><button disabled={busy} onClick={() => { const action = pending; setPending(undefined); action?.run(); }}>放弃修改</button><button className="save-button" disabled={busy} onClick={async () => { const action = pending; if (await save()) { setPending(undefined); action?.run(); } }}>保存并继续</button></>}</div>
    </div></div>}
  </div>;
}
