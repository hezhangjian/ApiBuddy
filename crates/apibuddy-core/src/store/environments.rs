use super::atomic::{WriteLock, atomic_write, ensure_dir, read_limited};
use super::services::ServiceStore;
use crate::environments::{Environments, empty_environments, validate_environments};
use crate::error::Result;
use std::path::PathBuf;

pub struct EnvironmentStore {
    path: PathBuf,
}

impl EnvironmentStore {
    pub fn for_service(services: &ServiceStore, service_id: &str) -> Result<Self> {
        let directory = services.directory(service_id)?;
        Ok(Self {
            path: directory.join("environments.json"),
        })
    }

    pub fn read(&self) -> Result<Environments> {
        if !self.path.exists() {
            return Ok(empty_environments());
        }
        let text = read_limited(&self.path, 2 * 1024 * 1024)?;
        let value: serde_json::Value = serde_json::from_str(&text)?;
        validate_environments(&value)
    }

    pub fn write(&self, data: &Environments) -> Result<Environments> {
        if let Some(parent) = self.path.parent() {
            ensure_dir(parent)?;
            let _lock = WriteLock::acquire(parent)?;
            let current = if self.path.exists() {
                self.read()?
            } else {
                empty_environments()
            };
            if current.revision != data.revision {
                return Err(crate::error::Error::msg("环境配置已被修改，请刷新后重试"));
            }
            let next = Environments {
                schema_version: 1,
                revision: data.revision + 1,
                profiles: data.profiles.clone(),
            };
            let checked = validate_environments(&serde_json::to_value(&next)?)?;
            let serialized = format!("{}\n", serde_json::to_string_pretty(&checked)?);
            if serialized.len() > 2 * 1024 * 1024 {
                return Err(crate::error::Error::msg("环境文件不能超过 2 MiB"));
            }
            atomic_write(&self.path, &serialized, 0o600)?;
            Ok(checked)
        } else {
            Err(crate::error::Error::msg("环境文件路径无效"))
        }
    }
}
