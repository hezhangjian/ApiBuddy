import { memo, useLayoutEffect, useMemo, useRef } from "react";
import { jsonTokens, textSegments, type TextRange } from "../shared/response-text";

export const ResponseText = memo(function ResponseText({ text, json, wrap, matches, activeMatch, running, follow, onFollow }: {
  text: string; json: boolean; wrap: boolean; matches: TextRange[]; activeMatch: number;
  running: boolean; follow: boolean; onFollow: (follow: boolean) => void;
}) {
  const body = useRef<HTMLPreElement>(null);
  const active = useRef<HTMLElement | null>(null);
  const tokens = useMemo(() => json ? jsonTokens(text) : [], [text, json]);
  const segments = useMemo(() => textSegments(text, tokens, matches), [text, tokens, matches]);
  useLayoutEffect(() => {
    const pane = body.current!, target = active.current;
    if (!matches.length || !target) return;
    const viewport = pane.getBoundingClientRect(), found = target.getBoundingClientRect();
    pane.scrollTop += found.top - viewport.top - pane.clientHeight / 2 + found.height / 2;
    if (!wrap) pane.scrollLeft += found.left - viewport.left - pane.clientWidth / 2 + found.width / 2;
  }, [activeMatch, matches, wrap]);
  useLayoutEffect(() => {
    if (running && follow && !matches.length && body.current) body.current.scrollTop = body.current.scrollHeight;
  }, [text, running, follow, matches.length]);
  return <pre ref={body} className={`response-body ${wrap ? "" : "no-wrap"}`} tabIndex={0} aria-label="响应正文"
    onScroll={() => {
      const pane = body.current!;
      if (running && !matches.length) onFollow(pane.scrollHeight - pane.scrollTop - pane.clientHeight < 30);
    }}>
    {segments.map((segment) => {
      const value = text.slice(segment.start, segment.end);
      const className = segment.kind ? `syntax-${segment.kind}` : undefined;
      if (segment.match === undefined) return className ? <span key={segment.start} className={className}>{value}</span> : value;
      return <mark key={segment.start} className={segment.match === activeMatch ? "current-match" : ""}
        ref={segment.match === activeMatch && segment.start === matches[activeMatch]?.start ? (element) => { active.current = element; } : undefined}>{value}</mark>;
    })}
  </pre>;
});
