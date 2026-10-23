import { useMemo, useState } from "react";
import type { ResponseState } from "./use-request";
import { rpc, unwrap, errorText } from "./rpc";
const labels = { idle: "等待请求", running: "请求进行中", completed: "已完成", cancelled: "已取消", failed: "请求失败", "timed-out": "已超时" };
export function ResponsePanel({ response }: { response: ResponseState }) {
  const [tab, setTab] = useState("Body");
  const [notice, setNotice] = useState("");
  const text = useMemo(() => {
    if (tab === "Headers") return response.meta?.headers.map(([key, value]) => `${key}: ${value}`).join("\n") ?? "";
    if (tab === "Raw") return response.meta ? `HTTP ${response.meta.status} ${response.meta.statusText}\n${response.meta.headers.map(([k, v]) => `${k}: ${v}`).join("\n")}\n\n${response.body}` : response.body;
    if (response.phase !== "running") { try { return JSON.stringify(JSON.parse(response.body), null, 2); } catch { /* Display non-JSON as text. */ } }
    return response.body;
  }, [response, tab]);
  return <section className="response-panel">
    <div className="section-heading"><div><span className="eyebrow">RESPONSE</span><h2>响应</h2></div>
      <div className="metrics"><span className={response.meta && response.meta.status < 400 ? "status-ok" : ""}>{response.meta ? `${response.meta.status} ${response.meta.statusText}` : labels[response.phase]}</span><span>{(response.elapsedMs / 1000).toFixed(2)} s</span><span>{(response.bytes / 1024).toFixed(1)} KiB</span></div>
    </div>
    <div className="tabs"><div>{["Body", "Headers", "Raw"].map((name) => <button key={name} className={tab === name ? "active" : ""} onClick={() => { setTab(name); setNotice(""); }}>{name}</button>)}</div>
      <button className="text-button" disabled={!text} onClick={async () => { try { unwrap(await rpc.request["clipboard-write"]({ text })); setNotice("已复制"); } catch (e) { setNotice(errorText(e)); } }}>{notice || "复制响应"}</button>
    </div>
    {response.error && <p className="error-message" role="alert">{response.error}</p>}
    {["cancelled", "timed-out"].includes(response.phase) && <p className="hint">{labels[response.phase]} · 已保留收到的内容</p>}
    {text ? <pre className="response-body">{text}</pre> : <div className="response-empty"><span className="empty-symbol">↗</span><h3>{response.phase === "idle" ? "准备好，发送一次请求" : labels[response.phase]}</h3><p>构筑模板，替换变量，在这里查看结果。</p><kbd>⌘ Enter</kbd><span className="hint"> 发送请求</span></div>}
    <div className="response-footer"><span>{labels[response.phase]}</span><span>文本响应 · 最大 8 MiB</span></div>
  </section>;
}
