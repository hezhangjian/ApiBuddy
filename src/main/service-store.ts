import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, rmdirSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { join } from "node:path";
import { DEFAULT_SERVICE, validateService, validateServiceId, validateServiceInput, type Service, type ServiceCatalog } from "../shared/services";
import { dataDirectory, TemplateStore } from "./template-store";
import { atomicWrite } from "./files";

export class ServiceStore {
  constructor(readonly root = dataDirectory()) {}
  directory(id: string) { return validateServiceId(id) === "default" ? this.root : join(this.root, "services", id); }
  get(id: string): Service {
    validateServiceId(id);
    if (id === "default") return { ...DEFAULT_SERVICE };
    const directory = this.directory(id);
    if (!lstatSync(join(this.root, "services")).isDirectory()) throw new Error("服务库目录不能是符号链接或普通文件");
    if (!lstatSync(directory).isDirectory()) throw new Error("服务目录无效");
    const file = join(directory, "service.json");
    const stat = lstatSync(file);
    if (!stat.isFile() || stat.size > 65536) throw new Error("服务文件格式错误或过大");
    const service = validateService(JSON.parse(readFileSync(file, "utf8")));
    if (service.id !== id) throw new Error("服务 ID 与目录不一致");
    return service;
  }
  list(): ServiceCatalog {
    const services = [{ ...DEFAULT_SERVICE }];
    const warnings: string[] = [];
    const directory = join(this.root, "services");
    if (existsSync(directory)) {
      if (!lstatSync(directory).isDirectory()) throw new Error("服务库目录不能是符号链接或普通文件");
      for (const id of readdirSync(directory).sort()) {
        if (id.startsWith(".")) continue;
        try {
          if (id === "default") throw new Error("默认服务使用根目录");
          services.push(this.get(id));
        } catch { warnings.push(`无法读取服务 ${id}，原文件已保留。`); }
      }
    }
    services.sort((a, b) => a.id === "default" ? -1 : b.id === "default" ? 1 : a.name.localeCompare(b.name));
    return { services, warnings };
  }
  create(value: unknown): Service {
    const input = validateServiceInput(value);
    const parent = join(this.root, "services");
    mkdirSync(parent, { recursive: true, mode: 0o700 });
    if (!lstatSync(parent).isDirectory()) throw new Error("服务库目录不能是符号链接或普通文件");
    const lock = join(parent, ".write-lock");
    try { mkdirSync(lock, { mode: 0o700 }); } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "EEXIST") throw new Error("服务目录正在写入，请稍后重试");
      throw error;
    }
    try {
      if (this.list().services.some((service) => service.slug === input.slug)) throw new Error("服务标识已存在，请使用不同标识");
      const service: Service = { ...input, id: randomUUID(), schemaVersion: 1 };
      const directory = this.directory(service.id);
      mkdirSync(directory, { mode: 0o700 });
      atomicWrite(join(directory, "service.json"), JSON.stringify(service, null, 2) + "\n");
      return service;
    } finally { rmdirSync(lock); }
  }
  templates(serviceId: string): TemplateStore {
    this.get(serviceId);
    return new TemplateStore(this.directory(serviceId));
  }
}
