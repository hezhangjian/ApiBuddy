use super::atomic::{WriteLock, atomic_write, ensure_dir, read_limited};
use crate::{
    Error, Result,
    protocol::{HistoryEntry, Workspace},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_WORKSPACE: u64 = 64 * 1024 * 1024;
const MAX_HISTORY: u64 = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct History {
    schema_version: u32,
    entries: Vec<HistoryEntry>,
}

pub struct DesktopStore {
    directory: PathBuf,
}

impl DesktopStore {
    pub fn new(root: &Path) -> Self {
        Self {
            directory: root.join("desktop"),
        }
    }

    fn initialize(&self) -> Result<()> {
        ensure_dir(&self.directory)
    }

    fn workspace_file(&self, client: &str) -> Result<PathBuf> {
        if !matches!(client, "native" | "webview") {
            return Err(Error::msg("未知桌面入口"));
        }
        Ok(self.directory.join(format!("{client}-workspace.json")))
    }

    pub fn workspace(&self, client: &str) -> Result<Workspace> {
        self.initialize()?;
        let path = self.workspace_file(client)?;
        if !path.try_exists()? {
            return Ok(Workspace::default());
        }
        let data: Workspace = serde_json::from_str(&read_limited(&path, MAX_WORKSPACE)?)?;
        Self::check_workspace(&data)?;
        Ok(data)
    }

    fn check_workspace(data: &Workspace) -> Result<()> {
        // Drafts may be incomplete or invalid HTTP. Restore them without normalizing user input.
        if data.schema_version != 1 || data.tabs.len() > 20 {
            return Err(Error::msg("工作区版本错误或标签过多，原文件已保留"));
        }
        crate::services::validate_service_id(&data.selected_service)?;
        for tab in &data.tabs {
            crate::services::validate_service_id(&tab.service.id)?;
            if let Some((id, _)) = &tab.saved {
                crate::templates::validate_id(id)?;
            }
            if serde_json::to_vec(&tab.input)?.len() > 2 * 1024 * 1024 {
                return Err(Error::msg("单个草稿不能超过 2 MiB"));
            }
        }
        Ok(())
    }

    pub fn save_workspace(&self, client: &str, mut data: Workspace) -> Result<Workspace> {
        Self::check_workspace(&data)?;
        self.initialize()?;
        let _lock = WriteLock::acquire(&self.directory)?;
        let previous = self.workspace(client)?;
        if previous.revision != data.revision {
            return Err(Error::msg(
                "另一窗口已更新草稿，自动恢复文件未覆盖；请保存需要保留的请求",
            ));
        }
        data.revision += 1;
        let text = serde_json::to_string(&data)?;
        if text.len() as u64 > MAX_WORKSPACE {
            return Err(Error::msg("工作区超过 64 MiB"));
        }
        atomic_write(&self.workspace_file(client)?, &text, 0o600)?;
        Ok(data)
    }

    pub fn history(&self) -> Result<Vec<HistoryEntry>> {
        self.initialize()?;
        let path = self.directory.join("history.json");
        if !path.try_exists()? {
            return Ok(vec![]);
        }
        let data: History = serde_json::from_str(&read_limited(&path, MAX_HISTORY)?)?;
        if data.schema_version != 1 || data.entries.len() > 50 {
            return Err(Error::msg("历史版本错误或条目过多，原文件已保留"));
        }
        Ok(data.entries)
    }

    pub fn append_history(&self, mut entry: HistoryEntry) -> Result<Vec<HistoryEntry>> {
        self.initialize()?;
        let _lock = WriteLock::acquire(&self.directory)?;
        if serde_json::to_vec(&entry)?.len() > 2 * 1024 * 1024 {
            return Err(Error::msg("单条历史不能超过 2 MiB"));
        }
        let mut entries = self.history()?;
        entry.completed_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        entries.retain(|e| e.id != entry.id);
        entries.insert(0, entry);
        entries.truncate(50);
        let mut data = History {
            schema_version: 1,
            entries,
        };
        loop {
            let text = serde_json::to_string(&data)?;
            if text.len() as u64 <= MAX_HISTORY {
                atomic_write(&self.directory.join("history.json"), &text, 0o600)?;
                return Ok(data.entries);
            }
            data.entries.pop();
        }
    }
}
