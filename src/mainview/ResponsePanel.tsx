import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ResponseState } from "./use-request";
import { rpc, unwrap, errorText } from "./rpc";
import { jsonPointer } from "../shared/json-view";
import { findText, nextMatch } from "../shared/response-text";
import { JsonTree } from "./JsonTree";
import { ResponseText } from "./ResponseText";

const labels = { idle: "等待请求", running: "请求进行中", completed: "已完成", cancelled: "已取消", failed: "请求失败", "timed-out": "已超时" };
export function ResponsePanel({ response, expanded, onExpand, onReveal, shortcutsDisabled }: {
  response: ResponseState; expanded: boolean; onExpand: () => void; onReveal: () => void; shortcutsDisabled: boolean;
}) {
  const [searchOpen, setSearchOpen] = useState(false), [pathOpen, setPathOpen] = useState(false);
  const [tab, setTab] = useState("Body"), [mode, setMode] = useState("pretty");
  const [notice, setNotice] = useState({ text: "", error: false });
  const [search, setSearch] = useState(""), [pointer, setPointer] = useState("");
  const [activeMatch, setActiveMatch] = useState(0), [wrap, setWrap] = useState(true), [follow, setFollow] = useState(true);
  const searchInput = useRef<HTMLInputElement>(null);
  const parsed = useMemo(() => {
    if (response.phase === "running" || !response.body) return undefined;
    try { return { value: JSON.parse(response.body) as unknown }; } catch { return undefined; }
  }, [response.body, response.phase]);
  const selected = useMemo(() => {
    if (!parsed) return undefined;
    try { return { value: jsonPointer(parsed.value, pointer), error: "" }; }
    catch (error) { return { value: parsed.value, error: errorText(error) }; }
  }, [parsed, pointer]);
  const text = useMemo(() => {
    if (tab === "Headers") return response.meta?.headers.map(([key, value]) => `${key}: ${value}`).join("\n") ?? "";
    if (tab === "Raw") return response.meta ? `HTTP ${response.meta.status} ${response.meta.statusText}\n${response.meta.headers.map(([k, v]) => `${k}: ${v}`).join("\n")}\n\n${response.body}` : response.body;
    return selected ? JSON.stringify(selected.value, null, 2) : response.body;
  }, [response.body, response.meta, tab, selected]);
  const matches = useMemo(() => findText(text, search), [text, search]);
  const active = Math.min(activeMatch, Math.max(0, matches.ranges.length - 1));
  const goToMatch = (step: number) => setActiveMatch(nextMatch(active, matches.ranges.length, step));
  useEffect(() => { setActiveMatch(0); }, [text, search]);
  useEffect(() => { if (response.phase === "running") setFollow(true); }, [response.phase]);
  useEffect(() => {
    if (!notice.text || notice.error) return;
    const timer = setTimeout(() => setNotice({ text: "", error: false }), 2500);
    return () => clearTimeout(timer);
  }, [notice]);
  const showSearch = () => {
    onReveal(); setSearchOpen(true);
    requestAnimationFrame(() => { searchInput.current?.focus(); searchInput.current?.select(); });
  };
  const closeSearch = () => { setSearchOpen(false); setSearch(""); };
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (shortcutsDisabled || event.isComposing || !(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== "f") return;
      event.preventDefault(); showSearch();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });
  const copy = async (value: string, label: string) => {
    try { unwrap(await rpc.request["clipboard-write"]({ text: value })); setNotice({ text: `已复制${label}`, error: false }); }
    catch (error) { setNotice({ text: errorText(error), error: true }); }
  };
  const onFollow = useCallback((value: boolean) => setFollow(value), []);
  const treeVisible = mode === "tree" && tab === "Body" && selected && !search;
  return <section className="response-panel" aria-label="响应">
    <div className="response-heading">
      <strong>响应</strong>
      <div className="metrics"><span className={response.meta && response.meta.status < 400 ? "status-ok" : response.meta ? "status-error" : ""}>{response.meta ? `${response.meta.status} ${response.meta.statusText}` : labels[response.phase]}</span>{response.phase !== "idle" && <><span>{(response.elapsedMs / 1000).toFixed(2)} s</span><span>{(response.bytes / 1024).toFixed(1)} KiB</span></>}</div>
      <button className="panel-expand" title={expanded ? "恢复分栏 · Esc" : "放大响应区"} aria-label={expanded ? "恢复分栏" : "放大响应区"} aria-pressed={expanded} onClick={onExpand}>{expanded ? "↙" : "⛶"}</button>
    </div>
    <div className="tabs response-tabs">
      <div>{["Body", "Headers", "Raw"].map((name) => <button key={name} className={tab === name ? "active" : ""} onClick={() => { setTab(name); setNotice({ text: "", error: false }); }}>{name}</button>)}</div>
      <div className="response-actions">
        {tab === "Body" && parsed && <select className="response-mode" aria-label="响应显示方式" value={mode} onChange={(e) => setMode(e.target.value)}><option value="pretty">格式化</option><option value="tree">JSON 树</option></select>}
        <button disabled={!text} className={searchOpen ? "active" : ""} title="查找响应 · ⌘ F" aria-label="查找响应" aria-expanded={searchOpen} onClick={() => searchOpen ? closeSearch() : showSearch()}>查找</button>
        {tab === "Body" && parsed && <button className={pathOpen ? "active" : ""} aria-expanded={pathOpen} onClick={() => { setPathOpen(!pathOpen); if (pathOpen) setPointer(""); }}>字段</button>}
        <button title={pointer && tab === "Body" ? "复制字段值" : "复制响应"} aria-label={pointer && tab === "Body" ? "复制字段值" : "复制响应"} disabled={!text || !!selected?.error && tab === "Body"} onClick={() => void copy(text, pointer && tab === "Body" ? "字段值" : "响应")}>复制</button>
      </div>
    </div>
    {searchOpen && <div className="response-tools">
      <div className="response-search"><input ref={searchInput} aria-label="搜索响应" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="查找 · Enter 下一处" maxLength={500} onKeyDown={(e) => {
        if (e.nativeEvent.isComposing) return;
        if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); closeSearch(); }
        if (e.key === "Enter" && !e.metaKey && !e.ctrlKey) { e.preventDefault(); goToMatch(e.shiftKey ? -1 : 1); }
      }} /><span aria-live="polite">{search ? `${matches.ranges.length ? active + 1 : 0}/${matches.ranges.length}${matches.truncated ? "+" : ""}` : ""}</span></div>
      <button className="search-step" title="上一处 · Shift Enter" aria-label="上一个匹配" disabled={!matches.ranges.length} onClick={() => goToMatch(-1)}>↑</button>
      <button className="search-step" title="下一处 · Enter" aria-label="下一个匹配" disabled={!matches.ranges.length} onClick={() => goToMatch(1)}>↓</button>
      <button className="icon-button" aria-label="关闭响应查找" onClick={closeSearch}>×</button>
    </div>}
    {pathOpen && tab === "Body" && parsed && <div className="pointer-row"><input aria-label="JSON 字段路径" placeholder="字段路径 /data/items/0" value={pointer} onChange={(e) => setPointer(e.target.value)} /><button className="text-button" disabled={!pointer} onClick={() => setPointer("")}>全部</button><button className="text-button" disabled={!pointer || !!selected?.error} onClick={() => void copy(pointer, "路径")}>复制路径</button></div>}
    {notice.error && <p className="error-message" role="alert">{notice.text}<button className="icon-button" aria-label="关闭复制错误" onClick={() => setNotice({ text: "", error: false })}>×</button></p>}
    {selected?.error && tab === "Body" && <p className="error-message" role="alert">{selected.error}；下方保留完整响应。</p>}
    {response.error && <p className="error-message" role="alert">{response.error}</p>}
    {text ? treeVisible ? <div className="json-tree"><JsonTree key={`${pointer}:${response.elapsedMs}:${response.bytes}`} value={selected.value} path={selected.error ? "" : pointer} onPath={(path) => { setPointer(path); setPathOpen(true); }} /></div>
      : <ResponseText text={text} json={tab === "Body" && !!parsed} wrap={wrap} matches={matches.ranges} activeMatch={active} running={response.phase === "running"} follow={follow} onFollow={onFollow} />
      : <div className="response-empty"><span>{response.phase === "idle" ? "发送请求后查看响应" : response.phase === "completed" ? "响应正文为空" : labels[response.phase]}</span>{response.phase === "idle" && <kbd>⌘ Enter</kbd>}</div>}
    <div className="response-footer">
      <span role="status">{notice.text && !notice.error ? notice.text : labels[response.phase]}{["cancelled", "timed-out"].includes(response.phase) ? " · 已保留收到的内容" : ""}</span>
      <div>{response.phase === "running" && <button disabled={!!search} aria-pressed={follow} title="跟随最新响应；向上滚动暂停" onClick={() => setFollow(!follow)}>{follow ? "跟随最新" : "继续跟随"}</button>}
        {!treeVisible && <button aria-label="响应自动换行" aria-pressed={wrap} onClick={() => setWrap(!wrap)}>{wrap ? "自动换行" : "不换行"}</button>}<span>{parsed ? "JSON" : "文本"}</span>
      </div>
    </div>
  </section>;
}
