import { referencedVariables, validateDraft, validateId, type RequestDraft, type Variable } from "./templates";

export type Environment = { id: string; name: string; variables: Variable[] };
export type Environments = { schemaVersion: 1; revision: number; profiles: Environment[] };
export const emptyEnvironments = (): Environments => ({ schemaVersion: 1, revision: 0, profiles: [] });

export function validateEnvironments(value: unknown): Environments {
  const data = value as Environments | null;
  if (!data || data.schemaVersion !== 1 || !Number.isSafeInteger(data.revision) || data.revision < 0 || !Array.isArray(data.profiles) || data.profiles.length > 30) throw new Error("环境配置格式错误，最多支持 30 个环境");
  const ids = new Set<string>(), names = new Set<string>();
  const profiles = data.profiles.map((profile) => {
    if (!profile || typeof profile.name !== "string" || !profile.name.trim() || profile.name.length > 80 || /[\r\n\0]/.test(profile.name)) throw new Error("环境名称需为 1–80 个字符");
    const id = validateId(profile.id), name = profile.name.trim();
    if (ids.has(id) || names.has(name.toLowerCase())) throw new Error("环境名称或 ID 重复");
    ids.add(id); names.add(name.toLowerCase());
    const { variables } = validateDraft({ method: "GET", url: "", headers: [], query: [], body: "", timeoutMs: 30000, stream: true, variables: profile.variables });
    return { id, name, variables };
  });
  return { schemaVersion: 1, revision: data.revision, profiles };
}

// The selected environment overrides template defaults; unrelated keys stay out of the request.
export function applyEnvironment(draft: RequestDraft, environment?: Environment): RequestDraft {
  if (!environment) return validateDraft(draft);
  const checked = validateDraft(draft);
  const allowed = new Set([...checked.variables.map((row) => row.key), ...referencedVariables(checked)]);
  const values = new Map(checked.variables.map((row) => [row.key, row.value]));
  for (const { key, value } of environment.variables) if (allowed.has(key)) values.set(key, value);
  return validateDraft({ ...checked, variables: [...values].map(([key, value]) => ({ key, value })) });
}

export function selectEnvironment(data: Environments, id?: string): Environment | undefined {
  if (!id) return undefined;
  const profile = data.profiles.find((item) => item.id === id);
  if (!profile) throw new Error("所选环境已不存在，请重新选择环境后发送");
  return profile;
}
