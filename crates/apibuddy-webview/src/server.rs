//! Private loopback asset server. No API proxy or data-store endpoint is exposed.
use std::{
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use tiny_http::{Header, Method, Response, Server, StatusCode};

pub struct Assets {
    pub url: String,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Assets {
    pub fn start(root: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let root = root.canonicalize()?;
        if !root.join("index.html").is_file() {
            return Err("Web UI index.html is missing".into());
        }
        let server = Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
        let address = server
            .server_addr()
            .to_ip()
            .ok_or("missing listen address")?
            .to_string();
        let prefix = format!("/{}/", uuid::Uuid::new_v4());
        let url = format!("http://{address}{prefix}");
        let nonce = uuid::Uuid::new_v4().to_string();
        let policy = format!(
            "default-src 'self'; script-src 'self' 'nonce-{nonce}' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
        );
        let stop = Arc::new(AtomicBool::new(false));
        let running = stop.clone();
        let thread = thread::spawn(move || {
            while !running.load(Ordering::Relaxed) {
                let Ok(Some(request)) = server.recv_timeout(Duration::from_millis(100)) else {
                    continue;
                };
                let host = request
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Host"))
                    .map(|h| h.value.as_str());
                let relative = request
                    .url()
                    .split('?')
                    .next()
                    .unwrap_or("")
                    .strip_prefix(&prefix);
                let path = relative.and_then(|relative| confined(&root, relative));
                if host != Some(address.as_str())
                    || !matches!(request.method(), &Method::Get | &Method::Head)
                    || path.is_none()
                {
                    let _ = request.respond(Response::empty(StatusCode(404)));
                    continue;
                }
                let path = path.unwrap();
                let Ok(mut data) = std::fs::read(&path) else {
                    let _ = request.respond(Response::empty(StatusCode(404)));
                    continue;
                };
                let mime = match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
                    "html" => "text/html; charset=utf-8",
                    "js" => "text/javascript; charset=utf-8",
                    "wasm" => "application/wasm",
                    "json" | "webmanifest" => "application/json",
                    "css" => "text/css",
                    "svg" => "image/svg+xml",
                    "png" => "image/png",
                    "woff2" => "font/woff2",
                    "ttf" => "font/ttf",
                    _ => "application/octet-stream",
                };
                if path.extension().is_some_and(|e| e == "html") {
                    data = String::from_utf8_lossy(&data)
                        .replace("<script", &format!("<script nonce=\"{nonce}\""))
                        .into_bytes();
                }
                let mut response = Response::from_data(data);
                for (name, value) in [
                    ("Content-Type", mime),
                    ("Cache-Control", "no-store"),
                    ("X-Content-Type-Options", "nosniff"),
                    ("Referrer-Policy", "no-referrer"),
                    ("Content-Security-Policy", policy.as_str()),
                ] {
                    response
                        .add_header(Header::from_bytes(name, value).expect("constant HTTP header"));
                }
                let _ = request.respond(response);
            }
        });
        Ok(Self {
            url,
            stop,
            thread: Some(thread),
        })
    }
}

fn confined(root: &Path, relative: &str) -> Option<PathBuf> {
    let path = Path::new(relative);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let path = root.join(path).canonicalize().ok()?;
    (path.starts_with(root) && path.is_file()).then_some(path)
}

impl Drop for Assets {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn assets_are_private_and_cannot_read_outside_the_bundle() {
        let root = std::env::temp_dir().join(format!("apibuddy-assets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("web")).unwrap();
        std::fs::write(root.join("web/index.html"), "ApiBuddy fixture").unwrap();
        std::fs::write(root.join("outside.txt"), "must not be served").unwrap();
        let assets = Assets::start(root.join("web")).unwrap();
        let url = assets.url.strip_prefix("http://").unwrap();
        let (address, prefix) = url.split_once('/').unwrap();
        let request = |path: String, host: &str| {
            let mut socket = std::net::TcpStream::connect(address).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            write!(
                socket,
                "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            let mut body = String::new();
            socket.read_to_string(&mut body).unwrap();
            body
        };
        assert!(request(format!("/{prefix}index.html"), address).contains("ApiBuddy fixture"));
        for (path, host) in [
            (format!("/{prefix}../outside.txt"), address),
            ("/index.html".into(), address),
            (format!("/{prefix}index.html"), "attacker.invalid"),
        ] {
            let response = request(path, host);
            assert!(response.contains("404"));
            assert!(!response.contains("must not be served"));
        }
        drop(assets);
        std::fs::remove_dir_all(root).unwrap();
    }
}
