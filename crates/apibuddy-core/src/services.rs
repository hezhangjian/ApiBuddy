use crate::error::{Error, Result};
use crate::templates::validate_id;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInput {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCatalog {
    pub services: Vec<Service>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CliConfig {
    pub data_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBundle {
    pub cli_path: String,
    pub markdown_path: String,
    pub markdown: String,
    pub service_id: String,
    pub setup: String,
    pub environment_name: Option<String>,
    pub commands: Vec<AgentCommand>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommand {
    pub name: String,
    pub method: String,
    pub description: String,
    pub example: String,
}

pub fn default_service() -> Service {
    Service {
        schema_version: 1,
        id: "default".into(),
        name: "我的集合".into(),
        slug: "default".into(),
        description: "原有请求模板".into(),
        base_url: "https://httpbin.org".into(),
    }
}

pub fn validate_service_id(value: &str) -> Result<String> {
    if value == "default" {
        return Ok(value.to_string());
    }
    validate_id(value)
}

pub fn validate_service_input(input: &ServiceInput) -> Result<ServiceInput> {
    let name = input.name.trim();
    let slug = input.slug.trim();
    let description = input.description.trim();
    let base_url = input.base_url.trim();
    if name.is_empty() || name.len() > 120 || name.chars().any(|c| matches!(c, '\r' | '\n' | '\0'))
    {
        return Err(Error::msg("服务名称不能为空或包含换行"));
    }
    if slug.len() > 48 || !slug_ok(slug) {
        return Err(Error::msg(
            "服务标识使用小写字母、数字和短横线，并以字母开头",
        ));
    }
    if description.len() > 2000 {
        return Err(Error::msg("服务 description 格式错误或过长"));
    }
    if base_url.len() > 16_384 {
        return Err(Error::msg("服务 baseUrl 格式错误或过长"));
    }
    if !base_url.is_empty() {
        let url =
            Url::parse(base_url).map_err(|_| Error::msg("服务地址需为有效的 HTTP/HTTPS URL"))?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(Error::msg("服务地址仅支持不含用户名密码的 HTTP/HTTPS URL"));
        }
    }
    Ok(ServiceInput {
        name: name.to_string(),
        slug: slug.to_string(),
        description: description.to_string(),
        base_url: base_url.to_string(),
    })
}

pub fn validate_service(value: &serde_json::Value) -> Result<Service> {
    let service: Service = serde_json::from_value(value.clone())?;
    if service.schema_version != 1 {
        return Err(Error::msg("不支持的服务文件版本"));
    }
    let input = validate_service_input(&ServiceInput {
        name: service.name.clone(),
        slug: service.slug.clone(),
        description: service.description.clone(),
        base_url: service.base_url.clone(),
    })?;
    Ok(Service {
        schema_version: 1,
        id: validate_service_id(&service.id)?,
        name: input.name,
        slug: input.slug,
        description: input.description,
        base_url: input.base_url,
    })
}

fn slug_ok(slug: &str) -> bool {
    let mut chars = slug.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
