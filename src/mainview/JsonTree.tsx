import { useState } from "react";
import { pointerKey } from "../shared/json-view";

export function JsonTree({ value, path = "", name = "$", depth = 0, onPath }: { value: unknown; path?: string; name?: string; depth?: number; onPath: (path: string) => void }) {
  const [open, setOpen] = useState(depth === 0), [limit, setLimit] = useState(100);
  const container = value !== null && typeof value === "object";
  const keys = container ? Object.keys(value) : [];
  const primitive = container ? "" : JSON.stringify(value);
  return <div className="json-node">
    <div className="json-line"><button className="json-toggle" aria-label={`${open ? "折叠" : "展开"} ${path || "/"}`} disabled={!container || !keys.length || depth >= 40} onClick={() => setOpen(!open)}>{container && keys.length ? open ? "▾" : "▸" : "·"}</button><button className="json-key" title={`查看字段 ${path || "根节点"}`} onClick={() => onPath(path)}>{name}</button><span className="json-colon">:</span>{container ? <span className="json-summary">{Array.isArray(value) ? `[ ${keys.length} 项 ]` : `{ ${keys.length} 个字段 }`}</span> : <span className={`json-value json-${value === null ? "null" : typeof value}`}>{primitive}</span>}</div>
    {container && open && depth < 40 && <div className="json-children">{keys.slice(0, limit).map((key) => <JsonTree key={key} name={key} path={`${path}/${pointerKey(key)}`} value={(value as Record<string, unknown>)[key]} depth={depth + 1} onPath={onPath} />)}{keys.length > limit && <button className="text-button" onClick={() => setLimit(limit + 100)}>再显示 {Math.min(100, keys.length - limit)} 项（剩余 {keys.length - limit} 项）</button>}</div>}
  </div>;
}
