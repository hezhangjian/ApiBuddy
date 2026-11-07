use apibuddy_core::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    time::Duration,
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("apibuddy-test-{}", uuid::Uuid::new_v4())))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn service_template_environment_roundtrip_and_conflicts() {
    let root = Temp::new();
    let services = ServiceStore::with_root(&root.0);
    let service = services
        .create(&ServiceInput {
            name: "测试服务".into(),
            slug: "orders".into(),
            description: String::new(),
            base_url: "http://localhost:1234".into(),
        })
        .unwrap();
    let templates = services.templates(&service.id).unwrap();
    let mut input = new_template();
    input.name = "get-order".into();
    input.request.url = "{{base_url}}/orders/{{id}}".into();
    input.request.variables.push(Variable {
        key: "id".into(),
        value: "1".into(),
    });
    let saved = templates.save(&input, None, None).unwrap();
    let updated = templates
        .save(&input, Some(&saved.id), Some(saved.revision))
        .unwrap();
    assert_eq!(updated.revision, 2);
    assert!(templates.save(&input, Some(&saved.id), Some(1)).is_err());
    assert_eq!(templates.list().unwrap().templates, vec![updated]);
    assert!(
        services
            .templates("default")
            .unwrap()
            .list()
            .unwrap()
            .templates
            .is_empty()
    );
    let store = EnvironmentStore::for_service(&services, &service.id).unwrap();
    let mut environments = empty_environments();
    let profile = Environment {
        id: uuid::Uuid::new_v4().to_string(),
        name: "local".into(),
        variables: vec![
            Variable {
                key: "base_url".into(),
                value: service.base_url.clone(),
            },
            Variable {
                key: "id".into(),
                value: "42".into(),
            },
        ],
    };
    environments.profiles.push(profile.clone());
    let written = store.write(&environments).unwrap();
    assert!(store.write(&environments).is_err());
    assert_eq!(store.read().unwrap(), written);
    let request = resolve_request(
        &apply_environment(&input.request, Some(&profile)).unwrap(),
        true,
    )
    .unwrap();
    assert_eq!(request.url, "http://localhost:1234/orders/42");
    assert_eq!(input.request.variables[1].value, "1");
}

fn server(reply: impl FnOnce(&mut std::net::TcpStream) + Send + 'static) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut data = [0; 4096];
        let _ = stream.read(&mut data);
        reply(&mut stream);
    });
    format!("http://{address}/echo")
}

#[test]
fn stream_preserves_split_utf8_and_non_success_responses() {
    let url = server(|stream| {
        stream.write_all(b"HTTP/1.1 422 Unprocessable Entity\r\nTransfer-Encoding: chunked\r\nContent-Type: text/plain\r\n\r\n").unwrap();
        for byte in "中文🙂".as_bytes() {
            stream
                .write_all(&[b'1', b'\r', b'\n', *byte, b'\r', b'\n'])
                .unwrap();
            stream.flush().unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
        stream.write_all(b"0\r\n\r\n").unwrap();
    });
    let mut draft = new_template().request;
    draft.url = url;
    let response = Engine::new()
        .send_blocking(resolve_request(&draft, true).unwrap())
        .unwrap();
    assert_eq!(response.status, Some(422));
    assert_eq!(response.body, "中文🙂");
    assert_eq!(response.bytes, "中文🙂".len());
    assert_eq!(response.outcome, Outcome::Completed);
}

#[test]
fn requests_cancel_and_timeout() {
    let engine = Engine::new();
    for cancel in [true, false] {
        let mut draft = new_template().request;
        draft.url = server(|_| std::thread::sleep(Duration::from_millis(1500)));
        draft.timeout_ms = 1000;
        let (mut operation, events) = engine
            .start(resolve_request(&draft, true).unwrap())
            .unwrap();
        if cancel {
            operation.cancel();
        }
        let event = events.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(
            matches!(event, Event::End(ref outcome) if *outcome == if cancel { Outcome::Cancelled } else { Outcome::TimedOut })
        );
    }
}

#[test]
fn response_limit_keeps_partial_body() {
    let mut draft = new_template().request;
    draft.stream = false;
    draft.url = server(|stream| {
        let size = MAX_RESPONSE_BYTES + 1;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {size}\r\n\r\n").unwrap();
        let _ = stream.write_all(&vec![b'x'; size]);
    });
    let response = Engine::new()
        .send_blocking(resolve_request(&draft, true).unwrap())
        .unwrap();
    assert_eq!(response.body.len(), MAX_RESPONSE_BYTES);
    assert!(matches!(response.outcome, Outcome::Failed(_)));
}
