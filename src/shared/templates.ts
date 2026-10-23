export const METHODS = ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"] as const;
export type Method = (typeof METHODS)[number];
export type Pair = { key: string; value: string; enabled: boolean };
export type Variable = { key: string; value: string };
export type RequestDraft = {
  method: Method; url: string; query: Pair[]; headers: Pair[]; body: string;
  variables: Variable[]; timeoutMs: number; stream: boolean;
};
export type TemplateInput = { name: string; description: string; request: RequestDraft };
export type SavedTemplate = TemplateInput & {
  schemaVersion: 1; id: string; revision: number; createdAt: string; updatedAt: string;
};
export type Library = { directory: string; templates: SavedTemplate[]; warnings: string[] };
export type ResolvedRequest = Omit<RequestDraft, "query" | "variables">;
export type Result<T> = { ok: true; value: T } | { ok: false; error: string };

export function newTemplate(): TemplateInput {
  return { name: "未命名请求", description: "", request: {
    method: "GET", url: "{{base_url}}/get", query: [],
    headers: [{ key: "Accept", value: "application/json", enabled: true }],
    body: "", variables: [{ key: "base_url", value: "https://httpbin.org" }], timeoutMs: 30000, stream: true,
  } };
}
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("数据必须是对象");
  return value as Record<string, unknown>;
}
function text(value: unknown, label: string, max = 16384): string {
  if (typeof value !== "string" || value.length > max) throw new Error(`${label}格式错误或过长`);
  return value;
}
function rows(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value) || value.length > 100) throw new Error(`${label}最多支持 100 项`);
  return value;
}
function pairs(value: unknown, label: string): Pair[] {
  return rows(value, label).map((entry) => {
    const row = object(entry);
    if (typeof row.enabled !== "boolean") throw new Error(`${label}启用状态错误`);
    return { key: text(row.key, label, 1024), value: text(row.value, label), enabled: row.enabled };
  });
}
export function validateDraft(value: unknown): RequestDraft {
  const draft = object(value);
  if (!METHODS.includes(draft.method as Method)) throw new Error("不支持的 HTTP 方法");
  if (!Number.isInteger(draft.timeoutMs) || Number(draft.timeoutMs) < 1000 || Number(draft.timeoutMs) > 600000) throw new Error("超时必须为 1–600 秒");
  if (typeof draft.stream !== "boolean") throw new Error("流式设置格式错误");
  const keys = new Set<string>();
  const variables = rows(draft.variables, "变量").map((entry) => {
    const row = object(entry);
    const key = text(row.key, "变量名", 100).trim();
    if (!/^[A-Za-z_][A-Za-z0-9_.-]*$/.test(key)) throw new Error("变量名需以字母或下划线开头，仅支持字母、数字、点、短横线和下划线");
    if (keys.has(key)) throw new Error(`变量 ${key} 重复`);
    keys.add(key);
    return { key, value: text(row.value, "变量值") };
  });
  return { method: draft.method as Method, url: text(draft.url, "URL"), query: pairs(draft.query, "查询参数"),
    headers: pairs(draft.headers, "请求头"), body: text(draft.body, "请求体", 1024 * 1024), variables,
    timeoutMs: Number(draft.timeoutMs), stream: draft.stream };
}
export function validateTemplate(value: unknown): TemplateInput {
  const input = object(value);
  const name = text(input.name, "模板名称", 120).trim();
  if (!name) throw new Error("请输入模板名称");
  return { name, description: text(input.description, "描述", 2000), request: validateDraft(input.request) };
}
export function validateId(value: unknown): string {
  if (typeof value !== "string" || !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)) throw new Error("模板或请求 ID 无效");
  return value;
}
export function validateSaved(value: unknown): SavedTemplate {
  const saved = object(value);
  if (saved.schemaVersion !== 1) throw new Error("不支持的模板文件版本");
  if (!Number.isSafeInteger(saved.revision) || Number(saved.revision) < 1) throw new Error("模板修订号无效");
  for (const key of ["createdAt", "updatedAt"]) {
    if (typeof saved[key] !== "string" || !Number.isFinite(Date.parse(saved[key]))) throw new Error("模板时间无效");
  }
  return { ...validateTemplate(saved), schemaVersion: 1, id: validateId(saved.id), revision: Number(saved.revision),
    createdAt: saved.createdAt as string, updatedAt: saved.updatedAt as string };
}
export function referencedVariables(draft: RequestDraft): string[] {
  const fields = [draft.url, ...draft.query.filter((x) => x.enabled).flatMap((x) => [x.key, x.value]),
    ...draft.headers.filter((x) => x.enabled).flatMap((x) => [x.key, x.value]),
    ...(["GET", "HEAD"].includes(draft.method) ? [] : [draft.body])];
  return [...new Set(fields.flatMap((field) => [...field.matchAll(/\{\{\s*([A-Za-z_][A-Za-z0-9_.-]*)\s*\}\}/g)].map((m) => m[1])))].sort();
}
export function resolveRequest(value: unknown, substitute = true): ResolvedRequest {
  const draft = validateDraft(value);
  const variables = new Map(draft.variables.map((v) => [v.key, v.value]));
  const missing = referencedVariables(draft).filter((key) => !variables.has(key));
  if (substitute && missing.length) throw new Error(`未定义变量：${missing.join("、")}`);
  const replace = (source: string) => substitute ? source.replace(/\{\{\s*([A-Za-z_][A-Za-z0-9_.-]*)\s*\}\}/g, (_, key) => variables.get(key)!) : source;
  let url: URL;
  try { url = new URL(replace(draft.url).trim()); } catch { throw new Error("请输入有效的绝对 URL"); }
  if (!["http:", "https:"].includes(url.protocol)) throw new Error("仅支持 HTTP 和 HTTPS");
  if (url.username || url.password) throw new Error("请通过请求头传递认证信息，URL 不支持用户名和密码");
  for (const row of draft.query.filter((q) => q.enabled && q.key.trim())) url.searchParams.append(replace(row.key), replace(row.value));
  const headers = draft.headers.filter((h) => h.enabled && h.key.trim()).map((row) => ({ key: replace(row.key).trim(), value: replace(row.value), enabled: true }));
  for (const row of headers) {
    if (!/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/.test(row.key) || /[\r\n\0]/.test(row.value)) throw new Error("请求头名称或值无效");
  }
  return { method: draft.method, url: url.href, headers, body: ["GET", "HEAD"].includes(draft.method) ? "" : replace(draft.body), timeoutMs: draft.timeoutMs, stream: draft.stream };
}
