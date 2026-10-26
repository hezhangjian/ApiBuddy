import { referencedVariables, type SavedTemplate } from "./templates";

const reservedCommands = new Set(["help", "list", "request", "run", "show", "environments"]);
export const reservedOptions = new Set(["dry-run", "env", "environment", "help", "var", "vars-file"]);
export const shellQuote = (value: string) => `'${value.replace(/'/g, `'"'"'`)}'`;
export type ServiceCommand = { name: string; template: SavedTemplate };

export function serviceCommands(templates: SavedTemplate[]): ServiceCommand[] {
  const stems = templates.map(({ name }) => {
    const stem = name.trim().toLowerCase().replace(/[^\p{L}\p{N}._-]+/gu, "-").replace(/^-+|-+$/g, "") || "api";
    return reservedCommands.has(stem) ? `api-${stem}` : stem;
  });
  const counts = new Map<string, number>();
  for (const stem of stems) counts.set(stem, (counts.get(stem) ?? 0) + 1);
  const used = new Set(stems);
  return templates.map((template, i) => {
    let name = stems[i];
    if (counts.get(name)! > 1) {
      name = `${name}-${template.id}`;
      while (used.has(name)) name += "-api";
    }
    used.add(name);
    return { name, template };
  });
}

export function commandVariables(template: SavedTemplate) {
  const referenced = referencedVariables(template.request);
  return [...new Set([...referenced, ...template.request.variables.map((v) => v.key)])].sort().map((key) => ({
    name: key, referenced: referenced.includes(key),
    hasDefault: template.request.variables.some((v) => v.key === key && v.value !== ""),
  }));
}
export function commandExample(executable: string, command: ServiceCommand, supplied = new Set<string>()): string {
  const variables = commandVariables(command.template).filter((v) => v.referenced && !supplied.has(v.name) && !(v.name === "base_url" && v.hasDefault));
  const flags = variables.map(({ name }) => {
    if (/token|secret|password|api[_.-]?key/i.test(name)) return `--env ${name}=${name.toUpperCase().replace(/[.-]/g, "_")}`;
    return reservedOptions.has(name) ? `--var ${shellQuote(`${name}=<${name}>`)}` : `--${name} ${shellQuote(`<${name}>`)}`;
  });
  return [executable, command.name, ...flags].join(" ");
}
