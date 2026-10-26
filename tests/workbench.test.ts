import { test } from "node:test";
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync, readFileSync, statSync, symlinkSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { importCurl, exportCurl } from "../src/shared/curl";
import { applyEnvironment, emptyEnvironments, selectEnvironment, validateEnvironments } from "../src/shared/environments";
import { emptyWorkspace, validateWorkspace, type EditorTab } from "../src/shared/workspace";
import { jsonPointer } from "../src/shared/json-view";
import { findText, jsonTokens, nextMatch, SEARCH_LIMIT, textSegments } from "../src/shared/response-text";
import { newTemplate, resolveRequest } from "../src/shared/templates";
import { ServiceStore } from "../src/main/service-store";
import { environmentStore, workspaceStore } from "../src/main/state-store";

test("response search keeps original Unicode offsets and treats punctuation literally", () => {
  const text = 'İ prefix 手机 [a.b] 手机 😀 VALUE value';
  const chinese = findText(text, "手机");
  assert.deepEqual(chinese.ranges.map((range) => text.slice(range.start, range.end)), ["手机", "手机"]);
  assert.equal(chinese.ranges[0].start, text.indexOf("手机"));
  assert.deepEqual(findText(text, "[a.b]").ranges.map((range) => text.slice(range.start, range.end)), ["[a.b]"]);
  assert.deepEqual(findText(text, "value").ranges.map((range) => text.slice(range.start, range.end)), ["VALUE", "value"]);
  assert.equal(findText(text, "😀").ranges[0].end - findText(text, "😀").ranges[0].start, 2);
  assert.deepEqual(findText(text, ""), { ranges: [], truncated: false });
});

test("response search reports its cap accurately and wraps navigation in both directions", () => {
  assert.equal(findText("x".repeat(SEARCH_LIMIT), "x").truncated, false);
  const capped = findText("x".repeat(SEARCH_LIMIT + 1), "x");
  assert.equal(capped.ranges.length, SEARCH_LIMIT); assert.equal(capped.truncated, true);
  assert.equal(nextMatch(2, 3, 1), 0); assert.equal(nextMatch(0, 3, -1), 2);
  assert.equal(nextMatch(0, 0, -1), 0); assert.equal(nextMatch(0, 1, 1), 0);
});

test("JSON syntax and search segmentation preserve every character including escaped strings", () => {
  const text = JSON.stringify({ "a\\\"b": "<script>手機</script> true \\\\", count: -2.5e10, enabled: true, empty: null }, null, 2);
  const tokens = jsonTokens(text);
  assert(tokens.some((token) => token.kind === "key" && text.slice(token.start, token.end) === '"a\\\\\\\"b"'));
  assert.equal(tokens.filter((token) => token.kind === "boolean").length, 1);
  assert.equal(tokens.filter((token) => token.kind === "null").length, 1);
  assert.equal(tokens.filter((token) => token.kind === "number").length, 1);
  const matches = findText(text, '\",\n  "count');
  const segments = textSegments(text, tokens, matches.ranges);
  assert.equal(segments.map((segment) => text.slice(segment.start, segment.end)).join(""), text);
  assert.equal(segments.filter((segment) => segment.match === 0).map((segment) => text.slice(segment.start, segment.end)).join(""), '\",\n  "count');
});

test("large JSON falls back to bounded plain rendering without truncating content", () => {
  const large = JSON.stringify({ value: "a".repeat(256 * 1024) });
  assert.deepEqual(jsonTokens(large), []);
  assert.deepEqual(jsonTokens(JSON.stringify(Array.from({ length: 8001 }, (_, index) => index))), []);
  const segments = textSegments(large, [], findText(large, "value").ranges);
  assert.equal(segments.map((segment) => large.slice(segment.start, segment.end)).join(""), large);
  assert.equal(segments.length, 3);
});

function temporary(run: (root: string) => void) {
  const root = mkdtempSync(join(tmpdir(), "apibuddy-workbench-"));
  try { run(root); } finally { rmSync(root, { recursive: true, force: true }); }
}
test("cURL import preserves quoted JSON, repeated headers, query strings and timeout", () => {
  const source = `curl 'http://127.0.0.1:42427/orders?a=1&a=2' \\\n -X POST -H 'Content-Type: application/json' -H 'X-Tag: one' -H 'X-Tag: two' --data-raw '{"name":"中文", "literal":"$(ignore)"}' --max-time 5`;
  const { template } = importCurl(source);
  const request = resolveRequest(template.request);
  assert.equal(request.method, "POST"); assert.equal(request.timeoutMs, 5000);
  assert.deepEqual(new URL(request.url).searchParams.getAll("a"), ["1", "2"]);
  assert.deepEqual(request.headers.filter((row) => row.key === "X-Tag").map((row) => row.value), ["one", "two"]);
  assert.equal(request.body, '{"name":"中文", "literal":"$(ignore)"}');
  const again = resolveRequest(importCurl(exportCurl(template.request)).template.request);
  assert.deepEqual(again, request);
});
test("cURL import supports inline flags, JSON, literal at-sign, GET data and HEAD", () => {
  const json = importCurl(`curl --url=https://example.test -H'X-Name: a' --json '{"x":1}'`).template.request;
  assert.equal(json.method, "POST"); assert(json.headers.some((row) => row.key === "Content-Type" && row.value === "application/json"));
  assert.equal(importCurl("curl https://example.test --data-raw '@literal'").template.request.body, "@literal");
  const get = resolveRequest(importCurl("curl -G 'https://example.test/?a=0' -d 'a=1' -d 'a=2'").template.request);
  assert.deepEqual(new URL(get.url).searchParams.getAll("a"), ["0", "1", "2"]); assert.equal(get.body, "");
  assert.equal(importCurl("curl -I https://example.test").template.request.method, "HEAD");
  const head = newTemplate().request; head.method = "HEAD";
  assert(exportCurl(head).includes("--head")); assert(!exportCurl(head).includes("--request HEAD"));
  assert.equal(importCurl(exportCurl(head)).template.request.method, "HEAD");
  const plain = newTemplate().request; plain.method = "POST"; plain.body = "hello"; plain.headers = [];
  assert(importCurl(exportCurl(plain)).template.request.headers.some((row) => row.key === "Content-Type" && row.value === "text/plain;charset=UTF-8"));
  const quoted = newTemplate().request; quoted.method = "POST"; quoted.url = "https://example.test";
  quoted.headers = [{ key: "Content-Type", value: "text/plain", enabled: true }]; quoted.variables = []; quoted.body = "single ' and double \" and $HOME; 中文\nnew line";
  assert.equal(importCurl(exportCurl(quoted)).template.request.body, quoted.body);
});
test("unsupported cURL semantics and shell execution are rejected without executing anything", () => {
  for (const source of [
    "curl https://example.test ; touch /tmp/never", "curl https://example.test | sh", "curl $URL", "curl \"https://example.test/$(whoami)\"",
    "curl https://example.test --data @/etc/passwd", "curl https://example.test --data-binary @payload", "curl https://example.test -k",
    "curl https://example.test --config file", "curl https://one.test https://two.test", "curl 'unclosed", "curl https://example.test -F file=@a",
    "curl https://example.test -X GET -d data", "curl https://example.test -m 0", "curl file:///etc/passwd", "curl https://example.test -b cookie.txt",
    "curl https://example.test -H 'X-Token: ok\r\nInjected: true'",
  ]) assert.throws(() => importCurl(source), source);
});
test("environment overrides are isolated, explicit, validated and never mutate templates", () => {
  const draft = newTemplate().request;
  draft.url = "{{base_url}}/{{id}}";
  const profile = { id: randomUUID(), name: "staging", variables: [{ key: "base_url", value: "https://staging.test" }, { key: "id", value: "42" }, { key: "unrelated", value: "unused" }] };
  const data = validateEnvironments({ ...emptyEnvironments(), profiles: [profile] });
  const result = applyEnvironment(draft, selectEnvironment(data, profile.id));
  assert.equal(resolveRequest(result).url, "https://staging.test/42");
  assert(!result.variables.some((row) => row.key === "unrelated"));
  assert.equal(draft.variables[0].value, "https://httpbin.org");
  assert.throws(() => selectEnvironment(data, randomUUID()), /不存在/);
  assert.throws(() => validateEnvironments({ ...data, profiles: [profile, { ...profile, id: randomUUID(), name: "STAGING" }] }), /重复/);
  assert.throws(() => validateEnvironments({ ...data, profiles: [{ ...profile, variables: [{ key: "token", value: 123 }] }] }));
});
test("environment store persists per service and rejects stale writers, corrupt data and symlinks", () => temporary((root) => {
  const services = new ServiceStore(root);
  const a = services.create({ name: "A", slug: "a", baseUrl: "https://a.test", description: "" });
  const b = services.create({ name: "B", slug: "b", baseUrl: "https://b.test", description: "" });
  const store = environmentStore(services, a.id);
  const profile = { id: randomUUID(), name: "local", variables: [{ key: "token", value: "test-only" }] };
  const first = store.save({ ...store.read(), profiles: [profile] });
  assert.equal(first.revision, 1); assert.deepEqual(environmentStore(new ServiceStore(root), a.id).read(), first);
  assert.equal(environmentStore(services, b.id).read().profiles.length, 0);
  assert.throws(() => store.save({ ...first, revision: 0 }), /其他窗口/);
  const file = join(services.directory(a.id), "environments.json");
  assert.equal(statSync(file).mode & 0o777, 0o600);
  writeFileSync(file, "broken"); assert.throws(() => store.save(first), /原文件已保留/); assert.equal(readFileSync(file, "utf8"), "broken");
  symlinkSync(file, join(services.directory(b.id), "environments.json"));
  assert.throws(() => environmentStore(services, b.id).save(emptyEnvironments()), /普通文件/);
  assert(!readdirSync(services.directory(a.id)).some((name) => name.endsWith(".lock")));
}));
test("workspace restores multiple service tabs and unfinished edits without changing templates", () => temporary((root) => {
  const services = new ServiceStore(root), store = workspaceStore(services);
  const input = newTemplate(); input.name = ""; input.request.variables.push({ key: "", value: "still typing" });
  const a: EditorTab = { id: randomUUID(), serviceId: "default", draft: input, baseline: newTemplate(), editorTab: "variables" };
  const b: EditorTab = { ...a, id: randomUUID(), serviceId: randomUUID(), environmentId: randomUUID() };
  const first = store.save({ ...emptyWorkspace(), tabs: [a, b], activeTabId: b.id });
  const restored = workspaceStore(new ServiceStore(root)).read();
  assert.deepEqual(restored, first); assert.equal(restored.tabs[0].draft.request.variables[1].key, "");
  assert.equal(statSync(join(root, "workspace.json")).mode & 0o777, 0o600);
  assert.throws(() => store.save({ ...first, revision: 0 }), /其他窗口/);
  assert.throws(() => validateWorkspace({ ...first, activeTabId: randomUUID() }), /标签不存在/);
  assert.throws(() => validateWorkspace({ ...first, tabs: [a, a] }), /重复/);
  assert.throws(() => validateWorkspace({ ...first, tabs: [{ ...a, serviceId: "../escape" }] }), /ID/);
  assert.throws(() => validateWorkspace({ ...first, tabs: Array.from({ length: 21 }, () => ({ ...a, id: randomUUID() })) }), /标签过多/);
}));
test("JSON pointer handles arrays, escaped keys, null and rejects inherited properties", () => {
  const value = JSON.parse('{"data":[{"name":"中文","nil":null}],"a/b":{"~key":42},"__proto__":{"safe":true}}');
  assert.equal(jsonPointer(value, "/data/0/name"), "中文"); assert.equal(jsonPointer(value, "/data/0/nil"), null);
  assert.equal(jsonPointer(value, "/a~1b/~0key"), 42); assert.equal(jsonPointer(value, "/__proto__/safe"), true);
  for (const path of ["$.data", "/data/01", "/data/length", "/toString", "/data/0/nope", "/a~2b"]) assert.throws(() => jsonPointer(value, path));
});
