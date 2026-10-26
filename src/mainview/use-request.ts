import { useEffect, useRef, useState } from "react";
import type { Metadata, Phase } from "../shared/rpc-schema";
import { resolveRequest, type RequestDraft } from "../shared/templates";
import { errorText, listen, rpc, unwrap } from "./rpc";
export type ResponseState = { body: string; meta?: Metadata; phase: Phase | "idle" | "running"; elapsedMs: number; bytes: number; error?: string };
export const emptyResponse = (): ResponseState => ({ body: "", phase: "idle", elapsedMs: 0, bytes: 0 });
type Active = { id: string; unsubscribe: () => void; timer: ReturnType<typeof setInterval> };
export function useRequests() {
  const [responses, setResponses] = useState<Record<string, ResponseState>>({});
  const current = useRef(new Map<string, Active>());
  const update = (tabId: string, patch: Partial<ResponseState>) => setResponses((value) => ({ ...value, [tabId]: { ...(value[tabId] ?? emptyResponse()), ...patch } }));
  const clear = (tabId: string) => {
    const active = current.current.get(tabId);
    if (active) {
      active.unsubscribe(); clearInterval(active.timer); current.current.delete(tabId);
      void rpc.request["abort-request"]({ requestId: active.id }).catch(() => {});
    }
  };
  useEffect(() => () => { for (const tabId of current.current.keys()) clear(tabId); }, []);
  const send = async (tabId: string, serviceId: string, request: RequestDraft) => {
    clear(tabId);
    try { resolveRequest(request); } catch (error) { update(tabId, { ...emptyResponse(), phase: "failed", error: errorText(error) }); return; }
    const id = crypto.randomUUID(), started = Date.now();
    update(tabId, { ...emptyResponse(), phase: "running" });
    const unsubscribe = listen(id, (event) => {
      const active = current.current.get(tabId);
      if (active?.id !== id) return;
      if (event.type === "headers") update(tabId, { meta: event.meta });
      if (event.type === "chunk") setResponses((value) => ({ ...value, [tabId]: { ...value[tabId], body: value[tabId].body + event.text } }));
      if (event.type === "end") {
        clearInterval(active.timer); active.unsubscribe(); current.current.delete(tabId);
        update(tabId, { phase: event.phase, elapsedMs: event.elapsedMs, bytes: event.bytes, error: event.error });
      }
    });
    const timer = setInterval(() => update(tabId, { elapsedMs: Date.now() - started }), 100);
    current.current.set(tabId, { id, unsubscribe, timer });
    try { unwrap(await rpc.request["perform-request"]({ serviceId, requestId: id, request })); }
    catch (error) {
      if (current.current.get(tabId)?.id === id) { clear(tabId); update(tabId, { phase: "failed", error: errorText(error) }); }
    }
  };
  const cancel = async (tabId: string) => {
    const active = current.current.get(tabId);
    if (!active) return;
    try { unwrap(await rpc.request["abort-request"]({ requestId: active.id })); }
    catch (error) { update(tabId, { error: errorText(error) }); }
  };
  const close = (tabId: string) => { clear(tabId); setResponses((value) => { const next = { ...value }; delete next[tabId]; return next; }); };
  return { responses, send, cancel, close };
}
