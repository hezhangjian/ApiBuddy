import { useEffect, useState } from "react";
import type { AgentBundle, Service } from "../shared/services";
import { errorText, rpc, unwrap } from "./rpc";

export function AgentPanel({ service, dirty, onClose }: { service: Service; dirty: boolean; onClose: () => void }) {
  const [bundle, setBundle] = useState<AgentBundle>();
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const executable = `apibuddy ${service.slug}`;
  useEffect(() => {
    let mounted = true;
    void rpc.request["generate-cli"]({ serviceId: service.id }).then((result) => {
      const next = unwrap(result); if (mounted) setBundle(next);
    }).catch((caught) => { if (mounted) setError(errorText(caught)); });
    return () => { mounted = false; };
  }, [service.id]);
  const copy = async () => {
    if (!bundle) return;
    try { unwrap(await rpc.request["clipboard-write"]({ text: bundle.markdown })); setCopied(true); }
    catch (caught) { setError(errorText(caught)); }
  };
  return <div className="modal-backdrop"><section className="modal agent-modal" role="dialog" aria-modal="true" aria-label="服务 CLI">
    <div className="agent-heading"><div><span className="eyebrow">统一入口 apibuddy · 按服务调用</span><h2>{service.name} → {executable}</h2></div><button autoFocus aria-label="关闭 Agent 接入" onClick={onClose}>×</button></div>
    <p>apibuddy --help 查看全部服务，{executable} --help 查看当前服务接口。给 AI 用：复制说明，粘贴到 Agent 会读取的 Markdown。</p>
    {dirty && <div className="notice">当前修改还没保存，CLI 使用已保存的接口。保存后立即生效。</div>}
    {error && <div className="error-message" role="alert">{error}</div>}
    {bundle ? <>
      {bundle.warnings.length > 0 && <div className="notice">{bundle.warnings.join(" ")}</div>}
      <div className="cli-setup"><strong>终端里先运行一次</strong><pre>{bundle.setup}</pre></div>
      <div className="cli-endpoints-heading"><strong>这个服务的接口 · {bundle.commands.length}</strong><code>{executable} --help</code></div>
      <div className="cli-endpoints">
        {bundle.commands.map((command) => <div className="cli-endpoint" key={command.name}><div><span className={`method method-${command.method.toLowerCase()}`}>{command.method}</span><span>{command.description}</span></div><code>{command.example}</code></div>)}
        {!bundle.commands.length && <p>还没有保存接口。保存请求后，这里会自动生成对应命令。</p>}
      </div>
      <p className="cli-caption">把 &lt;参数名&gt; 换成实际值；已有默认值的参数可以省略。需要 Node.js 18+。</p>
      <div className="cli-raw"><strong>没存模板的接口也能调用</strong><code>{executable} request GET /path</code><span>将 /path 换成服务地址后的接口路径。</span></div>
      <div className="modal-actions"><span className="hint" role="status">{copied ? "已复制，粘贴给你的 Agent 即可" : "新增或修改接口，保存后直接调用"}</span><button className="save-button" onClick={() => void copy()}>{copied ? "✓ 已复制" : "复制给 AI Agent"}</button></div>
      <details className="agent-details"><summary>查看 Markdown 说明与文件位置</summary><textarea aria-label="Agent Markdown 说明" className="agent-markdown" readOnly spellCheck={false} value={bundle.markdown} /><div className="agent-paths"><span>CLI 文件</span><code>{bundle.cliPath}</code><span>说明文件</span><code>{bundle.markdownPath}</code></div></details>
    </> : !error && <p role="status">正在生成服务 CLI…</p>}
  </section></div>;
}
