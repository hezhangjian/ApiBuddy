#!/usr/bin/env python3
"""Local, credential-free HTTP fixture for desktop and CLI verification."""
import json
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit, parse_qs


class Handler(BaseHTTPRequestHandler):
    def _reply(self):
        parsed = urlsplit(self.path)
        if parsed.path == "/slow":
            time.sleep(5)
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
    server = ThreadingHTTPServer(("127.0.0.1", 42427), Handler)
    print("fixture listening on http://127.0.0.1:42427", flush=True)
    server.serve_forever()
