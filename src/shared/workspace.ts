import { validateDraft, validateId, type TemplateInput } from "./templates";
import { validateServiceId } from "./services";

export type EditorTab = {
  id: string; serviceId: string; templateId?: string; revision?: number;
  draft: TemplateInput; baseline: TemplateInput; environmentId?: string; editorTab: string;
};
export type Workspace = { schemaVersion: 1; revision: number; activeTabId: string; tabs: EditorTab[] };
export const MAX_TABS = 20;
export const emptyWorkspace = (): Workspace => ({ schemaVersion: 1, revision: 0, activeTabId: "", tabs: [] });

function editorInput(value: unknown): TemplateInput {
  const input = value as TemplateInput | null;
  if (!input || typeof input.name !== "string" || input.name.length > 120 || typeof input.description !== "string" || input.description.length > 2000 || !input.request) throw new Error("草稿格式错误");
  // Incomplete variable names and empty template names are valid editor states.
  const request = validateDraft({ ...input.request, variables: [] });
  if (!Array.isArray(input.request.variables) || input.request.variables.length > 100) throw new Error("草稿变量过多");
  request.variables = input.request.variables.map((row) => {
    if (!row || typeof row.key !== "string" || row.key.length > 100 || typeof row.value !== "string" || row.value.length > 16384) throw new Error("草稿变量格式错误");
    return { key: row.key, value: row.value };
  });
  return { name: input.name, description: input.description, request };
}

export function validateWorkspace(value: unknown): Workspace {
  const data = value as Workspace | null;
  if (!data || data.schemaVersion !== 1 || !Number.isSafeInteger(data.revision) || data.revision < 0 || !Array.isArray(data.tabs) || data.tabs.length > MAX_TABS) throw new Error("工作区格式错误或标签过多");
  const ids = new Set<string>();
  const tabs = data.tabs.map((tab) => {
    if (!tab) throw new Error("标签格式错误");
    const id = validateId(tab.id);
    if (ids.has(id)) throw new Error("标签 ID 重复");
    ids.add(id);
    if (tab.templateId !== undefined) {
      validateId(tab.templateId);
      if (!Number.isSafeInteger(tab.revision) || Number(tab.revision) < 1) throw new Error("模板修订号无效");
    }
    if (tab.environmentId) validateId(tab.environmentId);
    return { id, serviceId: validateServiceId(tab.serviceId), templateId: tab.templateId, revision: tab.revision,
      draft: editorInput(tab.draft), baseline: editorInput(tab.baseline), environmentId: tab.environmentId,
      editorTab: ["query", "headers", "body", "variables", "preview"].includes(tab.editorTab) ? tab.editorTab : "query" };
  });
  if ((tabs.length && !ids.has(data.activeTabId)) || (!tabs.length && data.activeTabId !== "")) throw new Error("当前标签不存在");
  return { schemaVersion: 1, revision: data.revision, activeTabId: data.activeTabId, tabs };
}
