use crate::Authentication;
use crate::error::{Error, Result};
use crate::templates::{
    Pair, RequestDraft, ResolvedRequest, referenced_variables, substitute, validate_draft,
};
use base64::Engine as _;
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

    let mut header_rows = draft.headers.clone();
    let auth_header = match &draft.auth {
        Authentication::Headers => None,
        Authentication::None => {
            header_rows.retain(|r| !r.key.eq_ignore_ascii_case("authorization"));
            None
        }
        Authentication::Bearer { token } => {
            Some(("Authorization".into(), format!("Bearer {}", replace(token))))
        }
        Authentication::Basic { username, password } => {
            let username = replace(username);
            if username.contains(':') {
                return Err(Error::msg("Basic 用户名不能包含冒号"));
            }
            Some((
                "Authorization".into(),
                format!(
                    "Basic {}",
                    base64::engine::general_purpose::STANDARD
                        .encode(format!("{username}:{}", replace(password)))
                ),
            ))
        }
        Authentication::ApiKey { key, value, query } => {
            let key = replace(key);
            if key.trim().is_empty() {
                return Err(Error::msg("请输入 API Key 名称"));
            }
            if *query {
                let pairs: Vec<(String, String)> = url
                    .query_pairs()
                    .filter(|(k, _)| k != &key)
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect();
                url.set_query(None);
                url.query_pairs_mut()
                    .extend_pairs(pairs)
                    .append_pair(&key, &replace(value));
                None
            } else {
                Some((key, replace(value)))
            }
        }
    };
    if let Some((key, value)) = auth_header {
        header_rows.retain(|r| !r.key.eq_ignore_ascii_case(&key));
        header_rows.push(Pair {
            key,
            value,
            enabled: true,
        });
    }
    if draft.form.is_some() && !matches!(draft.method.as_str(), "GET" | "HEAD") {
        header_rows.retain(|r| !r.key.eq_ignore_ascii_case("content-type"));
        header_rows.push(Pair {
            key: "Content-Type".into(),
            value: "application/x-www-form-urlencoded".into(),
            enabled: true,
        });
    }
    let headers = header_rows
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
    } else if let Some(form) = &draft.form {
        let mut encoded = url::form_urlencoded::Serializer::new(String::new());
        for row in form
            .iter()
            .filter(|r| r.enabled && !r.key.trim().is_empty())
        {
            encoded.append_pair(&replace(&row.key), &replace(&row.value));
        }
        encoded.finish()
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
