use apibuddy_core::{
    protocol::{DraftTab, HistoryEntry, Workspace},
    store::DesktopStore,
    *,
};
use base64::Engine as _;

struct Sandbox(std::path::PathBuf);
impl Sandbox {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("apibuddy-desktop-{}", uuid::Uuid::new_v4())))
    }
    fn store(&self) -> DesktopStore {
        DesktopStore::new(&self.0)
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn auth_and_form_resolve_environment_then_encode_and_export() {
    let mut input = new_template();
    input.request.method = "POST".into();
    input.request.url = "http://localhost/echo".into();
    input.request.headers.push(Pair {
        key: "Authorization".into(),
        value: "Bearer {{unused}}".into(),
        enabled: true,
    });
    input.request.auth = Authentication::Basic {
        username: "{{user}}".into(),
        password: "{{pass}}".into(),
    };
    input.request.form = Some(vec![
        Pair {
            key: "name".into(),
            value: "{{user}} & +".into(),
            enabled: true,
        },
        Pair {
            key: "ignored".into(),
            value: "{{missing}}".into(),
            enabled: false,
        },
    ]);
    let env = Environment {
        id: uuid::Uuid::new_v4().to_string(),
        name: "fixture".into(),
        variables: vec![
            Variable {
                key: "user".into(),
                value: "中文".into(),
            },
            Variable {
                key: "pass".into(),
                value: "test:password".into(),
            },
        ],
    };
    let request = resolve_request(
        &apply_environment(&input.request, Some(&env)).unwrap(),
        true,
    )
    .unwrap();
    let auth = request
        .headers
        .iter()
        .find(|r| r.key == "Authorization")
        .unwrap();
    assert_eq!(
        auth.value,
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("中文:test:password")
        )
    );
    assert_eq!(request.body, "name=%E4%B8%AD%E6%96%87+%26+%2B");
    let imported = curl::import(&curl::export(&request)).unwrap();
    assert_eq!(resolve_request(&imported.request, true).unwrap(), request);
    input.request.auth = Authentication::ApiKey {
        key: "key".into(),
        value: "a + b".into(),
        query: true,
    };
    input.request.url = "http://localhost/echo?key=old&x=1".into();
    input.request.headers.clear();
    input.request.form = None;
    let key = resolve_request(&input.request, true).unwrap();
    assert_eq!(key.url, "http://localhost/echo?x=1&key=a+%2B+b");
    input.request.auth = Authentication::Bearer {
        token: "bad\r\nInjected: value".into(),
    };
    assert!(resolve_request(&input.request, true).is_err());
}

#[test]
fn old_templates_load_and_incomplete_workspace_round_trips_without_overwriting_conflicts() {
    let root = Sandbox::new();
    let store = root.store();
    let mut input = new_template();
    let mut old = serde_json::to_value(&input).unwrap();
    old["request"].as_object_mut().unwrap().remove("auth");
    old["request"].as_object_mut().unwrap().remove("form");
    assert_eq!(serde_json::from_value::<TemplateInput>(old).unwrap(), input);
    input.request.url = "{{unfinished".into();
    input.request.timeout_ms = 0;
    let mut workspace = Workspace::default();
    workspace.tabs.push(DraftTab {
        service: default_service(),
        input: input.clone(),
        baseline: new_template(),
        saved: None,
        environment: None,
        section: 2,
    });
    let stale = workspace.clone();
    let saved = store.save_workspace("native", workspace).unwrap();
    assert_eq!(root.store().workspace("native").unwrap(), saved);
    assert_eq!(saved.tabs[0].input, input);
    assert!(store.save_workspace("native", stale).is_err());
    assert!(store.workspace("../../escape").is_err());
    assert!(store.workspace("webview").unwrap().tabs.is_empty());
    let file = root.0.join("desktop/native-workspace.json");
    std::fs::write(&file, "{corrupt").unwrap();
    assert!(store.save_workspace("native", saved).is_err());
    assert_eq!(std::fs::read_to_string(file).unwrap(), "{corrupt");
}

#[test]
fn history_merges_two_hosts_survives_restart_and_retains_recent_fifty() {
    let root = Sandbox::new();
    let native = root.store();
    let web = root.store();
    for i in 0..55 {
        let entry = HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            service: default_service(),
            input: new_template(),
            environment: None,
            summary: format!("request {i}"),
            completed_at: 0,
        };
        if i % 2 == 0 {
            native.append_history(entry).unwrap();
        } else {
            web.append_history(entry).unwrap();
        }
    }
    let history = root.store().history().unwrap();
    assert_eq!(history.len(), 50);
    assert_eq!(history[0].summary, "request 54");
    assert_eq!(history[49].summary, "request 5");
    assert!(history[0].completed_at > 0);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(root.0.join("desktop/history.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
