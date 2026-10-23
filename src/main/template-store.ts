import { closeSync, existsSync, fsyncSync, lstatSync, mkdirSync, openSync, readFileSync, readdirSync, renameSync, rmdirSync, unlinkSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { isAbsolute, join, resolve } from "node:path";
import { randomUUID } from "node:crypto";
import { type Library, type SavedTemplate, validateId, validateSaved, validateTemplate } from "../shared/templates";

const MAX_FILE_BYTES = 2 * 1024 * 1024;
export function dataDirectory(env = process.env, home = homedir(), cwd = process.cwd()): string {
  const variable = ["APIBUDDY_DIR", "APIABC_DIR", "APIX_DIR"].find((name) => env[name] !== undefined);
  const configured = variable === undefined ? undefined : env[variable];
  if (configured === undefined) {
    const directory = join(home, ".apibuddy");
    return [directory, join(home, ".apiabc"), join(home, ".apix")].find(existsSync) ?? directory;
  }
  if (!configured.trim()) throw new Error(`${variable} 不能为空`);
  const expanded = configured === "~" ? home : configured.startsWith("~/") ? join(home, configured.slice(2)) : configured;
  return isAbsolute(expanded) ? resolve(expanded) : resolve(cwd, expanded);
}
export class TemplateStore {
  readonly directory: string;
  constructor(root = dataDirectory()) { this.directory = join(root, "templates"); }
  private initialize() {
    mkdirSync(this.directory, { recursive: true, mode: 0o700 });
    if (!lstatSync(this.directory).isDirectory()) throw new Error("模板目录不能是符号链接或普通文件");
  }
  private file(id: unknown) { return join(this.directory, `${validateId(id)}.json`); }
  private read(id: unknown): SavedTemplate {
    const file = this.file(id);
    const stat = lstatSync(file);
    if (!stat.isFile() || stat.size > MAX_FILE_BYTES) throw new Error("模板文件类型错误或超过 2 MiB");
    const saved = validateSaved(JSON.parse(readFileSync(file, "utf8")));
    if (saved.id !== id) throw new Error("文件名与模板 ID 不一致");
    return saved;
  }
  list(): Library {
    this.initialize();
    const templates: SavedTemplate[] = [];
    const warnings: string[] = [];
    for (const name of readdirSync(this.directory).sort()) {
      if (!name.endsWith(".json")) continue;
      try { templates.push(this.read(name.slice(0, -5))); }
      catch { warnings.push(`无法读取 ${name}；原文件已保留。`); }
    }
    templates.sort((a, b) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id));
    return { directory: this.directory, templates, warnings };
  }
  private write<T>(operation: () => T): T {
    this.initialize();
    const lock = join(this.directory, ".write-lock");
    try { mkdirSync(lock, { mode: 0o700 }); }
    catch (error) {
      if ((error as NodeJS.ErrnoException).code === "EEXIST") throw new Error("模板库正在写入，请稍后重试；若应用异常退出，请检查存储目录中的 .write-lock");
      throw error;
    }
    try { return operation(); } finally { rmdirSync(lock); }
  }
  save(input: unknown, id?: string, revision?: number): SavedTemplate {
    const template = validateTemplate(input);
    if (id !== undefined) validateId(id);
    return this.write(() => {
      const previous = id === undefined ? undefined : this.read(id);
      if (previous && previous.revision !== revision) throw new Error("此模板已被其他窗口修改，请刷新后重新打开，或另存为新模板");
      const now = new Date().toISOString();
      const saved: SavedTemplate = { ...template, schemaVersion: 1, id: previous?.id ?? randomUUID(), revision: (previous?.revision ?? 0) + 1, createdAt: previous?.createdAt ?? now, updatedAt: now };
      const serialized = JSON.stringify(saved, null, 2) + "\n";
      if (Buffer.byteLength(serialized) > MAX_FILE_BYTES) throw new Error("模板文件不能超过 2 MiB");
      const temporary = join(this.directory, `.${saved.id}.${randomUUID()}.tmp`);
      try {
        const fd = openSync(temporary, "wx", 0o600);
        try { writeFileSync(fd, serialized); fsyncSync(fd); } finally { closeSync(fd); }
        renameSync(temporary, this.file(saved.id));
      } finally { if (existsSync(temporary)) unlinkSync(temporary); }
      return saved;
    });
  }
  delete(id: string, revision: number) {
    this.write(() => {
      const previous = this.read(id);
      if (previous.revision !== revision) throw new Error("此模板已被修改，请刷新后再删除");
      unlinkSync(this.file(id));
    });
  }
}
