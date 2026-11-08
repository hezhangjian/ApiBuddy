import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

function fixture() {
  const sent = [], timers = new Map();
  let listener;
  const window = {
    nsh: { postMessage(text) { sent.push(JSON.parse(text)); } },
    addEventListener(name, callback) { if (name === 'message') listener = callback; },
  };
  const context = vm.createContext({ window, location: { hash: '#test-session' },
    setTimeout(fn) { const id = timers.size + 1; timers.set(id, fn); return id; },
    clearTimeout(id) { timers.delete(id); },
  });
  vm.runInContext(readFileSync(new URL('./host.js', import.meta.url), 'utf8').replace('export function', 'function') + '\ncreateHost();', context);
  return { window, sent, timers, reply(data) { listener({ data: JSON.stringify(data) }); } };
}

test('replies match both the session and operation, then release the pending call', async () => {
  const f = fixture();
  const a = f.window.apibuddy.call('{"method":"catalog"}');
  const b = f.window.apibuddy.call('{"method":"load","service":"default"}');
  f.reply({ token: 'wrong-session', id: 1, result: { Ok: 'wrong' } });
  assert.equal(f.timers.size, 2);
  f.reply({ token: 'test-session', id: 2, result: { Ok: 'second' } });
  f.reply({ token: 'test-session', id: 1, result: { Err: 'conflict' } });
  assert.deepEqual(JSON.parse(await b), { Ok: 'second' });
  assert.deepEqual(JSON.parse(await a), { Err: 'conflict' });
  assert.equal(f.timers.size, 0);
  assert.equal(f.sent[1].command.service, 'default');
});

test('a disconnected host rejects instead of leaving the UI waiting indefinitely', async () => {
  const f = fixture();
  const result = f.window.apibuddy.call('{"method":"catalog"}');
  const failure = assert.rejects(result, reason => reason === '本地后端响应超时');
  [...f.timers.values()][0]();
  await failure;
});

test('copy uses the native clipboard channel without sending it to an HTTP endpoint', () => {
  const f = fixture();
  f.window.apibuddy.copy('中文✓');
  assert.equal(f.sent[0].clipboard, '中文✓');
  assert.equal(f.sent[0].token, 'test-session');
});
