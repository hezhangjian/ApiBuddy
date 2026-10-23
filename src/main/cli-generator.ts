import { lstatSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import type { AgentBundle, CliConfig, Service } from "../shared/services";
import { commandExample, serviceCommands, shellQuote, type ServiceCommand } from "../shared/cli-commands";
import { atomicWrite } from "./files";
import { ServiceStore } from "./service-store";

export function agentMarkdown(service: Service, setup: string, commands: ServiceCommand[], warnings: string[]): string {
  const executable = `apibuddy ${service.slug}`;
  const label = service.name.replace(/[\[\]<>`#*]/g, "");
  const examples = commands.map((command) => `# ${command.template.request.method} · ${command.template.description.replace(/[\r\n`]/g, " ") || command.template.name.replace(/[\r\n`]/g, " ")}\n${commandExample(executable, command)}`).join("\n\n");
  return `## ${label}\n\n统一使用 apibuddy <服务> <接口> 调用。需要 Node.js 18+ 和本机命令执行能力。\n\n\`\`\`sh\n${setup}\n\napibuddy --help  # 查看全部服务\n${executable} --help  # 查看这个服务的全部接口和参数\n${examples ? `\n${examples}\n` : ""}\`\`\`\n\n尖括号替换成实际值，有默认值的参数可省略。追加 --dry-run 可先预览；执行后返回 JSON。Agent 每次执行时，将初始化两行与所需接口命令放在同一次 shell 调用中。\n\n没保存成模板的接口也能调用（路径是服务地址后的接口路径）：\n\`\`\`sh\n${executable} request GET /path\n\`\`\`\n\n服务和接口以 --help 实时列出的内容为准；新增、修改后立即生效，无需重新生成 CLI。仅执行用户当前任务授权的请求。${warnings.length ? "\n\n部分接口文件无法读取，请在 ApiBuddy 中检查存储提示。" : ""}\n`;
}
export function generateCli(store: ServiceStore, serviceId: string, runtime: string): AgentBundle {
  const service = store.get(serviceId);
  if (!runtime.trim()) throw new Error("CLI 运行时未打包，请重新构建应用");
  const library = store.templates(serviceId).list();
  const commands = serviceCommands(library.templates);
  const directory = join(store.directory(serviceId), "cli");
  const sharedDirectory = join(store.root, "cli");
  const bin = join(store.root, "bin");
  for (const folder of new Set([directory, sharedDirectory, bin])) {
    mkdirSync(folder, { recursive: true, mode: 0o700 });
    if (!lstatSync(folder).isDirectory()) throw new Error("CLI 目录不能是符号链接或普通文件");
  }
  const cliPath = join(bin, "apibuddy");
  const runtimePath = join(sharedDirectory, "apibuddy.cjs");
  const markdownPath = join(directory, "AGENT.md");
  const config: CliConfig = { dataRoot: store.root };
  const setup = `export APIBUDDY_DIR=${shellQuote(store.root)}\nexport PATH="$APIBUDDY_DIR/bin:$PATH"`;
  const markdown = agentMarkdown(service, setup, commands, library.warnings);
  atomicWrite(runtimePath, `#!/usr/bin/env node\nconst APIBUDDY_CLI_CONFIG = ${JSON.stringify(config)};\n${runtime}\n`, 0o700);
  // Keep the public command short; its CommonJS runtime works inside ESM projects too.
  atomicWrite(cliPath, `#!/bin/sh\nexec node "$(dirname "$0")/../cli/apibuddy.cjs" "$@"\n`, 0o700);
  atomicWrite(markdownPath, markdown);
  return { cliPath, markdownPath, markdown, serviceId, setup, warnings: library.warnings,
    commands: commands.map((command) => ({ name: command.name, method: command.template.request.method,
      description: command.template.description || command.template.name, example: commandExample(`apibuddy ${service.slug}`, command) })) };
}
