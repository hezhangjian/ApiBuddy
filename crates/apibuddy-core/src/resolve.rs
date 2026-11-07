use crate::error::{Error, Result};
use crate::templates::{
    Pair, RequestDraft, ResolvedRequest, referenced_variables, substitute, validate_draft,
};
use std::collections::HashMap;
use url::Url;

pub fn resolve_request(draft: &RequestDraft, substitute_vars: bool) -> Result<ResolvedRequest> {
    let draft = validate_draft(draft)?;
    let variables: HashMap<String, String> = draft
        .variables
        .iter()
        .map(|row| (row.key.clone(), row.value.clone()))
        .collect();
    let missing: Vec<_> = referenced_variables(&draft)
        .into_iter()
        .filter(|key| !variables.contains_key(key))
        .collect();
    if substitute_vars && !missing.is_empty() {
        return Err(Error::msg(format!("未定义变量：{}", missing.join("、"))));
    }

    let replace = |source: &str| {
        if substitute_vars {
            substitute(source, &variables)
        } else {
            source.to_string()
        }
    };

    let mut url =
        Url::parse(replace(&draft.url).trim()).map_err(|_| Error::msg("请输入有效的绝对 URL"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(Error::msg("仅支持 HTTP 和 HTTPS"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::msg(
            "请通过请求头传递认证信息，URL 不支持用户名和密码",
        ));
    }

    for row in draft
        .query
        .iter()
        .filter(|row| row.enabled && !row.key.trim().is_empty())
    {
        url.query_pairs_mut()
            .append_pair(&replace(&row.key), &replace(&row.value));
    }

    let headers = draft
        .headers
        .iter()
        .filter(|row| row.enabled && !row.key.trim().is_empty())
        .map(|row| {
            let key = replace(&row.key).trim().to_string();
            let value = replace(&row.value);
            if !header_name_ok(&key) || value.chars().any(|c| matches!(c, '\r' | '\n' | '\0')) {
                return Err(Error::msg("请求头名称或值无效"));
            }
            Ok(Pair {
                key,
                value,
                enabled: true,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let body = if matches!(draft.method.as_str(), "GET" | "HEAD") {
        String::new()
    } else {
        replace(&draft.body)
    };

    Ok(ResolvedRequest {
        method: draft.method,
        url: url.to_string(),
        headers,
        body,
        timeout_ms: draft.timeout_ms,
        stream: draft.stream,
    })
}

fn header_name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|b| {
            matches!(b,
                b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^'
                | b'_' | b'`' | b'|' | b'~' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'
            )
        })
}
