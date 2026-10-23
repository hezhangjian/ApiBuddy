import { createServer } from "node:http";
const server = createServer((request, response) => {
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
server.listen(42427, "127.0.0.1", () => console.log("ApiBuddy fixture: http://127.0.0.1:42427 (/echo, /slow, /stream)"));
