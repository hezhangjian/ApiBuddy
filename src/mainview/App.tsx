import { useEffect, useState } from "react";
import { METHODS, newTemplate, referencedVariables, resolveRequest, type Library, type Method, type RequestDraft, type SavedTemplate, type TemplateInput } from "../shared/templates";
import { DEFAULT_SERVICE, type Service } from "../shared/services";
import { templateForService } from "../shared/service-templates";
import { AgentPanel } from "./AgentPanel";
import { InterfaceDialog } from "./InterfaceDialog";
import { ServiceDialog } from "./ServiceDialog";
import { PairEditor } from "./PairEditor";
import { ResponsePanel } from "./ResponsePanel";
import { errorText, rpc, unwrap } from "./rpc";
import { useRequest } from "./use-request";
import { useClipboardShortcuts } from "./clipboard";

const inputOf = ({ name, description, request }: TemplateInput): TemplateInput => ({ name, description, request });
export default function App() {
  const [services, setServices] = useState<Service[]>([DEFAULT_SERVICE]);
  const [service, setService] = useState<Service>(DEFAULT_SERVICE);
  const [creatingService, setCreatingService] = useState(false);
  const [creatingInterface, setCreatingInterface] = useState(false);
  const [agentOpen, setAgentOpen] = useState(false);
  const [draft, setDraft] = useState<TemplateInput>(newTemplate);
  const [baseline, setBaseline] = useState(() => JSON.stringify(newTemplate()));
  const [saved, setSaved] = useState<SavedTemplate>();
  const [library, setLibrary] = useState<Library>({ directory: "正在读取存储目录…", templates: [], warnings: [] });
  const [search, setSearch] = useState("");
  const [tab, setTab] = useState("variables");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(true);
  const [pending, setPending] = useState<{ title: string; run: () => void }>();
  const [deleting, setDeleting] = useState(false);
  const { response, send, cancel, reset } = useRequest(service.id);
  const dirty = JSON.stringify(draft) !== baseline;
  useClipboardShortcuts();

  const reload = async () => {
    try {
      const [catalogResult, libraryResult] = await Promise.all([
        rpc.request["list-services"](undefined), rpc.request["list-templates"]({ serviceId: service.id }),
      ]);
      const catalog = unwrap(catalogResult);
      setServices(catalog.services);
      const next = unwrap(libraryResult);
      setLibrary({ ...next, warnings: [...catalog.warnings, ...next.warnings] });
    } catch (error) { setNotice(errorText(error)); }
  };
  useEffect(() => { void reload().finally(() => setBusy(false)); }, []);
  const switchService = async (next: Service) => {
    setBusy(true); setNotice("");
    try {
      const nextLibrary = unwrap(await rpc.request["list-templates"]({ serviceId: next.id }));
      const nextDraft = templateForService(next);
      reset(); setService(next); setLibrary(nextLibrary); setSearch("");
      setSaved(undefined); setDraft(nextDraft); setBaseline(JSON.stringify(nextDraft));
    } catch (error) { setNotice(errorText(error)); }
    finally { setBusy(false); }
  };
  const updateRequest = (patch: Partial<RequestDraft>) => setDraft((value) => ({ ...value, request: { ...value.request, ...patch } }));
  const open = (template?: SavedTemplate) => {
    const next = template ? inputOf(template) : templateForService(service);
    setDraft(next); setBaseline(JSON.stringify(next)); setSaved(template); setNotice(""); reset();
  };
  const navigate = (title: string, run: () => void) => {
    if (busy) return;
    if (dirty) setPending({ title, run }); else run();
  };
  const createInterface = async (template: TemplateInput) => {
    setBusy(true);
    try {
      const next = unwrap(await rpc.request["save-template"]({ serviceId: service.id, template }));
      setLibrary((current) => ({ ...current, templates: [...current.templates, next].sort((a, b) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id)) }));
      setSearch(""); open(next);
      setTab(next.request.variables.some((variable) => variable.key !== "base_url") ? "variables" : ["GET", "HEAD"].includes(next.request.method) ? "query" : "body");
      setNotice(`已创建「${next.name}」，可继续配置请求并保存。`);
    } finally { setBusy(false); }
  };
  const save = async (copy = false): Promise<boolean> => {
    if (busy) return false;
    setBusy(true); setNotice("");
    const snapshot = copy ? { ...draft, name: `${draft.name} 副本` } : draft;
    try {
      const next = unwrap(await rpc.request["save-template"]({ serviceId: service.id, template: snapshot, id: copy ? undefined : saved?.id, revision: copy ? undefined : saved?.revision }));
      setSaved(next); setDraft(inputOf(next)); setBaseline(JSON.stringify(inputOf(next)));
      setNotice(copy ? "副本已保存" : "模板已保存"); await reload(); return true;
    } catch (error) { setNotice(errorText(error)); return false; }
    finally { setBusy(false); }
  };
  const remove = async () => {
    if (!saved || busy) return;
    setBusy(true);
    try { unwrap(await rpc.request["delete-template"]({ serviceId: service.id, id: saved.id, revision: saved.revision })); setDeleting(false); open(); await reload(); }
    catch (error) { setNotice(errorText(error)); setDeleting(false); }
    finally { setBusy(false); }
  };
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.isComposing || !(event.metaKey || event.ctrlKey) || pending || deleting || busy || creatingService || creatingInterface || agentOpen) return;
      if (event.key.toLowerCase() === "s") { event.preventDefault(); void save(); }
      if (event.key === "Enter") { event.preventDefault(); void send(draft.request); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });
  const filtered = library.templates.filter((item) => `${item.name} ${item.description} ${item.request.method} ${item.request.url}`.toLowerCase().includes(search.toLowerCase()));
  const missing = referencedVariables(draft.request).filter((key) => !draft.request.variables.some((row) => row.key.trim() === key));
  let preview = "";
  let previewError = "";
  try {
    const resolved = resolveRequest(draft.request);
    preview = `${resolved.method} ${resolved.url}\n${resolved.headers.map((h) => `${h.key}: ${h.value}`).join("\n")}${resolved.body ? `\n\n${resolved.body}` : ""}`;
  } catch (error) { previewError = errorText(error); }
  const tabLabels = [
    ["query", `参数 ${draft.request.query.filter((row) => row.enabled).length}`],
    ["headers", `请求头 ${draft.request.headers.filter((row) => row.enabled).length}`],
    ["body", "请求体"], ["variables", `变量 ${draft.request.variables.length}`], ["preview", "预览"],
  ];

  return <div className="app-shell">
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark" aria-hidden="true">ab</span><div><strong>ApiBuddy</strong><span>请求工作台</span></div><span className="local-badge">LOCAL</span></div>
      <div className="service-picker"><label htmlFor="service-picker">当前服务</label><select id="service-picker" aria-label="当前服务" disabled={busy} value={service.id} onChange={(e) => {
        const next = services.find((item) => item.id === e.target.value);
        if (next && next.id !== service.id) navigate(`切换到「${next.name}」`, () => void switchService(next));
      }}>{services.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select><button className="text-button" disabled={busy} onClick={() => navigate("新建服务", () => setCreatingService(true))}>＋ 新建服务</button></div>
      <button className="new-template" disabled={busy} onClick={() => navigate("新建接口", () => setCreatingInterface(true))}>＋ 新建接口</button>
      <label className="search"><span>⌕</span><input aria-label="搜索模板" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="搜索名称、URL…" /></label>
      <div className="library-heading"><span>服务接口 <b>{library.templates.length}</b></span><button title="刷新模板库" aria-label="刷新模板库" disabled={busy} onClick={() => { setBusy(true); setNotice(""); void reload().finally(() => setBusy(false)); }}>↻</button></div>
      <nav className="template-list" aria-label="请求模板">
        {filtered.map((item) => <button key={item.id} disabled={busy} className={`template-item ${saved?.id === item.id ? "selected" : ""}`} onClick={() => navigate(`打开「${item.name}」`, () => open(item))}>
          <span className={`method method-${item.request.method.toLowerCase()}`}>{item.request.method}</span><span className="template-label"><strong>{item.name}</strong><small>{item.request.url || "尚未设置 URL"}</small></span>
        </button>)}
        {!filtered.length && <div className="library-empty">{search ? "没有匹配的模板" : <>把常用请求存下来。<br />下一次，从这里开始。</>}</div>}
      </nav>
      <button className="agent-button" disabled={busy} onClick={() => setAgentOpen(true)}>⌘ CLI 与 AI 接入 <span>apibuddy 服务 接口 ↗</span></button>
      <footer className="storage"><div><span className="storage-dot" />本地模板库</div><code title={library.directory}>{library.directory}</code><span>通过 APIBUDDY_DIR 自定义目录</span></footer>
    </aside>
    <main className="main-workspace">
      <header className="workspace-header"><div><span className="eyebrow">BUILD · SAVE · REUSE</span><h1>{service.name} <span className="service-subtitle">/ 接口</span></h1></div><span className={`save-indicator ${dirty ? "unsaved" : ""}`}>{dirty ? "● 未保存修改" : saved ? "✓ 已保存" : "新模板"}</span></header>
      {(notice || library.warnings.length > 0) && <div className="notice" role="status"><span>{notice || library.warnings.join(" ")}</span><button aria-label="关闭提示" onClick={() => { setNotice(""); setLibrary((value) => ({ ...value, warnings: [] })); }}>×</button></div>}
      <div className="work-panels">
        <section className="request-panel">
          <fieldset disabled={busy}>
            <div className="section-heading"><div><span className="eyebrow">REQUEST TEMPLATE</span><h2>构筑请求</h2></div><span className="revision">{saved ? `修订 ${saved.revision}` : "草稿"}</span></div>
            <div className="template-meta"><label>接口名称<input className="template-name" placeholder="例如 get-order，中文也可以" aria-label="接口名称" maxLength={120} value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} /></label><input aria-label="模板描述" className="description" placeholder="这个接口做什么？例如：根据 ID 查询订单" maxLength={2000} value={draft.description} onChange={(e) => setDraft({ ...draft, description: e.target.value })} /></div>
            <div className="request-line"><select aria-label="HTTP 方法" value={draft.request.method} onChange={(e) => updateRequest({ method: e.target.value as Method })}>{METHODS.map((method) => <option key={method}>{method}</option>)}</select><input className="url-input" aria-label="请求 URL" placeholder="https://… 或 {{base_url}}/path" value={draft.request.url} onChange={(e) => updateRequest({ url: e.target.value })} onKeyDown={(e) => { if (e.key === "Enter" && !e.nativeEvent.isComposing) { e.preventDefault(); void send(draft.request); } }} /></div>
            <div className="tabs editor-tabs"><div>{tabLabels.map(([value, label]) => <button key={value} className={tab === value ? "active" : ""} onClick={() => setTab(value)}>{label}</button>)}</div></div>
            <div className="editor-content">
              {tab === "query" && <><p className="hint">参数会自动进行 URL 编码，同名参数按行保留。</p><PairEditor label="参数" rows={draft.request.query} onChange={(query) => updateRequest({ query })} /></>}
              {tab === "headers" && <><p className="hint">值中可以使用 {"{{变量}}"}，例如 Bearer {"{{token}}"}。</p><PairEditor label="请求头" rows={draft.request.headers} onChange={(headers) => updateRequest({ headers })} /></>}
              {tab === "body" && <><div className="body-heading"><span className="hint">文本 / JSON · 支持 {"{{变量}}"}</span><button className="text-button" onClick={() => { try { updateRequest({ body: JSON.stringify(JSON.parse(draft.request.body), null, 2) }); } catch { setNotice("请求体不是有效 JSON，未作修改"); } }}>格式化 JSON</button></div>{["GET", "HEAD"].includes(draft.request.method) && <p className="hint">{draft.request.method} 不发送请求体，内容仍保存在模板中。</p>}<textarea className="body-editor" aria-label="请求体" spellCheck={false} placeholder={'{\n  "name": "{{name}}"\n}'} value={draft.request.body} onChange={(e) => updateRequest({ body: e.target.value })} /></>}
              {tab === "variables" && <><div className="variable-intro"><span className="variable-icon">{"{ }"}</span><div><strong>写一次，换个变量再用。</strong><p>在 URL、参数、请求头和请求体中使用 {"{{变量名}}"}。</p></div></div>
                <div className="variable-labels"><span>变量名</span><span>默认值</span><span /></div>
                {draft.request.variables.map((row, index) => <div className="variable-row" key={index}><input aria-label={`变量 ${index + 1} 名称`} placeholder="base_url" value={row.key} onChange={(e) => updateRequest({ variables: draft.request.variables.map((v, i) => i === index ? { ...v, key: e.target.value } : v) })} /><input aria-label={`变量 ${index + 1} 值`} placeholder="https://api.example.com" value={row.value} onChange={(e) => updateRequest({ variables: draft.request.variables.map((v, i) => i === index ? { ...v, value: e.target.value } : v) })} /><button className="icon-button" aria-label={`移除变量 ${index + 1}`} onClick={() => updateRequest({ variables: draft.request.variables.filter((_, i) => i !== index) })}>×</button></div>)}
                <button className="text-button add-row" onClick={() => updateRequest({ variables: [...draft.request.variables, { key: "", value: "" }] })}>＋ 添加变量</button>
                {missing.length > 0 && <button className="missing-variables" onClick={() => updateRequest({ variables: [...draft.request.variables, ...missing.map((key) => ({ key, value: "" }))] })}>补齐未定义变量：{missing.join("、")}</button>}
                <p className="storage-note">默认值随模板以明文存储。临时凭据可填写后直接发送，无需保存。</p>
              </>}
              {tab === "preview" && <><p className="hint">这是替换变量后的请求。文本直接替换，不自动转义 JSON。</p>{previewError ? <p className="error-message" role="alert">{previewError}</p> : <pre className="request-preview">{preview}</pre>}</>}
            </div>
            <div className="request-options"><label><input type="checkbox" checked={draft.request.stream} onChange={(e) => updateRequest({ stream: e.target.checked })} />流式显示</label><label>超时 <input type="number" aria-label="超时秒数" min={1} max={600} value={draft.request.timeoutMs / 1000} onChange={(e) => updateRequest({ timeoutMs: Number(e.target.value) * 1000 })} /> 秒</label></div>
            <div className="send-row">{response.phase === "running" ? <button className="send-button cancel-button" onClick={() => void cancel()}>停止请求</button> : <button className="send-button" onClick={() => void send(draft.request)}>发送请求 <span>↗</span></button>}<span className="hint">⌘ Enter</span></div>
            <div className="template-actions"><button className="save-button" onClick={() => void save()}>{busy ? "保存中…" : "保存接口"}<kbd>⌘ S</kbd></button><button onClick={() => void save(true)}>另存副本</button><button className="danger text-button" disabled={!saved} onClick={() => setDeleting(true)}>删除</button></div>
          </fieldset>
        </section>
        <ResponsePanel response={response} />
      </div>
    </main>
    {creatingService && <ServiceDialog onClose={() => setCreatingService(false)} onCreated={async (next) => {
      setServices((current) => [...current, next]); await switchService(next);
    }} />}
    {creatingInterface && <InterfaceDialog service={service} onClose={() => setCreatingInterface(false)} onCreate={createInterface} />}
    {agentOpen && <AgentPanel service={service} dirty={dirty} onClose={() => setAgentOpen(false)} />}
    {(pending || deleting) && <div className="modal-backdrop"><div className="modal" role="dialog" aria-modal="true" aria-label={deleting ? "删除模板" : "未保存的修改"}>
      <h2>{deleting ? "删除这个模板？" : "先处理未保存的修改"}</h2><p>{deleting ? `「${saved?.name}」将从本地模板库删除，当前编辑内容也会丢弃。` : `继续${pending?.title}之前，可以先保存当前模板。`}</p>
      <div className="modal-actions"><button disabled={busy} autoFocus onClick={() => { setPending(undefined); setDeleting(false); }}>取消</button>{deleting ? <button className="danger-button" disabled={busy} onClick={() => void remove()}>确认删除</button> : <><button disabled={busy} onClick={() => { const action = pending; setPending(undefined); action?.run(); }}>放弃修改</button><button className="save-button" disabled={busy} onClick={async () => { const action = pending; if (await save()) { setPending(undefined); action?.run(); } }}>保存并继续</button></>}</div>
    </div></div>}
  </div>;
}
