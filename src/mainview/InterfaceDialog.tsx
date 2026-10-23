import { useRef, useState } from "react";
import type { Service } from "../shared/services";
import { buildServiceTemplate } from "../shared/service-templates";
import { METHODS, type Method, type TemplateInput } from "../shared/templates";
import { errorText } from "./rpc";

export function InterfaceDialog({ service, onClose, onCreate }: {
  service: Service; onClose: () => void; onCreate: (template: TemplateInput) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [method, setMethod] = useState<Method>("GET");
  const [endpoint, setEndpoint] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const submitting = useRef(false);
  const create = async () => {
    if (submitting.current) return;
    submitting.current = true; setBusy(true); setError("");
    try { await onCreate(buildServiceTemplate(service, { name, method, endpoint })); onClose(); }
    catch (caught) { setError(errorText(caught)); }
    finally { submitting.current = false; setBusy(false); }
  };
  const preview = endpoint.startsWith("/") ? service.baseUrl.replace(/\/+$/, "") + endpoint : endpoint;
  return <div className="modal-backdrop"><form className="modal interface-modal" role="dialog" aria-modal="true" aria-label="新建接口"
    onKeyDown={(event) => {
      if (event.key === "Escape" && !busy) { event.preventDefault(); onClose(); }
      if (event.key === "Tab") {
        const fields = [...event.currentTarget.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLButtonElement>("input, select, button")].filter((field) => !field.disabled);
        const first = fields[0], last = fields[fields.length - 1];
        if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
        if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
      }
    }}
    onSubmit={(event) => { event.preventDefault(); void create(); }}>
    <h2>新建接口</h2><p>添加到「{service.name}」<code> · apibuddy {service.slug}</code></p>
    <fieldset disabled={busy}>
      <label>接口名称<input autoFocus aria-label="新接口名称" required maxLength={120} placeholder="例如 get-order，中文也可以" value={name} onChange={(event) => setName(event.target.value)} /></label>
      <label>请求方法<select aria-label="新接口请求方法" value={method} onChange={(event) => setMethod(event.target.value as Method)}>{METHODS.map((value) => <option key={value}>{value}</option>)}</select></label>
      <label>接口路径或 URL<input aria-label="新接口路径或 URL" aria-describedby="interface-path-hint" required maxLength={16384} spellCheck={false} placeholder={service.baseUrl ? "/orders/{{id}}" : "https://api.example.com/orders"} value={endpoint} onChange={(event) => setEndpoint(event.target.value)} /></label>
      <p className="hint" id="interface-path-hint">{service.baseUrl ? <>以 / 开头会接在服务地址后，也可以填写完整 URL。</> : <>当前服务未设置地址，请填写完整 URL。</>} 使用 {"{{id}}"} 等变量，创建后可填写变量值。</p>
      {preview && <div className="interface-url-preview"><span>请求地址</span><code>{method} {preview}</code></div>}
      {error && <div className="error-message" role="alert">{error}</div>}
      <div className="modal-actions"><button type="button" onClick={onClose}>取消</button><button className="save-button" type="submit">{busy ? "创建中…" : "创建接口"}</button></div>
      <p className="interface-save-note">创建后保存到左侧列表，可继续配置请求参数、请求头和正文。</p>
    </fieldset>
  </form></div>;
}
