import { useMemo, useState } from "react";
import { importCurl } from "../shared/curl";
import type { TemplateInput } from "../shared/templates";
import { errorText, rpc, unwrap } from "./rpc";
export function CurlDialog({ onImport, onClose }: { onImport: (template: TemplateInput) => void; onClose: () => void }) {
  const [source, setSource] = useState(""), [notice, setNotice] = useState("");
  const parsed = useMemo(() => { if (!source.trim()) return undefined; try { return importCurl(source); } catch (error) { return { error: errorText(error) }; } }, [source]);
  return <div className="modal-backdrop"><form className="modal curl-modal" role="dialog" aria-modal="true" aria-label="导入 cURL" onSubmit={(event) => { event.preventDefault(); if (parsed && "template" in parsed) { onImport(parsed.template); onClose(); } }}>
    <div className="agent-heading"><div><span className="eyebrow">IMPORT REQUEST</span><h2>把 cURL 变成可编辑的请求</h2></div><button type="button" onClick={onClose} aria-label="关闭 cURL 导入">×</button></div>
    <p>粘贴终端命令或浏览器的 Copy as cURL（bash），将在新标签中打开。</p>
    <textarea autoFocus aria-label="cURL 命令" className="body-editor curl-input" spellCheck={false} value={source} maxLength={1024 * 1024} onChange={(e) => setSource(e.target.value)} placeholder={"curl 'https://api.example.com/orders' \\\n  -H 'Accept: application/json'"} />
    <button type="button" className="text-button" onClick={async () => { try { const value = unwrap(await rpc.request["clipboard-read"](undefined)); if (value) setSource(value); else setNotice("剪贴板为空"); } catch (error) { setNotice(errorText(error)); } }}>从剪贴板粘贴</button>
    {parsed && "error" in parsed && <div className="error-message" role="alert">{parsed.error}</div>}
    {parsed && "template" in parsed && <div className="curl-preview"><span className={`method method-${parsed.template.request.method.toLowerCase()}`}>{parsed.template.request.method}</span><code>{parsed.template.request.url}</code><span>{parsed.template.request.headers.length} 个请求头 · {parsed.template.request.body.length} 字符请求体</span>{parsed.warnings.map((warning) => <small key={warning}>{warning}</small>)}</div>}
    {notice && <p role="status">{notice}</p>}
    <div className="modal-actions"><button type="button" onClick={onClose}>取消</button><button className="save-button" type="submit" disabled={!parsed || !("template" in parsed)}>导入到新标签</button></div>
  </form></div>;
}
