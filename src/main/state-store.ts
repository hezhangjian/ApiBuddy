import { lstatSync, mkdirSync, readFileSync, rmdirSync } from "node:fs";
import { join } from "node:path";
import { atomicWrite } from "./files";
import { emptyEnvironments, validateEnvironments } from "../shared/environments";
import { emptyWorkspace, validateWorkspace } from "../shared/workspace";
import type { ServiceStore } from "./service-store";

export class StateStore<T extends { revision: number }> {
  constructor(private directory: string, private name: string, private validate: (value: unknown) => T,
    private empty: () => T, private limit = 8 * 1024 * 1024) {}
  read(): T {
    const file = join(this.directory, this.name);
    let stat;
    try { stat = lstatSync(file); }
    catch (error) { if ((error as NodeJS.ErrnoException).code === "ENOENT") return this.empty(); throw error; }
    if (!stat.isFile() || stat.size > this.limit) throw new Error(`${this.name} 不是普通文件或超过大小限制，原文件已保留`);
    try { return this.validate(JSON.parse(readFileSync(file, "utf8"))); }
    catch (error) { throw new Error(`${this.name} 无法读取，原文件已保留：${error instanceof Error ? error.message : "格式错误"}`); }
  }
  save(value: unknown): T {
    const input = this.validate(value);
    mkdirSync(this.directory, { recursive: true, mode: 0o700 });
    const lock = join(this.directory, `.${this.name}.lock`);
    try { mkdirSync(lock, { mode: 0o700 }); }
    catch (error) { if ((error as NodeJS.ErrnoException).code === "EEXIST") throw new Error("配置正在写入，请稍后重试"); throw error; }
    try {
      const current = this.read();
      if (current.revision !== input.revision) throw new Error("配置已被其他窗口修改，请重新打开后继续；当前修改尚未写入");
      const next = this.validate({ ...input, revision: input.revision + 1 });
      const content = JSON.stringify(next, null, 2) + "\n";
      if (Buffer.byteLength(content) > this.limit) throw new Error("配置过大，请减少标签或变量内容后重试");
      atomicWrite(join(this.directory, this.name), content);
      return next;
    } finally { rmdirSync(lock); }
  }
}
export function environmentStore(services: ServiceStore, serviceId: string) {
  services.get(serviceId);
  return new StateStore(services.directory(serviceId), "environments.json", validateEnvironments, emptyEnvironments, 2 * 1024 * 1024);
}
export const workspaceStore = (services: ServiceStore) => new StateStore(services.root, "workspace.json", validateWorkspace, emptyWorkspace);
