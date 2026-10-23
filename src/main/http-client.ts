import type { RequestEvent, Phase } from "../shared/rpc-schema";
import { resolveRequest, validateId, type RequestDraft, type ResolvedRequest } from "../shared/templates";
export const MAX_RESPONSE_BYTES = 8 * 1024 * 1024;
type Active = { controller: AbortController; reason?: "cancelled" | "timed-out" };
type Sink = (event: RequestEvent) => void | Promise<void>;
export class HttpClient {
  private active = new Map<string, Active>();
  constructor(private emit: Sink) {}
  get activeCount() { return this.active.size; }
  start(requestId: string, draft: RequestDraft, substitute = true) {
    validateId(requestId);
    if (this.active.has(requestId)) throw new Error("请求 ID 已存在");
    if (this.active.size >= 8) throw new Error("同时执行的请求过多");
    const request = resolveRequest(draft, substitute);
    const active: Active = { controller: new AbortController() };
    this.active.set(requestId, active);
    void this.run(requestId, request, active);
  }
  cancel(requestId: string) {
    validateId(requestId);
    const active = this.active.get(requestId);
    if (active) { active.reason ??= "cancelled"; active.controller.abort(); }
  }
  dispose() { for (const id of this.active.keys()) this.cancel(id); }
  private async run(requestId: string, request: ResolvedRequest, active: Active) {
    const started = Date.now();
    let bytes = 0;
    let phase: Phase = "completed";
    let error: string | undefined;
    let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
    let pending = "";
    const timer = setTimeout(() => { active.reason ??= "timed-out"; active.controller.abort(); }, request.timeoutMs);
    try {
      const headers = new Headers();
      for (const row of request.headers) headers.append(row.key, row.value);
      const response = await fetch(request.url, { method: request.method, headers,
        body: ["GET", "HEAD"].includes(request.method) ? undefined : request.body, signal: active.controller.signal });
      await this.emit({ requestId, type: "headers", meta: { status: response.status, statusText: response.statusText, headers: Array.from(response.headers.entries()), url: response.url } });
      reader = response.body?.getReader();
      const decoder = new TextDecoder();
      if (reader) {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          const remaining = MAX_RESPONSE_BYTES - bytes;
          const accepted = value.subarray(0, Math.max(0, remaining));
          bytes += accepted.length;
          pending += decoder.decode(accepted, { stream: true });
          if (value.length > remaining) throw new Error("limit");
          if (request.stream && pending) {
            await this.emit({ requestId, type: "chunk", text: pending });
            pending = "";
          }
        }
      }
      pending += decoder.decode();
      if (active.reason) phase = active.reason;
    } catch (caught) {
      phase = active.reason ?? "failed";
      if (phase === "failed") error = caught instanceof Error && caught.message === "limit"
        ? "响应超过 8 MiB，已停止接收并保留已收到的内容" : "请求失败，请检查地址、网络、TLS 证书或响应是否中断";
    } finally {
      clearTimeout(timer);
      active.controller.abort();
      try { await reader?.cancel(); } catch { /* The stream may already be closed. */ }
      try { reader?.releaseLock(); } catch { /* Cancellation may have released the reader. */ }
      this.active.delete(requestId);
      try {
        if (pending) await this.emit({ requestId, type: "chunk", text: pending });
        await this.emit({ requestId, type: "end", phase, error, elapsedMs: Date.now() - started, bytes });
      } catch { /* A closed renderer must not keep the request alive. */ }
    }
  }
}
