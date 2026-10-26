import { useEffect, useRef, useState } from "react";
import { emptyWorkspace, type EditorTab, type Workspace } from "../shared/workspace";
import { newTemplate } from "../shared/templates";
import { errorText, rpc, unwrap } from "./rpc";

export function freshTab(serviceId = "default"): EditorTab {
  const draft = newTemplate();
  return { id: crypto.randomUUID(), serviceId, draft, baseline: draft, editorTab: "query" };
}
const initial = (): Workspace => { const tab = freshTab(); return { ...emptyWorkspace(), tabs: [tab], activeTabId: tab.id }; };
export function useWorkspace() {
  const [workspace, setWorkspace] = useState<Workspace>(initial);
  const [ready, setReady] = useState(false);
  const [status, setStatus] = useState("正在恢复工作区…");
  const [error, setError] = useState("");
  const revision = useRef(0), latest = useRef(workspace), enabled = useRef(false), writing = useRef(false), pending = useRef(false);
  latest.current = workspace;
  useEffect(() => {
    let mounted = true;
    void rpc.request["load-workspace"](undefined).then((result) => {
      const saved = unwrap(result);
      if (!mounted) return;
      revision.current = saved.revision;
      if (saved.tabs.length) setWorkspace(saved);
      enabled.current = true;
      setStatus(saved.tabs.length ? "已恢复上次工作区" : "草稿自动保存在本机");
    }).catch((caught) => { if (mounted) { setError(errorText(caught)); setStatus("草稿恢复失败，自动保存已停用"); } })
      .finally(() => { if (mounted) setReady(true); });
    return () => { mounted = false; };
  }, []);
  const persist = async () => {
    pending.current = true;
    if (!enabled.current || writing.current) return;
    writing.current = true;
    try {
      while (pending.current && enabled.current) {
        pending.current = false;
        const snapshot = latest.current;
        setStatus("正在保护草稿…");
        const saved = unwrap(await rpc.request["save-workspace"]({ data: { ...snapshot, revision: revision.current } }));
        revision.current = saved.revision;
      }
      setStatus("草稿已保存在本机");
    } catch (caught) {
      enabled.current = false;
      setError(errorText(caught)); setStatus("草稿未写入，请保存接口或重启后处理冲突");
    } finally { writing.current = false; }
  };
  useEffect(() => { if (ready) void persist(); }, [workspace, ready]);
  const updateTab = (id: string, update: (tab: EditorTab) => EditorTab) => setWorkspace((value) => ({ ...value, tabs: value.tabs.map((tab) => tab.id === id ? update(tab) : tab) }));
  return { workspace, setWorkspace, updateTab, ready, status, error };
}
