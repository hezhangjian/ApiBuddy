// The only JavaScript owned by ApiBuddy: adapt the NSH WebView message channel.
export function createHost() {
  const token = location.hash.slice(1);
  const pending = new Map();
  let nextId = 0;
  if (!window.nsh?.postMessage || !token) {
    throw new Error("请从 ApiBuddy WebView 桌面程序打开，本页面需要本地 Rust 后端。");
  }
  window.addEventListener("message", event => {
    if (typeof event.data !== "string") return;
    let reply;
    try { reply = JSON.parse(event.data); } catch { return; }
    if (reply.token !== token) return;
    const item = pending.get(reply.id);
    if (!item) return;
    pending.delete(reply.id);
    clearTimeout(item.timer);
    item.resolve(JSON.stringify(reply.result));
  });
  window.apibuddy = Object.freeze({
    call(text) {
      return new Promise((resolve, reject) => {
        if (pending.size >= 64) { reject("本地后端繁忙，请稍后重试"); return; }
        const id = ++nextId;
        const timer = setTimeout(() => { pending.delete(id); reject("本地后端响应超时"); }, 15000);
        pending.set(id, { resolve, reject, timer });
        try {
          window.nsh.postMessage(JSON.stringify({ token, id, command: JSON.parse(text) }));
        } catch (error) {
          pending.delete(id); clearTimeout(timer); reject(String(error));
        }
      });
    },
    copy(text) { window.nsh.postMessage(JSON.stringify({ token, clipboard: text })); },
  });
  return { call() { throw new Error("Unsupported host function"); } };
}
