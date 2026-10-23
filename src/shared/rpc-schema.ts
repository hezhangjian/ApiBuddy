import type { RPCSchema } from "electrobun/view";
import type { Library, RequestDraft, Result, SavedTemplate, TemplateInput } from "./templates";
import type { AgentBundle, Service, ServiceCatalog, ServiceInput } from "./services";
export type Metadata = { status: number; statusText: string; headers: [string, string][]; url: string };
export type Phase = "completed" | "cancelled" | "failed" | "timed-out";
export type RequestEvent = { requestId: string } & (
  | { type: "headers"; meta: Metadata }
  | { type: "chunk"; text: string }
  | { type: "end"; phase: Phase; error?: string; elapsedMs: number; bytes: number }
);
export type ApiBuddyRpcSchema = {
  bun: RPCSchema<{
    requests: {
      "abort-request": { params: { requestId: string }; response: Result<null> };
      "clipboard-read": { params: void; response: Result<string | null> };
      "clipboard-write": { params: { text: string }; response: Result<null> };
      "create-service": { params: ServiceInput; response: Result<Service> };
      "delete-template": { params: { serviceId: string; id: string; revision: number }; response: Result<null> };
      "generate-cli": { params: { serviceId: string }; response: Result<AgentBundle> };
      "list-services": { params: void; response: Result<ServiceCatalog> };
      "list-templates": { params: { serviceId: string }; response: Result<Library> };
      "perform-request": { params: { serviceId: string; requestId: string; request: RequestDraft }; response: Result<null> };
      "save-template": { params: { serviceId: string; template: TemplateInput; id?: string; revision?: number }; response: Result<SavedTemplate> };
    }; messages: Record<string, never>;
  }>;
  webview: RPCSchema<{ requests: Record<string, never>; messages: { "request-event": RequestEvent } }>;
};
