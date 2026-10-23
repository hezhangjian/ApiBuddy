import type { Service } from "./services";
import { newTemplate, referencedVariables, validateTemplate, type Method, type TemplateInput } from "./templates";

export function templateForService(service: Service): TemplateInput {
  const template = newTemplate();
  if (service.id !== "default") {
    template.request.url = "{{base_url}}/";
    template.request.variables[0].value = service.baseUrl.replace(/\/+$/, "");
  }
  return template;
}

export function buildServiceTemplate(service: Service, input: { name: string; method: Method; endpoint: string }): TemplateInput {
  const endpoint = input.endpoint.trim();
  if (!endpoint || /[\r\n\0]/.test(endpoint)) throw new Error("请输入接口路径或完整 URL");
  const relative = endpoint.startsWith("/") && !endpoint.startsWith("//");
  if (relative && !service.baseUrl) throw new Error("当前服务未设置地址，请填写完整 HTTP/HTTPS URL");
  if (!relative && !/^https?:\/\//i.test(endpoint)) throw new Error("接口路径以 / 开头，完整 URL 以 http:// 或 https:// 开头");
  const template = templateForService(service);
  template.name = input.name;
  template.request.method = input.method;
  template.request.url = relative ? `{{base_url}}${endpoint}` : endpoint;
  template.request.variables = referencedVariables(template.request).map((key) => ({
    key, value: key === "base_url" ? service.baseUrl.replace(/\/+$/, "") : "",
  }));
  // Validate URL structure without requiring values for newly discovered variables.
  const preview = (relative ? service.baseUrl.replace(/\/+$/, "") + endpoint : endpoint)
    .replace(/\{\{\s*([A-Za-z_][A-Za-z0-9_.-]*)\s*\}\}/g, "value");
  let url: URL;
  try { url = new URL(preview); } catch { throw new Error("请输入有效的 HTTP/HTTPS URL"); }
  if (!["http:", "https:"].includes(url.protocol) || url.username || url.password) throw new Error("URL 仅支持 HTTP/HTTPS，认证信息请在创建后填写到请求头");
  return validateTemplate(template);
}
