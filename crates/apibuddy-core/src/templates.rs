use crate::error::{Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::LazyLock;
use uuid::Uuid;

pub const METHODS: &[&str] = &["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"];

pub type Method = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pair {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Variable {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequestDraft {
    pub method: Method,
    pub url: String,
    pub query: Vec<Pair>,
    pub headers: Vec<Pair>,
    pub body: String,
    pub variables: Vec<Variable>,
    pub timeout_ms: u64,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateInput {
    pub name: String,
    pub description: String,
    pub request: RequestDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedTemplate {
    pub schema_version: u32,
    pub id: String,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
    pub name: String,
    pub description: String,
    pub request: RequestDraft,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<Pair>,
    pub body: String,
    pub timeout_ms: u64,
    pub stream: bool,
}

static VARIABLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{\s*([A-Za-z_][A-Za-z0-9_.-]*)\s*\}\}").expect("variable regex")
});

pub fn new_template() -> TemplateInput {
    TemplateInput {
        name: "未命名请求".into(),
        description: String::new(),
        request: RequestDraft {
            method: "GET".into(),
            url: "{{base_url}}/get".into(),
            query: vec![],
            headers: vec![Pair {
                key: "Accept".into(),
                value: "application/json".into(),
                enabled: true,
            }],
            body: String::new(),
            variables: vec![Variable {
                key: "base_url".into(),
                value: "https://httpbin.org".into(),
            }],
            timeout_ms: 30_000,
            stream: true,
        },
    }
}

pub fn validate_id(value: &str) -> Result<String> {
    let ok = Uuid::parse_str(value).is_ok();
    if !ok {
        return Err(Error::msg("模板或请求 ID 无效"));
    }
    Ok(value.to_ascii_lowercase())
}

pub fn validate_draft(draft: &RequestDraft) -> Result<RequestDraft> {
    if !METHODS.contains(&draft.method.as_str()) {
        return Err(Error::msg("不支持的 HTTP 方法"));
    }
    if !(1000..=600_000).contains(&draft.timeout_ms) {
        return Err(Error::msg("超时必须为 1–600 秒"));
    }
    if draft.url.len() > 16_384 {
        return Err(Error::msg("URL格式错误或过长"));
    }
    if draft.body.len() > 1024 * 1024 {
        return Err(Error::msg("请求体格式错误或过长"));
    }
    if draft.query.len() > 100 || draft.headers.len() > 100 || draft.variables.len() > 100 {
        return Err(Error::msg("查询参数、请求头或变量最多支持 100 项"));
    }

    let mut keys = HashSet::new();
    let mut variables = Vec::with_capacity(draft.variables.len());
    for row in &draft.variables {
        let key = row.key.trim();
        if key.len() > 100 {
            return Err(Error::msg("变量名格式错误或过长"));
        }
        if !variable_name_ok(key) {
            return Err(Error::msg(
                "变量名需以字母或下划线开头，仅支持字母、数字、点、短横线和下划线",
            ));
        }
        if !keys.insert(key.to_string()) {
            return Err(Error::msg(format!("变量 {key} 重复")));
        }
        if row.value.len() > 16_384 {
            return Err(Error::msg("变量值格式错误或过长"));
        }
        variables.push(Variable {
            key: key.to_string(),
            value: row.value.clone(),
        });
    }

    Ok(RequestDraft {
        method: draft.method.clone(),
        url: draft.url.clone(),
        query: validate_pairs(&draft.query, "查询参数")?,
        headers: validate_pairs(&draft.headers, "请求头")?,
        body: draft.body.clone(),
        variables,
        timeout_ms: draft.timeout_ms,
        stream: draft.stream,
    })
}

pub fn validate_template(input: &TemplateInput) -> Result<TemplateInput> {
    let name = input.name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(Error::msg("请输入模板名称"));
    }
    if input.description.len() > 2000 {
        return Err(Error::msg("描述格式错误或过长"));
    }
    Ok(TemplateInput {
        name: name.to_string(),
        description: input.description.clone(),
        request: validate_draft(&input.request)?,
    })
}

pub fn validate_saved(value: &serde_json::Value) -> Result<SavedTemplate> {
    let saved: SavedTemplate = serde_json::from_value(value.clone())?;
    if saved.schema_version != 1 {
        return Err(Error::msg("不支持的模板文件版本"));
    }
    if saved.revision < 1 {
        return Err(Error::msg("模板修订号无效"));
    }
    validate_id(&saved.id)?;
    let checked = validate_template(&TemplateInput {
        name: saved.name.clone(),
        description: saved.description.clone(),
        request: saved.request.clone(),
    })?;
    Ok(SavedTemplate {
        schema_version: 1,
        id: saved.id.to_ascii_lowercase(),
        revision: saved.revision,
        created_at: saved.created_at,
        updated_at: saved.updated_at,
        name: checked.name,
        description: checked.description,
        request: checked.request,
    })
}

pub fn referenced_variables(draft: &RequestDraft) -> Vec<String> {
    let mut fields = vec![draft.url.as_str()];
    for row in draft.query.iter().filter(|row| row.enabled) {
        fields.push(&row.key);
        fields.push(&row.value);
    }
    for row in draft.headers.iter().filter(|row| row.enabled) {
        fields.push(&row.key);
        fields.push(&row.value);
    }
    if !matches!(draft.method.as_str(), "GET" | "HEAD") {
        fields.push(&draft.body);
    }
    let mut names = BTreeSet::new();
    for field in fields {
        for caps in VARIABLE_RE.captures_iter(field) {
            names.insert(caps[1].to_string());
        }
    }
    names.into_iter().collect()
}

fn validate_pairs(rows: &[Pair], label: &str) -> Result<Vec<Pair>> {
    rows.iter()
        .map(|row| {
            if row.key.len() > 1024 || row.value.len() > 16_384 {
                return Err(Error::msg(format!("{label}格式错误或过长")));
            }
            Ok(Pair {
                key: row.key.clone(),
                value: row.value.clone(),
                enabled: row.enabled,
            })
        })
        .collect()
}

fn variable_name_ok(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

pub(crate) fn substitute(source: &str, variables: &HashMap<String, String>) -> String {
    VARIABLE_RE
        .replace_all(source, |caps: &regex::Captures| {
            variables
                .get(&caps[1])
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_referenced_variables() {
        let draft = new_template().request;
        assert_eq!(referenced_variables(&draft), vec!["base_url".to_string()]);
    }
}
