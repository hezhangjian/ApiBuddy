import { lstatSync, readFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { commandExample, commandVariables, reservedOptions, serviceCommands } from "../shared/cli-commands";
import type { CliConfig, Service, ServiceCatalog } from "../shared/services";
import { METHODS, newTemplate, referencedVariables, resolveRequest, type RequestDraft, type SavedTemplate, type Variable } from "../shared/templates";
import type { Metadata, RequestEvent } from "../shared/rpc-schema";
import { HttpClient } from "../main/http-client";
import { ServiceStore } from "../main/service-store";
import { dataDirectory } from "../main/template-store";

declare const APIBUDDY_CLI_CONFIG: CliConfig;
function output(value: unknown) { process.stdout.write(JSON.stringify(value) + "\n"); }
function summary(template: SavedTemplate) {
  return { id: template.id, name: template.name, description: template.description, method: template.request.method,
    variables: commandVariables(template), revision: template.revision };
}
function printServices(catalog: ServiceCatalog) {
  const lines = ["ApiBuddy · apibuddy <服务> <接口> [--参数 值]", "", "服务："];
  for (const service of catalog.services) lines.push(`  apibuddy ${service.slug}    ${service.name}`);
  lines.push("", "apibuddy <服务> --help  查看该服务全部接口和参数。", "APIBUDDY_DIR 自定义存储目录。新增服务和接口保存后即可调用。", ...catalog.warnings);
  process.stdout.write(lines.join("\n") + "\n");
}
function printHelp(service: Service, templates: SavedTemplate[], warnings: string[]) {
  const executable = `apibuddy ${service.slug}`;
  const commands = serviceCommands(templates);
  const lines = [`${service.name} · ${executable}`, "", `全部已保存接口（${commands.length}）：`];
  for (const command of commands) {
    lines.push(`  ${commandExample(executable, command)}`, `    ${command.template.request.method}  ${command.template.description || command.template.name}`);
  }
  if (!commands.length) lines.push("  尚未保存接口；可先使用下面的 request 命令。");
  lines.push("", "任意接口：", `  ${executable} request GET /path`, `  ${executable} request POST /path --data '{"name":"example"}' --header 'Content-Type: application/json'`,
    "", "<参数名> 换成实际值；默认值可省略。接口命令加 --help 查看参数，--dry-run 仅预览。",
    "APIBUDDY_DIR 自定义存储目录。返回 JSON；需要 Node.js 18+。");
  if (warnings.length) lines.push("", ...warnings);
  process.stdout.write(lines.join("\n") + "\n");
}
function printEndpointHelp(service: Service, name: string, template: SavedTemplate) {
  const lines = [commandExample(`apibuddy ${service.slug}`, { name, template }), template.description || template.name, "", "参数："];
  for (const variable of commandVariables(template)) {
    const flag = reservedOptions.has(variable.name) ? `--var ${variable.name}=<值>` : `--${variable.name} <值>`;
    lines.push(`  ${flag}  ${variable.hasDefault ? "可选，使用已保存默认值" : "无默认值"}`);
  }
  lines.push("  --env 变量名=ENV_NAME  从环境变量读取凭据", "  --dry-run             预览，不发送请求");
  process.stdout.write(lines.join("\n") + "\n");
}
function keyValue(value: string): [string, string] {
  const separator = value.indexOf("=");
  if (separator < 1) throw new Error("选项需使用 key=value 格式");
  return [value.slice(0, separator), value.slice(separator + 1)];
}
function options(args: string[], template: SavedTemplate): { variables: Variable[]; dryRun: boolean } {
  const values = new Map(template.request.variables.map((v) => [v.key, v.value]));
  const allowed = new Set([...values.keys(), ...referencedVariables(template.request)]);
  const set = (key: string, value: unknown) => {
    if (!allowed.has(key)) throw new Error(`模板没有变量 ${key}`);
    if (typeof value !== "string") throw new Error(`变量 ${key} 必须是字符串`);
    values.set(key, value);
  };
  let dryRun = false;
  for (let index = 0; index < args.length; index++) {
    const flag = args[index];
    if (flag === "--dry-run") { dryRun = true; continue; }
    if (!["--var", "--env", "--vars-file"].includes(flag)) {
      if (!flag.startsWith("--")) throw new Error(`未知选项 ${flag}`);
      const separator = flag.indexOf("=");
      const key = flag.slice(2, separator < 0 ? undefined : separator);
      const value = separator < 0 ? args[++index] : flag.slice(separator + 1);
      if (value === undefined || (separator < 0 && value.startsWith("--"))) throw new Error(`${flag} 缺少值；以 -- 开头的值请使用 --${key}=值`);
      set(key, value); continue;
    }
    const value = args[++index];
    if (value === undefined) throw new Error(`${flag} 缺少值`);
    if (flag === "--vars-file") {
      const stat = lstatSync(value);
      if (!stat.isFile() || stat.size > 65536) throw new Error("变量文件需为不超过 64 KiB 的普通 JSON 文件");
      const data: unknown = JSON.parse(readFileSync(value, "utf8"));
      if (!data || typeof data !== "object" || Array.isArray(data)) throw new Error("变量文件需为字符串键值对象");
      for (const [key, entry] of Object.entries(data)) set(key, entry);
    } else {
      const [key, entry] = keyValue(value);
      if (flag === "--env") {
        if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(entry)) throw new Error("环境变量名称无效");
        if (process.env[entry] === undefined) throw new Error(`环境变量 ${entry} 未设置`);
        set(key, process.env[entry]);
      } else set(key, entry);
    }
  }
  return { variables: [...values].map(([key, value]) => ({ key, value })), dryRun };
}
function rawRequest(service: Service, args: string[]): { draft: RequestDraft; dryRun: boolean } {
  const [method, path, ...flags] = args;
  if (!METHODS.includes(method as RequestDraft["method"])) throw new Error("用法：request GET /path；方法支持 " + METHODS.join(", "));
  if (!service.baseUrl) throw new Error("此服务尚未设置默认地址，请使用已保存的接口命令");
  if (!path || !path.startsWith("/") || path.startsWith("//") || path.includes("\\")) throw new Error("接口路径需以 / 开头，例如 /orders");
  const base = new URL(service.baseUrl);
  // Treat the service base pathname as an API prefix, e.g. /v1 + /orders.
  const url = new URL(base.origin + base.pathname.replace(/\/$/, "") + path);
  if (url.origin !== base.origin) throw new Error("接口路径不能更换服务地址");
  const draft = newTemplate().request;
  draft.method = method as RequestDraft["method"]; draft.url = url.href; draft.variables = [];
  let dryRun = false;
  for (let index = 0; index < flags.length; index++) {
    const flag = flags[index];
    if (flag === "--dry-run") { dryRun = true; continue; }
    if (!["--data", "--header", "--query"].includes(flag)) throw new Error(`未知选项 ${flag}`);
    const value = flags[++index]; if (value === undefined) throw new Error(`${flag} 缺少值`);
    if (flag === "--data") draft.body = value;
    if (flag === "--query") { const [key, entry] = keyValue(value); draft.query.push({ key, value: entry, enabled: true }); }
    if (flag === "--header") {
      const separator = value.indexOf(":"); if (separator < 1) throw new Error("请求头使用 '名称: 值' 格式");
      draft.headers.push({ key: value.slice(0, separator).trim(), value: value.slice(separator + 1).trim(), enabled: true });
    }
  }
  if (["GET", "HEAD"].includes(method) && draft.body) throw new Error(`${method} 不支持 --data 请求体`);
  return { draft, dryRun };
}
async function execute(service: Service, draft: RequestDraft, dryRun: boolean, templateId?: string) {
  const resolved = resolveRequest(draft, templateId !== undefined);
  if (dryRun) { output({ ok: true, dryRun: true, serviceId: service.id, templateId, request: resolved }); return; }
  let body = "";
  let meta: Metadata | undefined;
  let finish!: (event: Extract<RequestEvent, { type: "end" }>) => void;
  const completed = new Promise<Extract<RequestEvent, { type: "end" }>>((resolve) => { finish = resolve; });
  const client = new HttpClient((event) => {
    if (event.type === "headers") meta = event.meta;
    if (event.type === "chunk") body += event.text;
    if (event.type === "end") finish(event);
  });
  const requestId = randomUUID();
  const cancel = () => client.cancel(requestId);
  process.on("SIGINT", cancel);
  process.on("SIGTERM", cancel);
  try {
    client.start(requestId, draft, templateId !== undefined);
    const end = await completed;
    const ok = end.phase === "completed" && (meta?.status ?? 500) < 400;
    output({ ok, serviceId: service.id, templateId, phase: end.phase, status: meta?.status ?? null,
      headers: meta?.headers ?? [], body, elapsedMs: end.elapsedMs, bytes: end.bytes, ...(end.error ? { error: end.error } : {}) });
    process.exitCode = end.phase === "cancelled" ? 130 : end.phase !== "completed" ? 3 : ok ? 0 : 4;
  } finally { client.dispose(); process.off("SIGINT", cancel); process.off("SIGTERM", cancel); }
}
async function main() {
  if (Number(process.versions.node.split(".")[0]) < 18) throw new Error("CLI 需要 Node.js 18 或更新版本");
  const [serviceSlug, command = "--help", ...args] = process.argv.slice(2);
  const configured = ["APIBUDDY_DIR", "APIABC_DIR", "APIX_DIR"].some((name) => process.env[name] !== undefined);
  const root = configured ? dataDirectory() : APIBUDDY_CLI_CONFIG.dataRoot;
  const services = new ServiceStore(root);
  const catalog = services.list();
  if (serviceSlug === undefined || ["--help", "-h"].includes(serviceSlug)) { printServices(catalog); return; }
  const matches = catalog.services.filter((service) => service.slug === serviceSlug);
  if (!matches.length) throw new Error(`没有服务 ${serviceSlug}，执行 apibuddy --help 查看全部服务`);
  if (matches.length > 1) throw new Error(`服务标识 ${serviceSlug} 重复，请检查服务配置`);
  const service = matches[0];
  const library = services.templates(service.id).list();
  const commands = serviceCommands(library.templates);
  if (["help", "--help", "-h"].includes(command)) { printHelp(service, library.templates, library.warnings); return; }
  if (command === "list") {
    if (args.length) throw new Error("list 不接受额外参数");
    output({ ok: true, service: { id: service.id, name: service.name, slug: service.slug }, templates: commands.map((entry) => ({ ...summary(entry.template), command: entry.name })), warnings: library.warnings }); return;
  }
  if (command === "request") {
    if (args.length === 1 && ["--help", "-h"].includes(args[0])) { printHelp(service, library.templates, library.warnings); return; }
    const parsed = rawRequest(service, args); await execute(service, parsed.draft, parsed.dryRun); return;
  }
  // Keep older integrations working while the primary interface uses named commands.
  if (command === "run" || command === "show") {
    const [selector, ...flags] = args;
    if (!selector) throw new Error(`${command} 需要接口名称或 ID`);
    const exact = library.templates.find((template) => template.id === selector);
    const matches = exact ? [exact] : library.templates.filter((template) => template.name === selector);
    if (!matches.length) throw new Error("当前服务内找不到此接口，请执行 --help");
    if (matches.length > 1) throw new Error("接口名称重复，请使用 --help 列出的独立命令");
    const template = matches[0];
    if (command === "show") {
      if (flags.length) throw new Error("show 不接受额外参数");
      output({ ok: true, serviceId: service.id, template: summary(template), timeoutMs: template.request.timeoutMs }); return;
    }
    const parsed = options(flags, template); await execute(service, { ...template.request, variables: parsed.variables }, parsed.dryRun, template.id); return;
  }
  const entry = commands.find((entry) => entry.name === command);
  if (!entry) throw new Error(`没有接口 ${command}，执行 apibuddy ${service.slug} --help 查看全部接口`);
  if (args.length === 1 && ["--help", "-h"].includes(args[0])) { printEndpointHelp(service, entry.name, entry.template); return; }
  const parsed = options(args, entry.template);
  await execute(service, { ...entry.template.request, variables: parsed.variables }, parsed.dryRun, entry.template.id);
}
void main().catch((error: unknown) => {
  output({ ok: false, error: error instanceof Error ? error.message : "CLI 执行失败" }); process.exitCode = 2;
});
