import { useEffect, useRef, useState } from "react";
import type { Metadata, Phase } from "../shared/rpc-schema";
import { resolveRequest, type RequestDraft } from "../shared/templates";
import { errorText, listen, rpc, unwrap } from "./rpc";
export type ResponseState = { body: string; meta?: Metadata; phase: Phase | "idle" | "running"; elapsedMs: number; bytes: number; error?: string };
const empty = (): ResponseState => ({ body: "", phase: "idle", elapsedMs: 0, bytes: 0 });
export function useRequest(serviceId: string) {
  const [response, setResponse] = useState<ResponseState>(empty);
  const current = useRef<{ id: string; unsubscribe: () => void; timer: ReturnType<typeof setInterval> }>();
  const clear = () => {
    if (current.current) {
      const { id, unsubscribe, timer } = current.current;
      unsubscribe(); clearInterval(timer); current.current = undefined;
      void rpc.request["abort-request"]({ requestId: id }).catch(() => {});
    }
  };
  useEffect(() => clear, []);
  const send = async (request: RequestDraft) => {
    clear();
    try { resolveRequest(request); } catch (error) { setResponse({ ...empty(), phase: "failed", error: errorText(error) }); return; }
    const id = crypto.randomUUID();
    const started = Date.now();
    setResponse({ ...empty(), phase: "running" });
    const unsubscribe = listen(id, (event) => {
      if (current.current?.id !== id) return;
      if (event.type === "headers") setResponse((value) => ({ ...value, meta: event.meta }));
      if (event.type === "chunk") setResponse((value) => ({ ...value, body: value.body + event.text }));
      if (event.type === "end") {
        clearInterval(current.current.timer); current.current = undefined;
        setResponse((value) => ({ ...value, phase: event.phase, elapsedMs: event.elapsedMs, bytes: event.bytes, error: event.error }));
      }
    });
    const timer = setInterval(() => setResponse((value) => ({ ...value, elapsedMs: Date.now() - started })), 100);
    current.current = { id, unsubscribe, timer };
    try { unwrap(await rpc.request["perform-request"]({ serviceId, requestId: id, request })); }
    catch (error) {
      if (current.current?.id === id) { clear(); setResponse((value) => ({ ...value, phase: "failed", error: errorText(error) })); }
    }
  };
  const cancel = async () => {
    if (!current.current) return;
    try { unwrap(await rpc.request["abort-request"]({ requestId: current.current.id })); }
    catch (error) { setResponse((value) => ({ ...value, error: errorText(error) })); }
  };
  return { response, send, cancel, reset: () => { clear(); setResponse(empty()); } };
}
