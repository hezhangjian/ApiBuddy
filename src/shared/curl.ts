import { METHODS, newTemplate, resolveRequest, validateDraft, type Method, type RequestDraft, type TemplateInput } from "./templates";
import { shellQuote } from "./cli-commands";

// Parse a single POSIX command as data. Nothing is evaluated or executed.
function tokenize(source: string): string[] {
  if (source.length > 1024 * 1024 || source.includes("\0")) throw new Error("cURL 内容过大或包含无效字符");
  const tokens: string[] = [];
  let word = "", quote = "", started = false;
  for (let i = 0; i < source.length; i++) {
    const char = source[i];
    if (quote === "'") { if (char === "'") quote = ""; else word += char; started = true; continue; }
    if (char === "\\") {
      const next = source[++i];
      if (next === undefined) throw new Error("cURL 末尾的反斜杠缺少内容");
      if (next === "\n") continue;
      if (next === "\r" && source[i + 1] === "\n") { i++; continue; }
      word += quote === '"' && !['"', "\\", "$", "`"].includes(next) ? "\\" + next : next;
      started = true; continue;
    }
    if (char === "$" || char === "`") throw new Error("请先把 shell 变量或命令替换成实际值；可用单引号保留字面文本");
    if (quote === '"') { if (char === '"') quote = ""; else word += char; started = true; continue; }
    if (char === "'" || char === '"') { quote = char; started = true; continue; }
    if (/[;|&<>]/.test(char)) throw new Error("仅支持单条 cURL，请给 URL 和参数加引号，不支持管道或重定向");
    if (/\s/.test(char)) { if (started) { tokens.push(word); word = ""; started = false; } }
    else { word += char; started = true; }
  }
  if (quote) throw new Error("cURL 引号未闭合");
  if (started) tokens.push(word);
  return tokens;
}

export function importCurl(source: string): { template: TemplateInput; warnings: string[] } {
  const tokens = tokenize(source.trim());
  if (!tokens.length || !/^(?:\/usr\/bin\/)?curl$/.test(tokens.shift()!)) throw new Error("请粘贴以 curl 开头的命令（POSIX / bash 格式）");
  const template = newTemplate();
  const draft = template.request;
  draft.headers = []; draft.variables = []; draft.url = "";
  let method: Method | undefined, head = false, get = false, json = false, hasData = false;
  const data: string[] = [], warnings: string[] = [];
  const aliases: Record<string, string> = { "-X": "--request", "-H": "--header", "-d": "--data", "-b": "--cookie", "-A": "--user-agent", "-e": "--referer", "-m": "--max-time" };
  for (let i = 0; i < tokens.length; i++) {
    let flag = tokens[i], inline: string | undefined;
    if (flag.startsWith("--") && flag.includes("=")) { const split = flag.indexOf("="); inline = flag.slice(split + 1); flag = flag.slice(0, split); }
    else if (flag.length > 2 && aliases[flag.slice(0, 2)]) { inline = flag.slice(2); flag = flag.slice(0, 2); }
    flag = aliases[flag] ?? flag;
    const argument = () => { const value = inline ?? tokens[++i]; if (value === undefined) throw new Error(`${flag} 缺少值`); return value; };
    if (flag === "--request") { const value = argument().toUpperCase(); if (!METHODS.includes(value as Method)) throw new Error(`不支持 HTTP 方法 ${value}`); method = value as Method; }
    else if (flag === "--url" || !flag.startsWith("-")) {
      if (draft.url) throw new Error("每次只支持导入一个 URL");
      draft.url = flag === "--url" ? argument() : flag;
    } else if (flag === "--header") {
      const value = argument(), split = value.indexOf(":");
      if (split < 1 || !value.slice(split + 1).trim()) throw new Error("请求头需使用 '名称: 值'，暂不支持移除默认请求头");
      draft.headers.push({ key: value.slice(0, split).trim(), value: value.slice(split + 1).trim(), enabled: true });
    } else if (["--data", "--data-raw", "--data-binary", "--json"].includes(flag)) {
      const value = argument();
      if (flag !== "--data-raw" && value.startsWith("@")) throw new Error("不读取 @文件，请粘贴请求体内容后导入");
      if (flag === "--json" && hasData && !json || flag !== "--json" && json) throw new Error("请勿混合 --json 和 --data 参数");
      json ||= flag === "--json"; hasData = true; data.push(value);
    } else if (["--cookie", "--user-agent", "--referer"].includes(flag)) {
      const value = argument();
      if (flag === "--cookie" && !value.includes("=")) throw new Error("只支持 Cookie 文本，不读取 Cookie 文件");
      draft.headers.push({ key: flag === "--cookie" ? "Cookie" : flag === "--user-agent" ? "User-Agent" : "Referer", value, enabled: true });
    } else if (flag === "--max-time") {
      const seconds = Number(argument());
      if (!Number.isFinite(seconds) || seconds < 1 || seconds > 600) throw new Error("超时支持 1–600 秒");
      draft.timeoutMs = Math.round(seconds * 1000);
    } else if (["-I", "--head"].includes(flag)) head = true;
    else if (["-G", "--get"].includes(flag)) get = true;
    else if (["-L", "--location", "--compressed", "-s", "--silent", "-S", "--show-error", "-i", "--include", "-g", "--globoff"].includes(flag) || /^-[sSiLg]+$/.test(flag)) {
      if (["-L", "--location"].includes(flag)) continue;
    } else throw new Error(`暂不支持 ${flag}；请移除该选项后导入，或手动配置请求`);
  }
  if (!draft.url) throw new Error("cURL 缺少 URL");
  draft.method = method ?? (head ? "HEAD" : get ? "GET" : hasData ? "POST" : "GET");
  draft.body = data.join(json ? "" : "&");
  if (get && hasData) {
    const url = new URL(draft.url);
    url.search = [url.search.slice(1), draft.body].filter(Boolean).join("&");
    draft.url = url.href; draft.body = "";
  }
  if (["GET", "HEAD"].includes(draft.method) && draft.body) throw new Error("该请求携带 GET/HEAD 请求体，当前尚不支持");
  const addHeader = (key: string, value: string) => { if (!draft.headers.some((row) => row.key.toLowerCase() === key.toLowerCase())) draft.headers.push({ key, value, enabled: true }); };
  if (hasData && !get) addHeader("Content-Type", json ? "application/json" : "application/x-www-form-urlencoded");
  if (json) addHeader("Accept", "application/json");
  // cURL is literal input; template variables can be configured after import.
  resolveRequest(draft, false);
  if (/\{\{/.test(JSON.stringify(draft))) warnings.push("导入内容含 {{变量}}，请在变量页填写对应值后发送。");
  warnings.push("请求沿用客户端的自动重定向行为；不导入浏览器 Cookie 存储或 shell 环境。");
  const url = new URL(draft.url);
  template.name = `${draft.method.toLowerCase()}-${url.pathname.split("/").filter(Boolean).pop() || url.hostname}`.slice(0, 120);
  return { template: { ...template, request: validateDraft(draft) }, warnings };
}

export function exportCurl(draft: RequestDraft): string {
  const request = resolveRequest(draft);
  const parts = ["curl --globoff --location", request.method === "HEAD" ? "--head" : `--request ${request.method}`, `--url ${shellQuote(request.url)}`, `--max-time ${request.timeoutMs / 1000}`];
  for (const row of request.headers) parts.push(`--header ${shellQuote(`${row.key}: ${row.value}`)}`);
  if (!["GET", "HEAD"].includes(request.method)) {
    // Fetch sends string bodies as UTF-8 text unless a content type is specified.
    if (!request.headers.some((row) => row.key.toLowerCase() === "content-type")) parts.push("--header 'Content-Type: text/plain;charset=UTF-8'");
    parts.push(`--data-raw ${shellQuote(request.body)}`);
  }
  return parts.join(" \\\n  ");
}
