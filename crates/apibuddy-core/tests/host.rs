use apibuddy_core::{
    host::Host,
    protocol::{Command, Events, Library},
    *,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    time::{Duration, Instant},
};

struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("apibuddy-bridge-{}", uuid::Uuid::new_v4())))
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// Round-trip every command through JSON, just like the WebView boundary.
fn call<T: serde::de::DeserializeOwned>(host: &mut Host, command: Command) -> T {
    let encoded = serde_json::to_string(&command).unwrap();
    serde_json::from_value(host.call(serde_json::from_str(&encoded).unwrap()).unwrap()).unwrap()
}

#[test]
fn both_hosts_read_the_same_files_and_detect_stale_revisions() {
    let root = Sandbox::new();
    let mut native = Host::new(root.0.clone());
    let mut web = Host::new(root.0.clone());
    let mut input = new_template();
    input.name = "跨入口保存".into();
    let saved: SavedTemplate = call(
        &mut native,
        Command::Save {
            service: "default".into(),
            input: input.clone(),
            id: None,
            revision: None,
        },
    );
    let library: Library = call(
        &mut web,
        Command::Load {
            service: "default".into(),
        },
    );
    assert_eq!(library.templates, vec![saved.clone()]);
    input.name = "WebView 修改".into();
    let updated: SavedTemplate = call(
        &mut web,
        Command::Save {
            service: "default".into(),
            input: input.clone(),
            id: Some(saved.id.clone()),
            revision: Some(1),
        },
    );
    assert_eq!(updated.revision, 2);
    assert!(
        native
            .call(Command::Save {
                service: "default".into(),
                input,
                id: Some(saved.id),
                revision: Some(1)
            })
            .is_err()
    );
    let library: Library = call(
        &mut native,
        Command::Load {
            service: "default".into(),
        },
    );
    assert_eq!(library.templates, vec![updated]);
    assert!(
        web.call(Command::Load {
            service: "../../outside".into()
        })
        .is_err()
    );
}

#[test]
fn unacknowledged_request_events_are_retained_and_native_http_has_no_cors_requirement() {
    let root = Sandbox::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/echo", listener.local_addr().unwrap());
    let fixture = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = [0; 8192];
        let n = stream.read(&mut request).unwrap();
        assert!(
            String::from_utf8_lossy(&request[..n])
                .to_lowercase()
                .contains("x-apibuddy: bridge")
        );
        let body = "本地请求✓";
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let mut host = Host::new(root.0.clone());
    let mut input = new_template();
    input.request.url = url;
    input.request.headers.push(Pair {
        key: "X-ApiBuddy".into(),
        value: "bridge".into(),
        enabled: true,
    });
    let id: u64 = call(
        &mut host,
        Command::Start {
            service: "default".into(),
            input,
            environment: None,
        },
    );
    let unrelated: Events = call(&mut host, Command::Poll { operations: vec![] });
    assert!(unrelated.is_empty());
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut body = String::new();
    let mut done = false;
    while !done && Instant::now() < deadline {
        let events: Events = call(
            &mut host,
            Command::Poll {
                operations: vec![id],
            },
        );
        for (_, batch) in events {
            for event in batch {
                match event {
                    Event::Text { text, .. } => body.push_str(&text),
                    Event::End(outcome) => {
                        assert_eq!(outcome, Outcome::Completed);
                        done = true;
                    }
                    _ => {}
                }
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    fixture.join().unwrap();
    assert!(done);
    assert_eq!(body, "本地请求✓");
}

#[test]
fn closing_pending_requests_releases_all_concurrency_slots() {
    let root = Sandbox::new();
    let mut host = Host::new(root.0.clone());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut input = new_template();
    input.request.url = format!("http://{}/slow", listener.local_addr().unwrap());
    for _ in 0..12 {
        let id: u64 = call(
            &mut host,
            Command::Start {
                service: "default".into(),
                input: input.clone(),
                environment: None,
            },
        );
        let _: () = call(
            &mut host,
            Command::Cancel {
                operation: id,
                discard: true,
            },
        );
    }
}
