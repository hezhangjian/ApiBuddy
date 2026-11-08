#!/usr/bin/env python3
"""Local, credential-free HTTP fixture for desktop and CLI verification."""
import json
import argparse
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit, parse_qs


class Handler(BaseHTTPRequestHandler):
    def _reply(self):
        parsed = urlsplit(self.path)
        if parsed.path == "/slow":
            time.sleep(5)
        if parsed.path in ("/inspect", "/large-json"):
            payload = json.dumps({
                "message": "İ 😀中文 Ab aB 中文",
                "items": [{"index": i, "name": f"记录 {i}", "match": "中文命中" if i in (0, 80, 149) else "普通内容"} for i in range(150)],
                "padding": "x" * (270000 if parsed.path == "/large-json" else 0),
                "data": [None, {"a/b": {"~name": "尾部字段中文😀"}}],
            }, ensure_ascii=False, indent=2).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            if self.command != "HEAD":
                self.wfile.write(payload)
            return
        if parsed.path == "/truncated":
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.send_header("Content-Length", "100")
            self.end_headers()
            self.wfile.write("已收到的中文片段".encode())
            self.wfile.flush()
            self.close_connection = True
            return
        if parsed.path == "/stream":
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("Connection", "close")
            self.end_headers()
            for i in range(3):
                self.wfile.write(f'data: {{"消息":"中文回显","序号":{i}}}\n\n'.encode())
                self.wfile.flush()
                time.sleep(0.5)
            self.close_connection = True
            return
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length).decode("utf-8", errors="replace") if length else ""
        payload = json.dumps({
            "method": self.command,
            "path": parsed.path,
            "query": parse_qs(parsed.query, keep_blank_values=True),
            "headers": dict(self.headers),
            "body": body,
            "message": "本地回显成功",
        }, ensure_ascii=False, indent=2).encode()
        status = int(parsed.path.rsplit("/", 1)[1]) if parsed.path.startswith("/status/") else 200
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        if self.command != "HEAD":
            try:
                self.wfile.write(payload)
            except (BrokenPipeError, ConnectionResetError):
                pass

    do_GET = do_POST = do_PUT = do_PATCH = do_DELETE = do_HEAD = do_OPTIONS = _reply

    def log_message(self, fmt, *args):
        # Request headers and bodies are deliberately excluded.
        print(f"[fixture] {self.address_string()} {fmt % args}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=42427, help="监听端口；0 使用随机端口")
    server = ThreadingHTTPServer(("127.0.0.1", parser.parse_args().port), Handler)
    print(f"fixture listening on http://127.0.0.1:{server.server_port}", flush=True)
    server.serve_forever()
