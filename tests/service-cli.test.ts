import { test } from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, rmSync, readFileSync, writeFileSync, statSync, symlinkSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { ServiceStore } from "../src/main/service-store";
import { environmentStore } from "../src/main/state-store";
import { TemplateStore } from "../src/main/template-store";
import { generateCli } from "../src/main/cli-generator";
import { serviceCommands } from "../src/shared/cli-commands";
import { buildServiceTemplate } from "../src/shared/service-templates";
import { METHODS, newTemplate, resolveRequest } from "../src/shared/templates";

const serviceInput = (slug: string) => ({ name: `服务 ${slug}`, slug, description: "测试服务", baseUrl: "http://127.0.0.1" });
test("new interfaces persist under their service and discover path variables for CLI calls", () => {
  const root = mkdtempSync(join(tmpdir(), "apibuddy-interface-"));
  try {
    const store = new ServiceStore(root);
    const service = store.create({ ...serviceInput("orders"), baseUrl: "https://example.test/v1/" });
    const input = buildServiceTemplate(service, { name: " get-order ", method: "GET", endpoint: "/orders/{{id}}?status={{status}}" });
    const saved = store.templates(service.id).save(input);
    assert.equal(saved.name, "get-order");
    assert.equal(saved.request.url, "{{base_url}}/orders/{{id}}?status={{status}}");
    const reopened = new ServiceStore(root).templates(service.id).list().templates[0];
    reopened.request.variables = reopened.request.variables.map((variable) => ({ ...variable, value: ({ id: "123", status: "pending" } as Record<string, string>)[variable.key] ?? variable.value }));
    assert.equal(resolveRequest(reopened.request).url, "https://example.test/v1/orders/123?status=pending");
    assert.equal(serviceCommands([reopened])[0].name, "get-order");
    assert.equal(store.templates("default").list().templates.length, 0);
    const absolute = buildServiceTemplate(service, { name: "create-order", method: "POST", endpoint: "https://other.test/orders" });
    assert.equal(resolveRequest(absolute.request).method, "POST");
    assert.equal(resolveRequest(absolute.request).url, "https://other.test/orders");
    assert.deepEqual(absolute.request.variables, []);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("new interfaces reject missing names, invalid URLs and paths without a service address", () => {
  const service = { ...serviceInput("orders"), id: randomUUID(), schemaVersion: 1 as const };
  const valid = { name: "get-order", method: "GET" as const, endpoint: "/orders" };
  assert.throws(() => buildServiceTemplate(service, { ...valid, name: " " }), /名称/);
  for (const endpoint of ["", "orders", "//other.test/orders", "file:///tmp/data", "https://", "http://user:pass@example.test", "/orders\n/test"]) {
    assert.throws(() => buildServiceTemplate(service, { ...valid, endpoint }));
  }
  assert.throws(() => buildServiceTemplate({ ...service, baseUrl: "" }, valid), /未设置地址/);
  assert.equal(buildServiceTemplate({ ...service, baseUrl: "" }, { ...valid, endpoint: "https://example.test/orders" }).request.url, "https://example.test/orders");
});

test("services isolate templates and preserve the existing default library", () => {
  const root = mkdtempSync(join(tmpdir(), "apibuddy-services-"));
  try {
    const legacy = new TemplateStore(root).save(newTemplate());
    const services = new ServiceStore(root);
    const a = services.create(serviceInput("orders"));
    const b = services.create(serviceInput("users"));
    const saved = services.templates(a.id).save({ ...newTemplate(), name: "共享名称" });
    services.templates(b.id).save({ ...newTemplate(), name: "共享名称" });
    assert.deepEqual(services.templates("default").list().templates, [legacy]);
    assert.equal(services.templates(a.id).list().templates[0].id, saved.id);
    assert.notEqual(services.templates(b.id).list().templates[0].id, saved.id);
    assert.throws(() => services.templates(b.id).save(newTemplate(), saved.id, saved.revision));
    assert.throws(() => services.templates(b.id).delete(saved.id, saved.revision));
    assert.equal(services.templates(a.id).list().templates.length, 1);
    assert.equal(new ServiceStore(root).list().services.length, 3);
    assert.throws(() => services.create(serviceInput("orders")), /已存在/);
    assert.throws(() => services.create(serviceInput("../escape")), /标识/);
    assert.throws(() => services.templates("../escape"), /ID/);
    assert.throws(() => services.templates(randomUUID()));
    assert.equal(statSync(join(services.directory(a.id), "service.json")).mode & 0o777, 0o600);
    assert.equal(services.create(serviceInput("after-error")).slug, "after-error");
    const commands = serviceCommands([
      { ...saved, name: "Get Order" }, { ...saved, id: randomUUID(), name: "get-order" },
      { ...saved, id: randomUUID(), name: "request" }, { ...saved, id: randomUUID(), name: "用户查询" },
    ]);
    assert.equal(new Set(commands.map((entry) => entry.name)).size, 4);
    assert.equal(commands[2].name, "api-request"); assert.equal(commands[3].name, "用户查询");
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("damaged or linked service metadata and template directories cannot cross services", () => {
  const root = mkdtempSync(join(tmpdir(), "apibuddy-service-errors-"));
  try {
    const services = new ServiceStore(root);
    const a = services.create(serviceInput("a"));
    const b = services.create(serviceInput("b"));
    const file = join(services.directory(b.id), "service.json");
    writeFileSync(file, "broken");
    symlinkSync(services.directory(a.id), join(root, "services", randomUUID()));
    const mismatch = randomUUID(); mkdirSync(join(root, "services", mismatch));
    writeFileSync(join(root, "services", mismatch, "service.json"), JSON.stringify(a));
    assert.equal(services.list().warnings.length, 3);
    assert.equal(services.list().services.length, 2);
    assert.equal(readFileSync(file, "utf8"), "broken");
    const c = services.create(serviceInput("c"));
    services.templates(a.id).list();
    symlinkSync(services.templates(a.id).directory, join(services.directory(c.id), "templates"));
    assert.throws(() => services.templates(c.id).list(), /符号链接/);
    mkdirSync(join(root, "services", "default"));
    assert.equal(services.list().warnings.length, 4);
    symlinkSync(services.directory(a.id), join(services.directory(c.id), "cli"));
    assert.throws(() => generateCli(services, c.id, "runtime"), /符号链接/);
    const linked = join(root, "linked"); mkdirSync(linked);
    symlinkSync(join(root, "services"), join(linked, "services"));
    assert.throws(() => new ServiceStore(linked).get(a.id), /符号链接/);
    assert.throws(() => new ServiceStore(linked).create(serviceInput("blocked")), /符号链接/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("generated standalone CLI: discovery, overrides, real HTTP, isolation and live saved edits", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "apibuddy-cli-中文 space-'quote-"));
  const services = new ServiceStore(root);
  writeFileSync(join(root, "package.json"), JSON.stringify({ type: "module" }));
  let received = 0;
  let onSlow: (() => void) | undefined;
  const server = createServer((req, res) => {
    received++;
    if (req.url === "/slow") { onSlow?.(); return; }
    if (req.url === "/error") { res.statusCode = 422; res.end("bad input"); return; }
    let body = "";
    req.on("data", (part) => body += part);
    req.on("end", () => res.end(JSON.stringify({ body, method: req.method, token: req.headers["x-token"], url: req.url })));
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address(); assert(address && typeof address === "object");
  const baseUrl = `http://127.0.0.1:${address.port}`;
  const service = services.create({ ...serviceInput("orders"), baseUrl });
  const other = services.create(serviceInput("users"));
  const input = newTemplate();
  input.name = "创建订单"; input.request.method = "POST"; input.request.url = "{{base_url}}/echo";
  input.request.body = "{{message}}";
  input.request.headers.push({ key: "X-Token", value: "{{token}}", enabled: true });
  input.request.variables = [{ key: "base_url", value: baseUrl }, { key: "message", value: "saved-value" }, { key: "token", value: "test-secret-only" }];
  const template = services.templates(service.id).save(input);
  const foreign = services.templates(other.id).save(input);
  const runtime = readFileSync(process.env.APIBUDDY_TEST_CLI_RUNTIME!, "utf8");
  const bundle = generateCli(services, service.id, runtime);
  function command(args: string[], env: Record<string, string | undefined> = {}, viaPath = false, serviceSlug: string | null = "orders") {
    const child = spawn(viaPath ? "apibuddy" : bundle.cliPath, serviceSlug === null ? args : [serviceSlug, ...args], { cwd: tmpdir(), env: { ...process.env, APIBUDDY_DIR: undefined, APIABC_DIR: undefined, APIX_DIR: undefined, PATH: `${root}/bin:${process.env.PATH}`, ...env }, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "", stderr = "";
    child.stdout.on("data", (part) => stdout += part); child.stderr.on("data", (part) => stderr += part);
    const done = new Promise<{ code: number | null; text: string; data: any }>((resolve, reject) => {
      child.on("error", reject); child.on("close", (code) => {
        try { assert.equal(stderr, ""); resolve({ code, text: stdout, data: stdout.trim().startsWith("{") ? JSON.parse(stdout) : undefined }); } catch (error) { reject(error); }
      });
    });
    return { child, done };
  }
  const run = (args: string[], env?: Record<string, string | undefined>) => command(args, env).done;
  try {
    await t.test("CLI and Markdown are private standalone files; discovery hides variable defaults", async () => {
      assert.equal(statSync(bundle.cliPath).mode & 0o777, 0o700);
      assert.equal(statSync(bundle.markdownPath).mode & 0o777, 0o600);
      assert.equal(readFileSync(bundle.markdownPath, "utf8"), bundle.markdown);
      assert(bundle.markdown.includes("'\"'\"'"));
      const list = await run(["list"]); assert.equal(list.code, 0); assert.equal(list.data.templates.length, 1);
      assert.equal(list.data.service.id, service.id); assert(!list.text.includes("test-secret-only"));
      const show = await run(["show", "创建订单"]); assert.equal(show.data.template.id, template.id);
      assert(show.data.template.variables.find((v: { name: string }) => v.name === "token").hasDefault);
      assert(!show.text.includes("test-secret-only"));
    });
    await t.test("a single apibuddy executable discovers and routes services without binding to the generated guide", async () => {
      assert.equal(bundle.cliPath, join(root, "bin", "apibuddy"));
      const rootHelp = await command(["--help"], {}, true, null).done;
      assert.equal(rootHelp.code, 0); assert(rootHelp.text.includes("apibuddy orders")); assert(rootHelp.text.includes("apibuddy users"));
      const noArgs = await command([], {}, false, null).done; assert.equal(noArgs.code, 0); assert(noArgs.text.includes("apibuddy default"));
      const userList = await command(["list"], {}, true, "users").done;
      assert.equal(userList.data.templates.length, 1); assert.equal(userList.data.templates[0].id, foreign.id);
      assert.equal(userList.data.service.id, other.id);
      assert.equal((await command(["run", template.id], {}, false, "users").done).code, 2);
      assert.equal((await command(["list"], {}, false, "missing-service").done).code, 2);
      assert.equal((await command(["list"], {}, false, "../outside").done).code, 2);
      const usersBundle = generateCli(services, other.id, runtime);
      assert.equal(usersBundle.cliPath, bundle.cliPath);
      assert(usersBundle.markdown.includes("apibuddy users")); assert(!usersBundle.markdown.includes("apibuddy orders"));
      assert.equal((await run(["list"])).data.service.id, service.id);
      const late = services.create({ ...serviceInput("billing"), baseUrl });
      const lateTemplate = services.templates(late.id).save({ ...input, name: "pay-invoice" });
      const refreshed = await command(["--help"], {}, false, null).done; assert(refreshed.text.includes("apibuddy billing"));
      const lateResult = await command(["pay-invoice", "--message", "late-service"], {}, true, "billing").done;
      assert.equal(lateResult.code, 0); assert.equal(lateResult.data.templateId, lateTemplate.id);
      assert.equal(JSON.parse(lateResult.data.body).body, "late-service");
      const serviceHelp = await command([], {}, false, "billing").done; assert(serviceHelp.text.includes("apibuddy billing pay-invoice"));
      const defaultList = await command(["list"], {}, false, "default").done; assert.equal(defaultList.data.templates.length, 0);
    });
    await t.test("dry-run sends nothing, supports literals and ordered environment/file overrides", async () => {
      const file = join(root, "variables.json"); writeFileSync(file, JSON.stringify({ message: "from file", token: "file-token" }));
      const count = received;
      const result = await run(["run", template.id, "--vars-file", file, "--var", "message=你好 '$& $(ignored)=value", "--env", "token=APIBUDDY_TEST_TOKEN", "--dry-run"], { APIBUDDY_TEST_TOKEN: "env-token" });
      assert.equal(result.code, 0); assert.equal(result.data.request.body, "你好 '$& $(ignored)=value");
      assert.equal(result.data.request.headers.find((h: { key: string }) => h.key === "X-Token").value, "env-token");
      assert.equal(received, count);
      assert.equal(services.templates(service.id).list().templates[0].request.variables[1].value, "saved-value");
    });
    await t.test("executes actual HTTP then reads an edited template without regeneration", async () => {
      const first = await run(["run", template.id, "--var", "message=中文🙂"]);
      assert.equal(first.code, 0); assert.equal(first.data.status, 200); assert.equal(JSON.parse(first.data.body).body, "中文🙂");
      services.templates(service.id).save({ ...input, request: { ...input.request, body: "updated {{message}}" } }, template.id, 1);
      const second = await run(["run", template.id]); assert.equal(JSON.parse(second.data.body).body, "updated saved-value");
    });
    await t.test("foreign IDs, ambiguous names, unknown variables and missing environment fail before HTTP", async () => {
      const count = received;
      for (const args of [["run", foreign.id], ["run", template.id, "--var", "unexpected=x"], ["run", template.id, "--env", "token=APIBUDDY_TEST_MISSING"], ["show", template.id, "extra"], ["list", "extra"], ["run", template.id, "--bad"]]) {
        assert.equal((await run(args, { APIBUDDY_TEST_MISSING: undefined })).code, 2);
      }
      services.templates(service.id).save(input);
      assert.equal((await run(["run", input.name])).code, 2);
      assert.equal((await run(["show", template.id])).code, 0);
      assert.equal(received, count);
    });
    await t.test("HTTP errors, timeout and SIGINT have distinct JSON results and exit codes", async () => {
      const failed = services.templates(service.id).save({ ...input, request: { ...input.request, url: baseUrl + "/error" } });
      const result = await run(["run", failed.id]); assert.equal(result.code, 4); assert.equal(result.data.status, 422); assert.equal(result.data.ok, false);
      const slow = services.templates(service.id).save({ ...input, request: { ...input.request, url: baseUrl + "/slow", timeoutMs: 1000 } });
      const timeout = await run(["run", slow.id]); assert.equal(timeout.code, 3); assert.equal(timeout.data.phase, "timed-out");
      const arrived = new Promise<void>((resolve) => { onSlow = resolve; });
      const interrupted = command(["run", slow.id]); await arrived; interrupted.child.kill("SIGINT");
      const cancelled = await interrupted.done; assert.equal(cancelled.code, 130); assert.equal(cancelled.data.phase, "cancelled");
      onSlow = undefined;
    });
    await t.test("one CLI calls every saved interface directly and discovers newly added interfaces", async () => {
      const names: string[] = [];
      for (const method of METHODS) {
        const name = `${method.toLowerCase()}-orders`; names.push(name);
        services.templates(service.id).save({ ...input, name, request: { ...input.request, method, url: baseUrl + "/echo?order={{id}}", variables: [...input.request.variables, { key: "id", value: "10" }] } });
        const result = await command([name, "--id", "20", "--message", "中文"], {}, true).done;
        assert.equal(result.code, 0); assert.equal(result.data.status, 200);
        if (method !== "HEAD") { const body = JSON.parse(result.data.body); assert.equal(body.method, method); assert.equal(body.url, "/echo?order=20"); }
      }
      const help = await run(["--help"]); assert.equal(help.code, 0);
      for (const name of names) assert(help.text.includes(name));
      assert(!help.text.includes("test-secret-only"));
      const details = await run(["get-orders", "--help"]); assert(details.text.includes("--id"));
      const dry = await run(["get-orders", "--id=30", "--dry-run"]); assert(dry.data.request.url.includes("order=30"));
      assert.equal((await run(["get-orders", "--unexpected", "x"])).code, 2);
      assert.equal((await run(["get-orders", "--id", "--dry-run"])).code, 2);
      const updated = generateCli(services, service.id, runtime);
      for (const name of names) assert(updated.markdown.includes(`apibuddy orders ${name}`));
      assert(updated.setup.includes("APIBUDDY_DIR=")); assert(!updated.markdown.includes("test-secret-only"));
      assert.equal(updated.commands.length, services.templates(service.id).list().templates.length);
    });
    await t.test("request invokes unsaved service paths with HTTP methods, body, headers and query", async () => {
      const count = received;
      const dry = await run(["request", "GET", "/not-saved", "--query", "q=中文 & ?", "--dry-run"]);
      assert.equal(dry.code, 0); assert.equal(new URL(dry.data.request.url).searchParams.get("q"), "中文 & ?"); assert.equal(received, count);
      for (const method of METHODS) {
        const args = ["request", method, "/unregistered", "--query", "id=123", "--header", "X-Token: raw-test"];
        if (!["GET", "HEAD"].includes(method)) args.push("--data", '{"name":"手机"}');
        const result = await run(args); assert.equal(result.code, 0); assert.equal(result.data.status, 200);
        if (method !== "HEAD") { const body = JSON.parse(result.data.body); assert.equal(body.url, "/unregistered?id=123"); assert.equal(body.token, "raw-test"); assert.equal(body.method, method); }
      }
      for (const path of ["https://outside.test/x", "//outside.test/x", "/\\outside.test/x"]) assert.equal((await run(["request", "GET", path])).code, 2);
      assert.equal((await run(["request", "GET", "/x", "--data", "ignored"])).code, 2);
      assert.equal((await run(["request", "INVALID", "/x"])).code, 2);
      const literal = await run(["request", "POST", "/unregistered", "--data", "literal {{ name }} $&"]);
      assert.equal(literal.code, 0); assert.equal(JSON.parse(literal.data.body).body, "literal {{ name }} $&");
    });
    await t.test("desktop environments drive live CLI requests and explicit options override environment defaults", async () => {
      const store = environmentStore(services, service.id);
      const profile = { id: randomUUID(), name: "staging", variables: [
        { key: "base_url", value: baseUrl }, { key: "message", value: "from-profile" }, { key: "token", value: "profile-test-token" },
      ] };
      const first = store.save({ ...store.read(), profiles: [profile] });
      const selectedBundle = generateCli(services, service.id, runtime, profile.id);
      assert.equal(selectedBundle.environmentName, "staging");
      assert(selectedBundle.commands.every((entry) => entry.example.endsWith("--environment 'staging'")));
      assert(!selectedBundle.markdown.includes("profile-test-token"));
      assert(selectedBundle.commands.every((entry) => !entry.example.includes("--env token=")));
      assert.equal(readFileSync(selectedBundle.markdownPath, "utf8"), selectedBundle.markdown);
      assert.throws(() => generateCli(services, service.id, runtime, randomUUID()), /不存在/);
      const list = await run(["environments"]);
      assert.equal(list.code, 0); assert.equal(list.data.environments[0].name, "staging"); assert(!list.text.includes("profile-test-token"));
      const fromProfile = await run(["run", template.id, "--environment", "staging"]);
      assert.equal(fromProfile.code, 0); assert.equal(JSON.parse(fromProfile.data.body).body, "updated from-profile");
      assert.equal(JSON.parse(fromProfile.data.body).token, "profile-test-token");
      const override = await run(["run", template.id, "--var", "message=explicit", "--environment=staging", "--env", "token=APIBUDDY_TEST_TOKEN"], { APIBUDDY_TEST_TOKEN: "explicit-token" });
      assert.equal(JSON.parse(override.data.body).body, "updated explicit"); assert.equal(JSON.parse(override.data.body).token, "explicit-token");
      assert.equal(services.templates(service.id).list().templates.find((item) => item.id === template.id)!.request.variables[1].value, "saved-value");
      const count = received;
      for (const args of [["run", template.id, "--environment", "missing"], ["run", template.id, "--environment"], ["run", template.id, "--environment", "staging", "--environment", "staging"]]) assert.equal((await run(args)).code, 2);
      assert.equal(received, count);
      store.save({ ...first, profiles: [{ ...profile, variables: [{ key: "base_url", value: baseUrl + "/env-prefix" }] }] });
      const raw = await run(["request", "GET", "/echo", "--environment", "staging"]);
      assert.equal(raw.code, 0); assert.equal(JSON.parse(raw.data.body).url, "/env-prefix/echo");
      const literal = await run(["request", "POST", "/echo", "--data", "--environment"]);
      assert.equal(literal.code, 0); assert.equal(JSON.parse(literal.data.body).body, "--environment");
    });
    await t.test("APIBUDDY_DIR selects the new service catalog without falling back to the original root", async () => {
      const alternate = join(root, "relocated"); mkdirSync(join(alternate, "services"), { recursive: true });
      cpSync(services.directory(service.id), join(alternate, "services", service.id), { recursive: true });
      const list = await run(["list"], { APIBUDDY_DIR: alternate }); assert.equal(list.code, 0); assert.equal(list.data.service.id, service.id);
      assert.equal((await run(["show", foreign.id], { APIBUDDY_DIR: alternate })).code, 2);
      assert.equal((await run(["list"], { APIBUDDY_DIR: "" })).code, 2);
      assert.equal((await command(["list"], { APIBUDDY_DIR: alternate }, false, "users").done).code, 2);
      const unrelatedRoot = join(root, "other-root");
      const unrelatedStore = new ServiceStore(unrelatedRoot);
      const differentOrders = unrelatedStore.create(serviceInput("orders"));
      const otherList = await run(["list"], { APIBUDDY_DIR: unrelatedRoot });
      assert.equal(otherList.data.service.id, differentOrders.id); assert.equal(otherList.data.templates.length, 0);
      const legacyList = await run(["list"], { APIX_DIR: unrelatedRoot });
      assert.equal(legacyList.data.service.id, differentOrders.id);
      const intermediateList = await run(["list"], { APIABC_DIR: unrelatedRoot, APIX_DIR: alternate });
      assert.equal(intermediateList.data.service.id, differentOrders.id);
      const allNames = await run(["list"], { APIBUDDY_DIR: alternate, APIABC_DIR: unrelatedRoot, APIX_DIR: unrelatedRoot });
      assert.equal(allNames.data.service.id, service.id);
      assert.equal((await run(["list"], { APIABC_DIR: "", APIX_DIR: unrelatedRoot })).code, 2);
      const preferredList = await run(["list"], { APIBUDDY_DIR: alternate, APIX_DIR: unrelatedRoot });
      assert.equal(preferredList.data.service.id, service.id);
      assert.equal((await run(["list"], { APIBUDDY_DIR: "", APIX_DIR: unrelatedRoot })).code, 2);
      assert.equal((await run(["list"], { APIX_DIR: "" })).code, 2);
    });
  } finally {
    server.closeAllConnections(); await new Promise<void>((resolve) => server.close(() => resolve()));
    rmSync(root, { recursive: true, force: true });
  }
});
