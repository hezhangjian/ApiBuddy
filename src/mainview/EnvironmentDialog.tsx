import { useState } from "react";
import type { Environments, Environment } from "../shared/environments";
import { errorText, rpc, unwrap } from "./rpc";

export function EnvironmentDialog({ serviceId, initial, onSaved, onClose }: {
  serviceId: string; initial: Environments; onSaved: (data: Environments) => void; onClose: () => void;
}) {
  const [data, setData] = useState(() => structuredClone(initial));
  const [selected, setSelected] = useState(initial.profiles[0]?.id ?? "");
  const [busy, setBusy] = useState(false), [error, setError] = useState(""), [removing, setRemoving] = useState(false);
  const profile = data.profiles.find((item) => item.id === selected);
  const update = (patch: Partial<Environment>) => setData((value) => ({ ...value, profiles: value.profiles.map((item) => item.id === selected ? { ...item, ...patch } : item) }));
  const create = () => {
    const next: Environment = { id: crypto.randomUUID(), name: "", variables: [{ key: "base_url", value: "" }] };
    setData((value) => ({ ...value, profiles: [...value.profiles, next] })); setSelected(next.id); setRemoving(false);
  };
  return <div className="modal-backdrop"><form className="modal environment-modal" role="dialog" aria-modal="true" aria-label="管理环境" onSubmit={async (event) => {
    event.preventDefault(); setBusy(true); setError("");
    try { const saved = unwrap(await rpc.request["save-environments"]({ serviceId, data })); onSaved(saved); onClose(); }
    catch (caught) { setError(errorText(caught)); } finally { setBusy(false); }
  }}>
    <div className="agent-heading"><div><span className="eyebrow">ENVIRONMENTS</span><h2>一份请求，切换不同环境</h2></div><button type="button" disabled={busy} onClick={onClose} aria-label="关闭环境管理">×</button></div>
    <p>环境值覆盖模板默认值。桌面与 CLI 共用，CLI 使用 <code>--environment 环境名</code>。</p>
    <fieldset disabled={busy}><div className="environment-layout"><nav aria-label="环境列表">
      {data.profiles.map((item) => <button type="button" className={item.id === selected ? "active" : ""} key={item.id} onClick={() => { setSelected(item.id); setRemoving(false); }}>{item.name || "未命名环境"}<small>{item.variables.length} 个变量</small></button>)}
      <button type="button" disabled={data.profiles.length >= 30} onClick={create}>＋ 新建环境</button>
    </nav><div className="environment-editor">{profile ? <>
      <label className="field-label">环境名称<input autoFocus aria-label="环境名称" maxLength={80} placeholder="例如 local、staging" value={profile.name} onChange={(e) => update({ name: e.target.value })} /></label>
      <div className="variable-labels"><span>变量名</span><span>环境值</span><span /></div>
      {profile.variables.map((row, i) => <div className="variable-row" key={i}>
        <input aria-label={`环境变量 ${i + 1} 名称`} maxLength={100} value={row.key} onChange={(e) => update({ variables: profile.variables.map((v, index) => index === i ? { ...v, key: e.target.value } : v) })} />
        <input aria-label={`环境变量 ${i + 1} 值`} maxLength={16384} value={row.value} onChange={(e) => update({ variables: profile.variables.map((v, index) => index === i ? { ...v, value: e.target.value } : v) })} />
        <button type="button" className="icon-button" aria-label={`移除环境变量 ${i + 1}`} onClick={() => update({ variables: profile.variables.filter((_, index) => index !== i) })}>×</button>
      </div>)}
      <button type="button" className="text-button" disabled={profile.variables.length >= 100} onClick={() => update({ variables: [...profile.variables, { key: "", value: "" }] })}>＋ 添加环境变量</button>
      <div className="environment-delete">{removing ? <><span>删除「{profile.name || "未命名环境"}」？保存后生效。</span><button type="button" className="danger-button" onClick={() => { const profiles = data.profiles.filter((item) => item.id !== selected); setData({ ...data, profiles }); setSelected(profiles[0]?.id ?? ""); setRemoving(false); }}>确认删除环境</button><button type="button" onClick={() => setRemoving(false)}>取消</button></> : <button type="button" className="text-button danger" onClick={() => setRemoving(true)}>删除环境</button>}</div>
    </> : <div className="library-empty">添加 local、staging 等环境，<br />通过 base_url 切换服务地址。<br /><button type="button" onClick={create}>新建第一个环境</button></div>}</div></div>
    <p className="hint">环境与草稿以本机 JSON 明文保存；系统密钥库尚未接入。</p>
    {error && <div className="error-message" role="alert">{error}</div>}
    <div className="modal-actions"><button type="button" onClick={onClose}>取消</button><button className="save-button" type="submit">{busy ? "保存中…" : "保存环境"}</button></div></fieldset>
  </form></div>;
}
