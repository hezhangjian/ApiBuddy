export function jsonPointer(value: unknown, pointer: string): unknown {
  if (!pointer) return value;
  if (!pointer.startsWith("/")) throw new Error("字段路径使用 JSON Pointer，例如 /data/items/0/id");
  let current = value;
  for (const part of pointer.slice(1).split("/")) {
    if (/~(?:[^01]|$)/.test(part)) throw new Error("路径中的 ~ 使用 ~0，/ 使用 ~1 转义");
    const key = part.replace(/~1/g, "/").replace(/~0/g, "~");
    if (Array.isArray(current) && !/^(0|[1-9][0-9]*)$/.test(key)) throw new Error(`数组下标无效：${key}`);
    if (current === null || typeof current !== "object" || !Object.prototype.hasOwnProperty.call(current, key)) throw new Error(`找不到字段：${key}`);
    current = (current as Record<string, unknown>)[key];
  }
  return current;
}
export const pointerKey = (key: string) => key.replace(/~/g, "~0").replace(/\//g, "~1");
