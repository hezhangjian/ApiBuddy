import { BrowserView, BrowserWindow, Updater, Utils } from "electrobun/main";
import { RESOURCES_FOLDER } from "electrobun/main/paths";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import events from "electrobun/main/events";
import type { ApiBuddyRpcSchema } from "../shared/rpc-schema";
import type { Result } from "../shared/templates";
import { HttpClient } from "./http-client";
import { ServiceStore } from "./service-store";
import { generateCli } from "./cli-generator";
import { environmentStore, workspaceStore } from "./state-store";

let store: ServiceStore | undefined;
const services = () => store ??= new ServiceStore();
function guard<T>(operation: () => T): Result<T> {
  try { return { ok: true, value: operation() }; }
  catch (error) {
    const code = (error as NodeJS.ErrnoException)?.code;
    const message = code === "EACCES" || code === "EPERM" ? "存储目录没有读写权限，请检查 APIBUDDY_DIR"
      : code === "ENOENT" ? "所需服务、模板或应用文件不存在，请刷新或重新构建应用"
      : code === "ENOTDIR" ? "存储路径不是目录，请检查 APIBUDDY_DIR"
      : error instanceof Error ? error.message : "操作失败";
    return { ok: false, error: message };
  }
}
const client = new HttpClient((event) => rpc.send["request-event"](event));
const rpc = BrowserView.defineRPC<ApiBuddyRpcSchema>({
  maxRequestTime: 15000,
  handlers: { requests: {
    "abort-request": ({ requestId }) => guard(() => { client.cancel(requestId); return null; }),
    "clipboard-read": () => guard(() => Utils.clipboardReadText()),
    "clipboard-write": ({ text }) => guard(() => {
      if (typeof text !== "string" || text.length > 10 * 1024 * 1024) throw new Error("剪贴板文本过长或格式错误");
      Utils.clipboardWriteText(text); return null;
    }),
    "create-service": (input) => guard(() => services().create(input)),
    "delete-template": ({ serviceId, id, revision }) => guard(() => { services().templates(serviceId).delete(id, revision); return null; }),
    "generate-cli": ({ serviceId, environmentId }) => guard(() => generateCli(services(), serviceId, readFileSync(join(RESOURCES_FOLDER, "app/cli/runtime.cjs"), "utf8"), environmentId)),
    "list-services": () => guard(() => services().list()),
    "list-templates": ({ serviceId }) => guard(() => services().templates(serviceId).list()),
    "load-environments": ({ serviceId }) => guard(() => environmentStore(services(), serviceId).read()),
    "save-environments": ({ serviceId, data }) => guard(() => environmentStore(services(), serviceId).save(data)),
    "load-workspace": () => guard(() => workspaceStore(services()).read()),
    "save-workspace": ({ data }) => guard(() => workspaceStore(services()).save(data)),
    "perform-request": ({ serviceId, requestId, request }) => guard(() => { services().get(serviceId); client.start(requestId, request); return null; }),
    "save-template": ({ serviceId, template, id, revision }) => guard(() => services().templates(serviceId).save(template, id, revision)),
  } },
});
let url = "views://mainview/index.html";
if (await Updater.localInfo.channel() === "dev" && process.env.APIBUDDY_HMR === "1") {
  url = "http://localhost:5173";
}
const window = new BrowserWindow({ title: "ApiBuddy — 请求工作台", url, rpc,
  frame: { width: 1320, height: 860, x: 60, y: 50 } });
events.on("close", (event: { data: { id: number } }) => { if (event.data.id === window.id) client.dispose(); });
