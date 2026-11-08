use apibuddy_core::{
    Error, Result,
    host::Host,
    protocol::{Command, Events},
};
use serde::de::DeserializeOwned;
use std::{path::PathBuf, sync::mpsc};
use tokio::sync::oneshot;

type Job = Box<dyn FnOnce(&mut Host) + Send>;

#[derive(Clone)]
pub struct Backend {
    worker: std::result::Result<mpsc::Sender<Job>, String>,
}

impl Backend {
    pub fn new(root: PathBuf) -> Self {
        let (sender, jobs) = mpsc::channel::<Job>();
        let worker = std::thread::Builder::new()
            .name("apibuddy-storage".into())
            .spawn(move || {
                let mut host = Host::new(root);
                for job in jobs {
                    job(&mut host);
                }
                // Dropping Host cancels its remaining network operations.
            })
            .map(|_| sender)
            .map_err(|error| format!("无法启动本地工作线程：{error}"));
        Self { worker }
    }

    async fn execute<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut Host) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let worker = self.worker.as_ref().map_err(|e| Error::msg(e.clone()))?;
        let (reply, result) = oneshot::channel();
        worker
            .send(Box::new(move |host| {
                let _ = reply.send(action(host));
            }))
            .map_err(|_| Error::msg("本地工作线程已停止，请重启应用"))?;
        result
            .await
            .map_err(|_| Error::msg("本地任务中断，请重试"))?
    }

    pub async fn call<T: DeserializeOwned + Send + 'static>(&self, command: Command) -> Result<T> {
        // File I/O and conversion both stay off the UI thread.
        self.execute(move |host| Ok(serde_json::from_value(host.call(command)?)?))
            .await
    }

    pub async fn poll(&self, operations: Vec<u64>) -> Result<Events> {
        // Response chunks cross threads as owned Rust values, without a JSON round trip.
        self.execute(move |host| Ok(host.poll(&operations))).await
    }
}

pub fn mac_shortcuts() -> bool {
    cfg!(target_os = "macos")
}

pub fn copy_text(text: String) {
    nsh_core::prelude::set_clipboard(text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn queued_storage_work_does_not_block_the_calling_thread() {
        let root = std::env::temp_dir().join(format!("apibuddy-worker-{}", uuid::Uuid::new_v4()));
        let backend = Backend::new(root.clone());
        let caller = std::thread::current().id();
        let (started, ready) = oneshot::channel();
        let (release, wait) = mpsc::channel();
        let job = tokio::spawn(async move {
            backend
                .execute(move |_| {
                    assert_ne!(std::thread::current().id(), caller);
                    let _ = started.send(());
                    wait.recv_timeout(Duration::from_secs(3)).unwrap();
                    Ok(42u32)
                })
                .await
                .unwrap()
        });
        tokio::time::timeout(Duration::from_secs(2), ready)
            .await
            .unwrap()
            .unwrap();
        // This task can continue while the worker is blocked on its own operation.
        assert!(!job.is_finished());
        release.send(()).unwrap();
        assert_eq!(job.await.unwrap(), 42);
        let _ = std::fs::remove_dir_all(root);
    }
}
