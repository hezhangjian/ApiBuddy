import type { Pair } from "../shared/templates";
export function PairEditor({ label, rows, onChange }: { label: string; rows: Pair[]; onChange: (rows: Pair[]) => void }) {
  const update = (index: number, patch: Partial<Pair>) => onChange(rows.map((row, i) => i === index ? { ...row, ...patch } : row));
  return <div className="pairs">
    <div className="pair-labels"><span /><span>名称</span><span>值</span><span /></div>
    {rows.map((row, index) => <div className="pair-row" key={index}>
      <input type="checkbox" aria-label={`${label} ${index + 1} 启用`} checked={row.enabled} onChange={(e) => update(index, { enabled: e.target.checked })} />
      <input aria-label={`${label} ${index + 1} 名称`} placeholder="Key" value={row.key} onChange={(e) => update(index, { key: e.target.value })} />
      <input aria-label={`${label} ${index + 1} 值`} placeholder="Value 或 {{变量}}" value={row.value} onChange={(e) => update(index, { value: e.target.value })} />
      <button className="icon-button" title="移除此行" aria-label={`移除${label} ${index + 1}`} onClick={() => onChange(rows.filter((_, i) => i !== index))}>×</button>
    </div>)}
    <button className="text-button add-row" onClick={() => onChange([...rows, { key: "", value: "", enabled: true }])}>＋ 添加{label}</button>
  </div>;
}
