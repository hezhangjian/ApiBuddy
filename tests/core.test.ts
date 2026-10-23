import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync, readFileSync, statSync, readdirSync, mkdirSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createServer } from "node:http";
import { randomUUID } from "node:crypto";
import { dataDirectory, TemplateStore } from "../src/main/template-store";
import { HttpClient, MAX_RESPONSE_BYTES } from "../src/main/http-client";
import { newTemplate, resolveRequest, validateTemplate } from "../src/shared/templates";
import type { RequestEvent } from "../src/shared/rpc-schema";

function temporary(run: (root: string) => void) {
  const root = mkdtempSync(join(tmpdir(), "apibuddy-store-test-"));
  try { run(root); } finally { rmSync(root, { recursive: true, force: true }); }
}
test("storage path supports default, absolute, relative and home expansion", () => {
  assert.equal(dataDirectory({}, "/home/test", "/workspace"), "/home/test/.apibuddy");
  assert.equal(dataDirectory({ APIBUDDY_DIR: "/tmp/中文 目录" }, "/home/test", "/workspace"), "/tmp/中文 目录");
  assert.equal(dataDirectory({ APIBUDDY_DIR: "~/requests" }, "/home/test", "/workspace"), "/home/test/requests");
  assert.equal(dataDirectory({ APIBUDDY_DIR: "data" }, "/home/test", "/workspace"), "/workspace/data");
  assert.throws(() => dataDirectory({ APIBUDDY_DIR: " " }), /不能为空/);
});
test("renamed storage reopens legacy templates without moving data and prefers explicit configuration", () => temporary((home) => {
  const legacy = join(home, ".apix");
  const current = join(home, ".apibuddy");
  assert.equal(dataDirectory({}, home), current);
  const saved = new TemplateStore(legacy).save(newTemplate());
  assert.equal(dataDirectory({}, home), legacy);
  assert.deepEqual(new TemplateStore(dataDirectory({}, home)).list().templates, [saved]);
  assert.deepEqual(readdirSync(home), [".apix"]);
  const intermediate = join(home, ".apiabc");
  const intermediateSaved = new TemplateStore(intermediate).save(newTemplate());
  assert.equal(dataDirectory({}, home), intermediate);
  assert.deepEqual(new TemplateStore(dataDirectory({}, home)).list().templates, [intermediateSaved]);
  assert.deepEqual(new TemplateStore(legacy).list().templates, [saved]);
  mkdirSync(current);
  assert.equal(dataDirectory({}, home), current);
  assert.equal(dataDirectory({ APIX_DIR: "~/legacy" }, home), join(home, "legacy"));
  assert.equal(dataDirectory({ APIABC_DIR: "~/intermediate", APIX_DIR: "~/legacy" }, home), join(home, "intermediate"));
  assert.equal(dataDirectory({ APIBUDDY_DIR: "current", APIABC_DIR: "intermediate", APIX_DIR: "legacy" }, home, home), join(home, "current"));
  assert.equal(dataDirectory({ APIBUDDY_DIR: "current", APIX_DIR: "legacy" }, home, home), join(home, "current"));
  assert.throws(() => dataDirectory({ APIBUDDY_DIR: "", APIX_DIR: legacy }, home), /APIBUDDY_DIR 不能为空/);
  assert.throws(() => dataDirectory({ APIX_DIR: " " }, home), /APIX_DIR 不能为空/);
  assert.throws(() => dataDirectory({ APIABC_DIR: "", APIX_DIR: legacy }, home), /APIABC_DIR 不能为空/);
}));
test("templates survive reopen, update with revisions, copy and delete", () => temporary((root) => {
  const store = new TemplateStore(join(root, "中文 目录"));
  const input = newTemplate(); input.name = "查询用户"; input.request.variables[0].value = "http://127.0.0.1";
  const first = store.save(input);
  assert.deepEqual(new TemplateStore(join(root, "中文 目录")).list().templates, [first]);
  const next = store.save({ ...input, name: "更新模板" }, first.id, first.revision);
  assert.equal(next.revision, 2); assert.equal(next.createdAt, first.createdAt);
  assert.throws(() => store.save(input, first.id, first.revision), /其他窗口/);
  assert.throws(() => store.delete(first.id, first.revision), /已被修改/);
  const copy = store.save({ ...input, name: "副本" });
  assert.notEqual(copy.id, first.id);
  const mode = statSync(join(store.directory, `${copy.id}.json`)).mode & 0o777;
  assert.equal(mode, 0o600);
  store.delete(next.id, next.revision);
  assert.equal(store.list().templates.length, 1);
  assert.deepEqual(readdirSync(store.directory), [`${copy.id}.json`]);
}));
test("bad JSON, newer schemas and symlinks are reported without overwriting", () => temporary((root) => {
  const store = new TemplateStore(root); const valid = store.save(newTemplate());
  const bad = join(store.directory, `${randomUUID()}.json`); writeFileSync(bad, "broken");
  const future = { ...valid, id: randomUUID(), schemaVersion: 99 }; writeFileSync(join(store.directory, `${future.id}.json`), JSON.stringify(future));
  symlinkSync(join(store.directory, `${valid.id}.json`), join(store.directory, `${randomUUID()}.json`));
  const list = store.list(); assert.equal(list.templates.length, 1); assert.equal(list.warnings.length, 3); assert.equal(readFileSync(bad, "utf8"), "broken");
  assert.throws(() => store.save(newTemplate(), "../../outside", 1), /ID/);
  assert.throws(() => store.delete("../../outside", 1), /ID/);
}));
test("failed writes preserve data and release the writer lock", () => temporary((root) => {
  const store = new TemplateStore(root); const first = store.save(newTemplate());
  assert.throws(() => store.save(newTemplate(), first.id, 0));
  assert.equal(store.list().templates[0].revision, 1);
  assert.equal(readdirSync(store.directory).length, 1);
  mkdirSync(join(store.directory, ".write-lock"));
  assert.throws(() => store.save(newTemplate()), /正在写入/);
  assert.equal(store.list().templates.length, 1);
}));
test("template rendering resolves every field, encodes query rows and preserves duplicate parameters", () => {
  const draft = newTemplate().request;
  draft.method = "POST"; draft.url = "{{base_url}}/users?x=0";
  draft.variables = [{ key: "base_url", value: "https://example.test" }, { key: "name", value: "你好 & $&" }];
  draft.query = [{ key: "x", value: "{{name}}", enabled: true }, { key: "off", value: "{{missing}}", enabled: false }];
  draft.headers = [{ key: "X-Name", value: "{{name}}", enabled: true }]; draft.body = '{"name":"{{name}}"}';
  const result = resolveRequest(draft);
  assert.deepEqual(new URL(result.url).searchParams.getAll("x"), ["0", "你好 & $&"]);
  assert.equal(result.headers[0].value, "你好 & $&"); assert.equal(result.body, '{"name":"你好 & $&"}');
  assert.equal(draft.url, "{{base_url}}/users?x=0");
});
test("unresolved variables, duplicates, unsupported URLs and header injection are rejected", () => {
  const draft = newTemplate().request;
  draft.url = "{{missing}}/test"; assert.throws(() => resolveRequest(draft), /未定义变量/);
  draft.url = "file:///etc/passwd"; assert.throws(() => resolveRequest(draft), /HTTP/);
  draft.url = "http://user:pass@example.test"; assert.throws(() => resolveRequest(draft), /认证/);
  draft.url = "https://example.test"; draft.headers[0].value = "ok\r\nInjected: true"; assert.throws(() => resolveRequest(draft), /请求头/);
  draft.variables.push(draft.variables[0]); assert.throws(() => validateTemplate({ ...newTemplate(), request: draft }), /重复/);
  assert.throws(() => validateTemplate({ ...newTemplate(), name: " " }), /名称/);
});
test("ignored GET body does not require unused body variables", () => {
  const draft = newTemplate().request; draft.body = "{{unused}}";
  assert.equal(resolveRequest(draft).body, "");
});

test("HTTP execution: early events, UTF-8 streaming, POST, non-2xx, timeout, cancellation, disconnect and limits", async (t) => {
  const server = createServer((req, res) => {
    if (req.url === "/slow") { const timer = setTimeout(() => res.end("late"), 2500); res.on("close", () => clearTimeout(timer)); return; }
    if (req.url === "/stream") { const value = Buffer.from("你好吗🙂"); res.write(value.subarray(0, 2)); setTimeout(() => res.end(value.subarray(2)), 55); return; }
    if (req.url === "/large") { res.end(Buffer.alloc(MAX_RESPONSE_BYTES + 1, "x")); return; }
    if (req.url === "/broken") { res.writeHead(200, { "Content-Length": "9999" }); res.write("partial"); setTimeout(() => res.destroy(), 30); return; }
    if (req.url === "/error") { res.statusCode = 422; res.end("invalid"); return; }
    let body = ""; req.on("data", (chunk) => body += chunk); req.on("end", () => res.end(JSON.stringify({ method: req.method, body, token: req.headers["x-token"] })));
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address(); assert(address && typeof address === "object");
  const base = `http://127.0.0.1:${address.port}`;
  async function request(path: string, options: { cancel?: boolean; dispose?: boolean; post?: boolean; timeout?: number } = {}) {
    const events: RequestEvent[] = [];
    let done!: () => void;
    const finished = new Promise<void>((resolve) => { done = resolve; });
    const client = new HttpClient((event) => { events.push(event); if (event.type === "end") done(); });
    const id = randomUUID(); const draft = newTemplate().request; draft.url = base + path; draft.timeoutMs = options.timeout ?? 5000;
    if (options.post) { draft.method = "POST"; draft.body = "hello {{message}}"; draft.variables.push({ key: "message", value: "世界" }); draft.headers.push({ key: "X-Token", value: "test-only", enabled: true }); }
    client.start(id, draft);
    if (options.cancel) client.cancel(id);
    if (options.dispose) client.dispose();
    const watchdog = setTimeout(() => { client.dispose(); done(); }, 8000);
    await finished; clearTimeout(watchdog);
    const ends = events.filter((event) => event.type === "end"); assert.equal(ends.length, 1);
    assert.equal(client.activeCount, 0); assert(events.every((event) => event.requestId === id));
    return { events, end: ends[0], body: events.flatMap((e) => e.type === "chunk" ? [e.text] : []).join("") };
  }
  try {
    await t.test("POST interpolation", async () => { const result = await request("/echo", { post: true }); assert.equal(result.end.phase, "completed"); assert.deepEqual(JSON.parse(result.body), { method: "POST", body: "hello 世界", token: "test-only" }); });
    await t.test("split UTF-8", async () => { const result = await request("/stream"); assert.equal(result.body, "你好吗🙂"); assert.equal(result.end.phase, "completed"); });
    await t.test("HTTP error response is still a completed exchange", async () => { const result = await request("/error"); assert.equal(result.body, "invalid"); assert.equal(result.end.phase, "completed"); assert.equal(result.events.find((e) => e.type === "headers")?.meta.status, 422); });
    await t.test("cancel before headers", async () => assert.equal((await request("/slow", { cancel: true })).end.phase, "cancelled"));
    await t.test("dispose active work", async () => assert.equal((await request("/slow", { dispose: true })).end.phase, "cancelled"));
    await t.test("timeout", async () => assert.equal((await request("/slow", { timeout: 1000 })).end.phase, "timed-out"));
    await t.test("truncated response", async () => assert.equal((await request("/broken")).end.phase, "failed"));
    await t.test("bounded response", async () => { const result = await request("/large"); assert.equal(result.end.phase, "failed"); assert.equal(result.end.bytes, MAX_RESPONSE_BYTES); assert.equal(Buffer.byteLength(result.body), MAX_RESPONSE_BYTES); });
  } finally { server.closeAllConnections(); await new Promise<void>((resolve) => server.close(() => resolve())); }
});
