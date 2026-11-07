use super::atomic::{WriteLock, atomic_write, ensure_dir, read_limited};
use super::templates::TemplateStore;
use crate::error::{Error, Result};
use crate::paths::data_directory;
use crate::services::{
    Service, ServiceCatalog, ServiceInput, default_service, validate_service, validate_service_id,
    validate_service_input,
};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct ServiceStore {
    pub root: PathBuf,
}

impl ServiceStore {
    pub fn open() -> Result<Self> {
        Ok(Self {
            root: data_directory()?,
        })
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn directory(&self, id: &str) -> Result<PathBuf> {
        let id = validate_service_id(id)?;
        if id == "default" {
            Ok(self.root.clone())
        } else {
            Ok(self.root.join("services").join(id))
        }
    }

    pub fn get(&self, id: &str) -> Result<Service> {
        let id = validate_service_id(id)?;
        if id == "default" {
            return Ok(default_service());
        }
        let services_dir = self.root.join("services");
        ensure_regular_dir(&services_dir)?;
        let directory = self.directory(&id)?;
        ensure_regular_dir(&directory)?;
        let file = directory.join("service.json");
        let text = read_limited(&file, 65_536)?;
        let value: serde_json::Value = serde_json::from_str(&text)?;
        let service = validate_service(&value)?;
        if service.id != id {
            return Err(Error::msg("服务 ID 与目录不一致"));
        }
        Ok(service)
    }

    pub fn list(&self) -> Result<ServiceCatalog> {
        let mut services = vec![default_service()];
        let mut warnings = Vec::new();
        let directory = self.root.join("services");
        if directory.exists() {
            ensure_regular_dir(&directory)?;
            let mut ids: Vec<_> = fs::read_dir(&directory)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| !name.starts_with('.'))
                .collect();
            ids.sort();
            for id in ids {
                if id == "default" {
                    warnings.push("无法读取服务 default，原文件已保留。".into());
                    continue;
                }
                match self.get(&id) {
                    Ok(service) => services.push(service),
                    Err(_) => warnings.push(format!("无法读取服务 {id}，原文件已保留。")),
                }
            }
        }
        services.sort_by(|a, b| match (a.id.as_str(), b.id.as_str()) {
            ("default", _) => std::cmp::Ordering::Less,
            (_, "default") => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
        Ok(ServiceCatalog { services, warnings })
    }

    pub fn create(&self, input: &ServiceInput) -> Result<Service> {
        let input = validate_service_input(input)?;
        let parent = self.root.join("services");
        ensure_dir(&parent)?;
        let _lock = WriteLock::acquire(&parent)?;
        if self
            .list()?
            .services
            .iter()
            .any(|service| service.slug == input.slug)
        {
            return Err(Error::msg("服务标识已存在，请使用不同标识"));
        }
        let service = Service {
            schema_version: 1,
            id: Uuid::new_v4().to_string(),
            name: input.name,
            slug: input.slug,
            description: input.description,
            base_url: input.base_url,
        };
        let directory = self.directory(&service.id)?;
        ensure_dir(&directory)?;
        let serialized = format!("{}\n", serde_json::to_string_pretty(&service)?);
        atomic_write(&directory.join("service.json"), &serialized, 0o600)?;
        Ok(service)
    }

    pub fn templates(&self, service_id: &str) -> Result<TemplateStore> {
        self.get(service_id)?;
        Ok(TemplateStore::new(self.directory(service_id)?))
    }
}

fn ensure_regular_dir(path: &Path) -> Result<()> {
    let meta = path.symlink_metadata()?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(Error::msg("服务库目录不能是符号链接或普通文件"));
    }
    Ok(())
}
