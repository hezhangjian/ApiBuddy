use crate::error::{Error, Result};
use crate::templates::ResolvedRequest;
use reqwest::{
    Client, Method, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::sync::{LazyLock, mpsc};
use std::time::Duration;
use tokio::{runtime::Runtime, sync::oneshot};

pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

pub use crate::protocol::{Event, Outcome};

pub struct Operation {
    cancel: Option<oneshot::Sender<()>>,
}

impl Operation {
    pub fn cancel(&mut self) {
        if let Some(tx) = self.cancel.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub struct Engine {
    runtime: &'static Runtime,
    client: Client,
}

impl Engine {
    pub fn new() -> Self {
        static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("HTTP runtime")
        });
        Self {
            runtime: &RUNTIME,
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("HTTP client"),
        }
    }

    pub fn start(&self, request: ResolvedRequest) -> Result<(Operation, mpsc::Receiver<Event>)> {
        let prepared = PreparedRequest::from_resolved(request)?;
        let (tx, rx) = mpsc::channel();
        let (cancel, cancelled) = oneshot::channel();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let timeout = prepared.timeout;
            let outcome = tokio::select! {
                biased;
                _ = cancelled => Outcome::Cancelled,
                result = tokio::time::timeout(timeout, execute(client, prepared, &tx)) => match result {
                    Ok(Ok(())) => Outcome::Completed,
                    Ok(Err(error)) => Outcome::Failed(error),
                    Err(_) => Outcome::TimedOut,
                }
            };
            let _ = tx.send(Event::End(outcome));
        });
        Ok((
            Operation {
                cancel: Some(cancel),
            },
            rx,
        ))
    }

    /// Blocking helper for CLI: collect the full response.
    pub fn send_blocking(&self, request: ResolvedRequest) -> Result<CollectedResponse> {
        let (mut operation, rx) = self.start(request)?;
        let mut collected = CollectedResponse::default();
        for event in rx {
            match event {
                Event::Headers {
                    status,
                    status_text,
                    headers,
                    url,
                } => {
                    collected.status = Some(status);
                    collected.status_text = status_text;
                    collected.headers = headers;
                    collected.url = url;
                }
                Event::Text { text, bytes } => {
                    collected.body.push_str(&text);
                    collected.bytes = bytes;
                }
                Event::End(outcome) => {
                    collected.outcome = outcome;
                    break;
                }
            }
        }
        operation.cancel();
        Ok(collected)
    }
}

#[derive(Debug, Default)]
pub struct CollectedResponse {
    pub status: Option<u16>,
    pub status_text: String,
    pub headers: String,
    pub url: String,
    pub body: String,
    pub bytes: usize,
    pub outcome: Outcome,
}

struct PreparedRequest {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: String,
    timeout: Duration,
    stream: bool,
}

impl PreparedRequest {
    fn from_resolved(request: ResolvedRequest) -> Result<Self> {
        let method = Method::from_bytes(request.method.as_bytes())
            .map_err(|_| Error::msg("请求方法无效"))?;
        let url = Url::parse(&request.url)
            .map_err(|_| Error::msg("请输入完整的 http:// 或 https:// URL"))?;
        let mut headers = HeaderMap::new();
        for row in request.headers {
            let name = HeaderName::from_bytes(row.key.as_bytes())
                .map_err(|_| Error::msg("请求头名称无效"))?;
            let value =
                HeaderValue::from_str(&row.value).map_err(|_| Error::msg("请求头值无效"))?;
            headers.append(name, value);
        }
        Ok(Self {
            method,
            url,
            headers,
            body: request.body,
            timeout: Duration::from_millis(request.timeout_ms),
            stream: request.stream,
        })
    }
}

async fn execute(
    client: Client,
    request: PreparedRequest,
    tx: &mpsc::Sender<Event>,
) -> std::result::Result<(), String> {
    let response = client
        .request(request.method, request.url)
        .headers(request.headers)
        .body(request.body)
        .send()
        .await
        .map_err(|_| "请求失败，请检查地址、网络、TLS 证书或响应是否中断".to_string())?;

    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();
    let url = response.url().to_string();
    let mut header_lines = Vec::new();
    for (name, value) in response.headers().iter() {
        header_lines.push(format!("{name}: {}", value.to_str().unwrap_or("<binary>")));
    }
    let _ = tx.send(Event::Headers {
        status,
        status_text,
        headers: header_lines.join("\n"),
        url,
    });

    let mut bytes = 0usize;
    let mut buffer = Vec::new();
    let mut decoder = encoding_rs::UTF_8.new_decoder_without_bom_handling();
    let mut response = response;
    loop {
        let chunk = response
            .chunk()
            .await
            .map_err(|_| "请求失败，请检查地址、网络、TLS 证书或响应是否中断".to_string())?;
        let Some(chunk) = chunk else {
            break;
        };
        let remaining = MAX_RESPONSE_BYTES.saturating_sub(bytes);
        let accepted = &chunk[..chunk.len().min(remaining)];
        bytes += accepted.len();
        if request.stream {
            let mut text = String::with_capacity(accepted.len() * 3 + 4);
            let (_, consumed, _) = decoder.decode_to_string(accepted, &mut text, false);
            debug_assert_eq!(consumed, accepted.len());
            if !text.is_empty() {
                let _ = tx.send(Event::Text { text, bytes });
            }
        } else {
            buffer.extend_from_slice(accepted);
        }
        if chunk.len() > remaining {
            if !request.stream {
                let _ = tx.send(Event::Text {
                    text: String::from_utf8_lossy(&buffer).into_owned(),
                    bytes,
                });
            }
            return Err("响应超过 8 MiB，已停止接收并保留已收到的内容".into());
        }
    }
    if !request.stream {
        let _ = tx.send(Event::Text {
            text: String::from_utf8_lossy(&buffer).into_owned(),
            bytes,
        });
    } else {
        let mut text = String::with_capacity(4);
        let _ = decoder.decode_to_string(&[], &mut text, true);
        let _ = tx.send(Event::Text { text, bytes });
    }
    Ok(())
}

impl Default for Outcome {
    fn default() -> Self {
        Self::Failed("未开始".into())
    }
}
