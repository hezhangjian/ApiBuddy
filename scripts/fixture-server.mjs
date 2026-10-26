import { createServer } from "node:http";
const server = createServer((request, response) => {
  if (request.url.startsWith("/large-json")) {
    response.setHeader("Content-Type", "application/json; charset=utf-8");
    response.end(JSON.stringify({ total: 160, items: Array.from({ length: 160 }, (_, index) => ({ id: index + 1, name: `商品 ${index + 1}`, status: "ready", price: 99.5, active: true, note: null, description: "用于验证长响应搜索、换行和分区滚动。".repeat(4) })) }));
    return;
  }
  if (request.url.startsWith("/empty")) { response.writeHead(204); response.end(); return; }
  if (request.url.startsWith("/stream-many")) {
    response.setHeader("Content-Type", "text/plain; charset=utf-8");
    let line = 0;
    const timer = setInterval(() => {
      response.write(`第 ${++line} 行：流式回显\n`);
      if (line >= 100) { clearInterval(timer); response.end("结束\n"); }
    }, 100);
    response.on("close", () => clearInterval(timer)); return;
  }
  if (request.url.startsWith("/slow")) {
    const timer = setTimeout(() => response.end("finished"), 10000);
    response.on("close", () => clearTimeout(timer)); return;
  }
  if (request.url.startsWith("/stream")) {
    response.setHeader("Content-Type", "text/plain; charset=utf-8");
    response.write("第一段\n");
    const timer = setTimeout(() => response.end("第二段 🙂\n"), 1500);
    response.on("close", () => clearTimeout(timer)); return;
  }
  let body = "";
  request.on("data", (chunk) => { body += chunk; });
  request.on("end", () => {
    response.setHeader("Content-Type", "application/json; charset=utf-8");
    response.end(JSON.stringify({ method: request.method, url: request.url, body, message: "本地回显成功" }));
  });
});
const port = Number(process.env.APIBUDDY_FIXTURE_PORT || 42427);
server.listen(port, "127.0.0.1", () => console.log(`ApiBuddy fixture: http://127.0.0.1:${port} (/echo, /slow, /stream, /large-json, /stream-many, /empty)`));
