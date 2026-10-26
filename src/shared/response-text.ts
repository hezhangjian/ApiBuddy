export type TextRange = { start: number; end: number };
export type JsonToken = TextRange & { kind: "key" | "string" | "number" | "boolean" | "null" };
export const SEARCH_LIMIT = 2000;

// Match against the original string: lowercasing can change Unicode offsets.
export function findText(text: string, query: string): { ranges: TextRange[]; truncated: boolean } {
  if (!query) return { ranges: [], truncated: false };
  const pattern = new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "giu");
  const ranges: TextRange[] = [];
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(text))) {
    if (ranges.length === SEARCH_LIMIT) return { ranges, truncated: true };
    ranges.push({ start: match.index, end: match.index + match[0].length });
  }
  return { ranges, truncated: false };
}

export function nextMatch(current: number, count: number, step: number): number {
  return count ? ((current + step) % count + count) % count : 0;
}

// Large payloads stay plain text to bound DOM nodes and keep copying lossless.
export function jsonTokens(text: string): JsonToken[] {
  if (text.length > 256 * 1024) return [];
  const pattern = /"(?:[^"\\]|\\.)*"|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|\b(?:true|false|null)\b/g;
  const tokens: JsonToken[] = [];
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(text))) {
    if (tokens.length >= 8000) return [];
    const value = match[0], end = match.index + value.length;
    const kind = value.startsWith('"') ? (/^\s*:/.test(text.slice(end, end + 32)) ? "key" : "string")
      : value === "null" ? "null" : value === "true" || value === "false" ? "boolean" : "number";
    tokens.push({ start: match.index, end, kind });
  }
  return tokens;
}

export type TextSegment = TextRange & { kind?: JsonToken["kind"]; match?: number };
export function textSegments(text: string, tokens: JsonToken[], matches: TextRange[]): TextSegment[] {
  const boundaries = new Set([0, text.length]);
  for (const range of [...tokens, ...matches]) { boundaries.add(range.start); boundaries.add(range.end); }
  const positions = [...boundaries].sort((a, b) => a - b);
  let token = 0, match = 0;
  return positions.slice(0, -1).map((start, index) => {
    const end = positions[index + 1];
    while (tokens[token]?.end <= start) token++;
    while (matches[match]?.end <= start) match++;
    return { start, end,
      kind: tokens[token]?.start <= start ? tokens[token].kind : undefined,
      match: matches[match]?.start <= start ? match : undefined,
    };
  });
}
