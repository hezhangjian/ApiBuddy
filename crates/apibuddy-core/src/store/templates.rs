use super::atomic::{WriteLock, atomic_write, ensure_dir, read_limited};
use crate::error::{Error, Result};
use crate::templates::{
    SavedTemplate, TemplateInput, validate_id, validate_saved, validate_template,
};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Library {
    pub directory: PathBuf,
    pub templates: Vec<SavedTemplate>,
    pub warnings: Vec<String>,
}

pub struct TemplateStore {
    pub directory: PathBuf,
}

impl TemplateStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            directory: root.as_ref().join("templates"),
        }
    }

    fn initialize(&self) -> Result<()> {
        ensure_dir(&self.directory)
    }

    fn file(&self, id: &str) -> Result<PathBuf> {
        Ok(self.directory.join(format!("{}.json", validate_id(id)?)))
    }

    fn read(&self, id: &str) -> Result<SavedTemplate> {
        let path = self.file(id)?;
        let text = read_limited(&path, MAX_FILE_BYTES)?;
        let value: serde_json::Value = serde_json::from_str(&text)?;
        let saved = validate_saved(&value)?;
        if saved.id != id.to_ascii_lowercase() {
            return Err(Error::msg("文件名与模板 ID 不一致"));
        }
        Ok(saved)
    }

    pub fn list(&self) -> Result<Library> {
        self.initialize()?;
        let mut templates = Vec::new();
        let mut warnings = Vec::new();
        let mut names: Vec<_> = fs::read_dir(&self.directory)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name())
            .collect();
        names.sort();
        for name in names {
            let name = name.to_string_lossy();
            if !name.ends_with(".json") {
                continue;
            }
            let id = &name[..name.len() - 5];
            match self.read(id) {
                Ok(template) => templates.push(template),
                Err(_) => warnings.push(format!("无法读取 {name}；原文件已保留。")),
            }
        }
        templates.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        Ok(Library {
            directory: self.directory.clone(),
            templates,
            warnings,
        })
    }

    pub fn save(
        &self,
        input: &TemplateInput,
        id: Option<&str>,
        revision: Option<u64>,
    ) -> Result<SavedTemplate> {
        let template = validate_template(input)?;
        if let Some(id) = id {
            validate_id(id)?;
        }
        self.initialize()?;
        let _lock = WriteLock::acquire(&self.directory)?;
        let previous = match id {
            Some(id) => Some(self.read(id)?),
            None => None,
        };
        if let (Some(previous), Some(revision)) = (&previous, revision) {
            if previous.revision != revision {
                return Err(Error::msg(
                    "此模板已被其他窗口修改，请刷新后重新打开，或另存为新模板",
                ));
            }
        } else if previous.is_some() && revision.is_none() {
            return Err(Error::msg(
                "此模板已被其他窗口修改，请刷新后重新打开，或另存为新模板",
            ));
        }

        let now = chrono_now();
        let saved = SavedTemplate {
            schema_version: 1,
            id: previous
                .as_ref()
                .map(|item| item.id.clone())
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            revision: previous.as_ref().map(|item| item.revision).unwrap_or(0) + 1,
            created_at: previous
                .as_ref()
                .map(|item| item.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
            name: template.name,
            description: template.description,
            request: template.request,
        };
        let serialized = format!("{}\n", serde_json::to_string_pretty(&saved)?);
        if serialized.len() as u64 > MAX_FILE_BYTES {
            return Err(Error::msg("模板文件不能超过 2 MiB"));
        }
        atomic_write(&self.file(&saved.id)?, &serialized, 0o600)?;
        Ok(saved)
    }

    pub fn delete(&self, id: &str, revision: u64) -> Result<()> {
        self.initialize()?;
        let _lock = WriteLock::acquire(&self.directory)?;
        let previous = self.read(id)?;
        if previous.revision != revision {
            return Err(Error::msg("此模板已被修改，请刷新后再删除"));
        }
        fs::remove_file(self.file(id)?)?;
        Ok(())
    }
}

fn chrono_now() -> String {
    // RFC3339 UTC without extra dependency.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_rfc3339(secs)
}

fn format_rfc3339(secs: u64) -> String {
    // Sufficient for local storage timestamps; not a full calendar implementation.
    const DAYS_PER_400Y: u64 = 146_097;
    const DAYS_PER_100Y: u64 = 36_524;
    const DAYS_PER_4Y: u64 = 1_461;
    let days = secs / 86_400;
    let tod = secs % 86_400;
    let hour = tod / 3600;
    let min = (tod % 3600) / 60;
    let sec = tod % 60;

    let mut y = 1970u64;
    let mut rem = days;
    let cycles400 = rem / DAYS_PER_400Y;
    y += cycles400 * 400;
    rem %= DAYS_PER_400Y;
    let cycles100 = (rem / DAYS_PER_100Y).min(3);
    y += cycles100 * 100;
    rem -= cycles100 * DAYS_PER_100Y;
    let cycles4 = rem / DAYS_PER_4Y;
    y += cycles4 * 4;
    rem %= DAYS_PER_4Y;
    let cycles1 = (rem / 365).min(3);
    y += cycles1;
    rem -= cycles1 * 365;

    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for (i, len) in mdays.iter().enumerate() {
        if rem < *len {
            month = (i + 1) as u64;
            break;
        }
        rem -= *len;
        month = (i + 2) as u64;
    }
    let day = rem + 1;
    format!("{y:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.000Z")
}
