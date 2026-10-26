import { useEffect, useRef, useState, type ReactNode } from "react";

type Layout = { sidebar: boolean; stacked: boolean; horizontal: number; vertical: number };
const defaults: Layout = { sidebar: true, stacked: false, horizontal: 45, vertical: 45 };
const clamp = (value: number) => Math.max(25, Math.min(75, value));
export function useLayout() {
  const [layout, setLayout] = useState<Layout>(() => {
    try {
      const saved = JSON.parse(localStorage.getItem("apibuddy.layout.v1") || "null");
      if (!saved || typeof saved !== "object") return defaults;
      return {
        sidebar: typeof saved.sidebar === "boolean" ? saved.sidebar : defaults.sidebar,
        stacked: typeof saved.stacked === "boolean" ? saved.stacked : defaults.stacked,
        horizontal: Number.isFinite(saved.horizontal) ? clamp(saved.horizontal) : defaults.horizontal,
        vertical: Number.isFinite(saved.vertical) ? clamp(saved.vertical) : defaults.vertical,
      };
    } catch { return defaults; }
  });
  useEffect(() => {
    try { localStorage.setItem("apibuddy.layout.v1", JSON.stringify(layout)); } catch { /* Layout remains usable without storage. */ }
  }, [layout]);
  return [layout, setLayout] as const;
}

export function SplitPane({ layout, onResize, onNarrowChange, focus, request, response }: {
  layout: Layout;
  focus?: "request" | "response";
  onResize: (direction: "horizontal" | "vertical", ratio: number) => void;
  onNarrowChange: (narrow: boolean) => void;
  request: ReactNode;
  response: ReactNode;
}) {
  const container = useRef<HTMLDivElement>(null);
  const [narrow, setNarrow] = useState(false);
  const [dragging, setDragging] = useState(false);
  useEffect(() => {
    const observer = new ResizeObserver(([entry]) => {
      const next = entry.contentRect.width < 660;
      setNarrow(next); onNarrowChange(next);
    });
    observer.observe(container.current!);
    return () => observer.disconnect();
  }, [onNarrowChange]);
  const stacked = layout.stacked || narrow;
  const direction = stacked ? "vertical" : "horizontal";
  const ratio = layout[direction];
  return <div ref={container} className={`work-panels ${stacked ? "stacked" : "side-by-side"} ${dragging ? "resizing" : ""} ${focus ? `focus-${focus}` : ""}`}
    style={focus ? { gridTemplateRows: "minmax(0, 1fr)", gridTemplateColumns: "minmax(0, 1fr)" }
      : stacked ? { gridTemplateRows: `minmax(0, ${ratio}fr) 7px minmax(0, ${100 - ratio}fr)` } : { gridTemplateColumns: `minmax(0, ${ratio}fr) 7px minmax(0, ${100 - ratio}fr)` }}>
    {request}
    <div className="panel-divider" role="separator" tabIndex={0} aria-label="调整请求与响应区域" aria-orientation={stacked ? "horizontal" : "vertical"}
      aria-valuemin={25} aria-valuemax={75} aria-valuenow={Math.round(ratio)} aria-valuetext={`请求 ${Math.round(ratio)}%，响应 ${Math.round(100 - ratio)}%`}
      title="拖动调整区域；方向键微调，双击恢复均分"
      onDoubleClick={() => onResize(direction, 50)}
      onKeyDown={(event) => {
        const lower = stacked ? "ArrowUp" : "ArrowLeft", higher = stacked ? "ArrowDown" : "ArrowRight";
        if (![lower, higher, "Home", "End", "Enter"].includes(event.key)) return;
        event.preventDefault();
        onResize(direction, event.key === "Home" ? 25 : event.key === "End" ? 75 : event.key === "Enter" ? 50 : clamp(ratio + (event.key === higher ? 2 : -2)));
      }}
      onPointerDown={(event) => { if (event.button !== 0) return; event.preventDefault(); event.currentTarget.focus(); event.currentTarget.setPointerCapture(event.pointerId); setDragging(true); }}
      onPointerMove={(event) => {
        if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
        const rect = container.current!.getBoundingClientRect();
        const size = (stacked ? rect.height : rect.width) - 7;
        if (size > 0) onResize(direction, clamp(((stacked ? event.clientY - rect.top : event.clientX - rect.left) - 3.5) / size * 100));
      }}
      onPointerUp={(event) => { if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId); setDragging(false); }}
      onLostPointerCapture={() => setDragging(false)} onPointerCancel={() => setDragging(false)} />
    {response}
  </div>;
}
