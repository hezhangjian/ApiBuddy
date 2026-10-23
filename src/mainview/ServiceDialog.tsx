import { useState } from "react";
import type { Service, ServiceInput } from "../shared/services";
import { errorText, rpc, unwrap } from "./rpc";

export function ServiceDialog({ onClose, onCreated }: { onClose: () => void; onCreated: (service: Service) => Promise<void> }) {
  const [input, setInput] = useState<ServiceInput>({ name: "", slug: "", baseUrl: "", description: "" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const create = async () => {
    if (busy) return;
    setBusy(true); setError("");
    try { await onCreated(unwrap(await rpc.request["create-service"](input))); onClose(); }
    catch (caught) { setError(errorText(caught)); }
    finally { setBusy(false); }
  };
  return <div className="modal-backdrop"><form className="modal service-modal" role="dialog" aria-modal="true" aria-label="新建服务" onSubmit={(event) => { event.preventDefault(); void create(); }}>
    <h2>新建服务</h2><p>填写服务名称和服务 ID，创建后通过 apibuddy 调用接口。</p>
    <fieldset disabled={busy}>
      <label>服务名称<input autoFocus aria-label="服务名称" required maxLength={120} placeholder="例如：订单服务" value={input.name} onChange={(e) => setInput({ ...input, name: e.target.value })} /></label>
      <label>服务 ID<input aria-label="服务 ID" aria-describedby="service-id-hint service-command-preview" required maxLength={48} pattern="[a-z][a-z0-9-]*" title="使用小写字母、数字和短横线，并以字母开头" autoCapitalize="none" spellCheck={false} placeholder="例如：orders" value={input.slug} onChange={(e) => setInput({ ...input, slug: e.target.value })} /></label>
      <p className="hint" id="service-id-hint">用于命令中的服务名，不能重复。使用小写字母、数字和短横线，以字母开头。</p>
      <p id="service-command-preview">调用方式：<code>apibuddy {input.slug || "<服务 ID>"} &lt;接口名&gt;</code></p>
      <label>服务地址<input aria-label="服务默认地址" placeholder="https://api.example.com" value={input.baseUrl} onChange={(e) => setInput({ ...input, baseUrl: e.target.value })} /></label>
      <label>服务描述<input aria-label="服务描述" maxLength={2000} placeholder="这个服务提供什么能力？" value={input.description} onChange={(e) => setInput({ ...input, description: e.target.value })} /></label>
      {error && <div className="error-message" role="alert">{error}</div>}
      <div className="modal-actions"><button type="button" onClick={onClose}>取消</button><button className="save-button" type="submit">{busy ? "创建中…" : "创建服务"}</button></div>
    </fieldset>
  </form></div>;
}
