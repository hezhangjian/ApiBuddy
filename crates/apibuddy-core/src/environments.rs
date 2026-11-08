use crate::error::{Error, Result};
use crate::templates::{RequestDraft, Variable, referenced_variables, validate_draft, validate_id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: Vec<Variable>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Environments {
    pub schema_version: u32,
    pub revision: u64,
    pub profiles: Vec<Environment>,
}

pub fn empty_environments() -> Environments {
    Environments {
        schema_version: 1,
        revision: 0,
        profiles: vec![],
    }
}

pub fn validate_environments(value: &serde_json::Value) -> Result<Environments> {
    let data: Environments = serde_json::from_value(value.clone())?;
    if data.schema_version != 1 || data.profiles.len() > 30 {
        return Err(Error::msg("环境配置格式错误，最多支持 30 个环境"));
    }
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    let mut profiles = Vec::with_capacity(data.profiles.len());
    for profile in &data.profiles {
        let name = profile.name.trim();
        if name.is_empty()
            || name.len() > 80
            || name.chars().any(|c| matches!(c, '\r' | '\n' | '\0'))
        {
            return Err(Error::msg("环境名称需为 1–80 个字符"));
        }
        let id = validate_id(&profile.id)?;
        if !ids.insert(id.clone()) || !names.insert(name.to_ascii_lowercase()) {
            return Err(Error::msg("环境名称或 ID 重复"));
        }
        let stub = RequestDraft {
            method: "GET".into(),
            url: String::new(),
            query: vec![],
            headers: vec![],
            body: String::new(),
            variables: profile.variables.clone(),
            timeout_ms: 30_000,
            stream: true,
            auth: Default::default(),
            form: None,
            form_draft: None,
        };
        let checked = validate_draft(&stub)?;
        profiles.push(Environment {
            id,
            name: name.to_string(),
            variables: checked.variables,
        });
    }
    Ok(Environments {
        schema_version: 1,
        revision: data.revision,
        profiles,
    })
}

/// Selected environment overrides template defaults for overlapping keys.
pub fn apply_environment(
    draft: &RequestDraft,
    environment: Option<&Environment>,
) -> Result<RequestDraft> {
    let checked = validate_draft(draft)?;
    let Some(environment) = environment else {
        return Ok(checked);
    };
    let allowed: std::collections::HashSet<_> = checked
        .variables
        .iter()
        .map(|row| row.key.clone())
        .chain(referenced_variables(&checked))
        .collect();
    let mut values: std::collections::BTreeMap<_, _> = checked
        .variables
        .iter()
        .map(|row| (row.key.clone(), row.value.clone()))
        .collect();
    for row in &environment.variables {
        if allowed.contains(&row.key) {
            values.insert(row.key.clone(), row.value.clone());
        }
    }
    validate_draft(&RequestDraft {
        variables: values
            .into_iter()
            .map(|(key, value)| Variable { key, value })
            .collect(),
        ..checked
    })
}

pub fn select_environment<'a>(
    data: &'a Environments,
    id: Option<&str>,
) -> Result<Option<&'a Environment>> {
    let Some(id) = id else {
        return Ok(None);
    };
    data.profiles
        .iter()
        .find(|item| item.id == id)
        .map(Some)
        .ok_or_else(|| Error::msg("所选环境已不存在，请重新选择环境后发送"))
}
