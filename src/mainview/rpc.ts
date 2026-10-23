import { Electroview } from "electrobun/view";
import type { ApiBuddyRpcSchema, RequestEvent } from "../shared/rpc-schema";
import type { Result } from "../shared/templates";
const listeners = new Map<string, (event: RequestEvent) => void>();
export const rpc = Electroview.defineRPC<ApiBuddyRpcSchema>({
  maxRequestTime: 15000,
  handlers: { requests: {}, messages: {
    "request-event": (event) => {
      listeners.get(event.requestId)?.(event);
      if (event.type === "end") listeners.delete(event.requestId);
    },
  } },
});
new Electroview({ rpc });
export function listen(requestId: string, callback: (event: RequestEvent) => void) {
  listeners.set(requestId, callback);
  return () => { listeners.delete(requestId); };
}
export function unwrap<T>(result: Result<T>): T {
  if (!result.ok) throw new Error(result.error);
  return result.value;
}
export function errorText(error: unknown): string { return error instanceof Error ? error.message : "操作失败，请重试"; }
