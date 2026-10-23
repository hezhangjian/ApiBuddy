import { validateId } from "./templates";

export type ServiceInput = { name: string; slug: string; description: string; baseUrl: string };
export type Service = ServiceInput & { schemaVersion: 1; id: string };
export type ServiceCatalog = { services: Service[]; warnings: string[] };
export type AgentBundle = {
  cliPath: string; markdownPath: string; markdown: string; serviceId: string; setup: string;
  commands: { name: string; method: string; description: string; example: string }[]; warnings: string[];
};
export type CliConfig = { dataRoot: string };
export const DEFAULT_SERVICE: Service = {
  schemaVersion: 1, id: "default", name: "默认服务", slug: "default", description: "原有请求模板", baseUrl: "https://httpbin.org",
};
export function validateServiceId(value: unknown): string {
  return value === "default" ? value : validateId(value);
}
export function validateServiceInput(value: unknown): ServiceInput {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("服务信息无效");
  const input = value as Record<string, unknown>;
  const field = (key: string, max: number) => {
    if (typeof input[key] !== "string" || input[key].length > max) throw new Error(`服务 ${key} 格式错误或过长`);
    return input[key].trim();
  };
  const name = field("name", 120);
  const slug = field("slug", 48);
  const description = field("description", 2000);
  const baseUrl = field("baseUrl", 16384);
  if (!name || /[\r\n\0]/.test(name)) throw new Error("服务名称不能为空或包含换行");
  if (!/^[a-z][a-z0-9-]{0,47}$/.test(slug)) throw new Error("服务标识使用小写字母、数字和短横线，并以字母开头");
  if (baseUrl) {
    let url: URL;
    try { url = new URL(baseUrl); } catch { throw new Error("服务地址需为有效的 HTTP/HTTPS URL"); }
    if (!["http:", "https:"].includes(url.protocol) || url.username || url.password) throw new Error("服务地址仅支持不含用户名密码的 HTTP/HTTPS URL");
  }
  return { name, slug, description, baseUrl };
}
export function validateService(value: unknown): Service {
  const input = value as Partial<Service> | null;
  if (!input || input.schemaVersion !== 1) throw new Error("不支持的服务文件版本");
  return { ...validateServiceInput(input), schemaVersion: 1, id: validateServiceId(input.id) };
}
