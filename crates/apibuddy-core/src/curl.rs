//! A deliberately small, non-executing POSIX cURL importer/exporter.
use crate::{Error, Pair, ResolvedRequest, Result, TemplateInput, new_template};

pub fn export(request: &ResolvedRequest) -> String {
    let mut parts = vec![format!(
        "curl -X {} {}",
        quote(&request.method),
        quote(&request.url)
    )];
    for header in &request.headers {
        parts.push(format!(
            "-H {}",
            quote(&format!("{}: {}", header.key, header.value))
        ));
    }
    if !request.body.is_empty() {
        parts.push(format!("--data-raw {}", quote(&request.body)));
    }
    parts.push(format!("--max-time {}", request.timeout_ms as f64 / 1000.0));
    parts.join(" \\\n  ")
}

pub fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn import(source: &str) -> Result<TemplateInput> {
    if source.len() > 2 * 1024 * 1024 {
        return Err(Error::msg("cURL 不能超过 2 MiB"));
    }
    let words = tokenize(source)?;
    if words.first().map(String::as_str) != Some("curl") {
        return Err(Error::msg("请输入单条 curl 命令"));
    }
    let mut input = new_template();
    input.name = "导入的请求".into();
    input.request.url.clear();
    input.request.headers.clear();
    input.request.variables.clear();
    let mut method = None;
    let mut bodies = Vec::new();
    let mut i = 1;
    while i < words.len() {
        let flag = words[i].as_str();
        i += 1;
        let takes_value = matches!(
            flag,
            "-X" | "--request"
                | "-H"
                | "--header"
                | "-d"
                | "--data"
                | "--data-raw"
                | "--data-binary"
                | "--url"
                | "--max-time"
                | "-m"
        );
        let value = if takes_value {
            let value = words
                .get(i)
                .ok_or_else(|| Error::msg(format!("{flag} 缺少值")))?;
            i += 1;
            value.as_str()
        } else {
            ""
        };
        match flag {
            "-X" | "--request" => method = Some(value.to_uppercase()),
            "-I" | "--head" => method = Some("HEAD".into()),
            "-H" | "--header" => {
                let (key, value) = value
                    .split_once(':')
                    .ok_or_else(|| Error::msg("请求头应为 Name: Value"))?;
                input.request.headers.push(Pair {
                    key: key.trim().into(),
                    value: value.trim_start().into(),
                    enabled: true,
                });
            }
            "-d" | "--data" | "--data-binary" | "--data-raw" => {
                if flag != "--data-raw" && value.starts_with('@') {
                    return Err(Error::msg("不读取 cURL 引用的文件；请粘贴正文"));
                }
                bodies.push(value.to_string());
            }
            "--url" => {
                if !input.request.url.is_empty() {
                    return Err(Error::msg("只支持一个 URL"));
                }
                input.request.url = value.into();
            }
            "--max-time" | "-m" => {
                let seconds: f64 = value.parse().map_err(|_| Error::msg("超时应为秒数"))?;
                if !seconds.is_finite() || !(1.0..=600.0).contains(&seconds) {
                    return Err(Error::msg("超时必须为 1–600 秒"));
                }
                input.request.timeout_ms = (seconds * 1000.0) as u64;
            }
            "-s" | "--silent" | "-S" | "--show-error" => {}
            flag if !flag.starts_with('-') && input.request.url.is_empty() => {
                input.request.url = flag.into()
            }
            _ => return Err(Error::msg(format!("暂不支持 cURL 参数：{flag}"))),
        }
    }
    input.request.method =
        method.unwrap_or_else(|| if bodies.is_empty() { "GET" } else { "POST" }.into());
    input.request.body = bodies.join("&");
    if !bodies.is_empty()
        && !input
            .request
            .headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("content-type"))
    {
        input.request.headers.push(Pair {
            key: "Content-Type".into(),
            value: "application/x-www-form-urlencoded".into(),
            enabled: true,
        });
    }
    crate::templates::validate_template(&input)?;
    if input.request.url.is_empty() {
        return Err(Error::msg("cURL 缺少 URL"));
    }
    Ok(input)
}

fn tokenize(source: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some('\'') => {
                if c == '\'' {
                    quote = None
                } else {
                    word.push(c)
                }
            }
            Some('"') => match c {
                '"' => quote = None,
                '$' | '`' => return Err(Error::msg("不支持 shell 展开；请使用单引号包裹字面值")),
                '\\' => match chars.next() {
                    Some('\n') => {}
                    Some(next @ ('"' | '\\' | '$' | '`')) => word.push(next),
                    Some(next) => {
                        word.push('\\');
                        word.push(next);
                    }
                    None => return Err(Error::msg("cURL 转义未完成")),
                },
                _ => word.push(c),
            },
            _ => match c {
                '\'' | '"' => {
                    quote = Some(c);
                    started = true;
                }
                '\\' => match chars.next() {
                    Some('\n') => {}
                    Some(next) => {
                        word.push(next);
                        started = true;
                    }
                    None => return Err(Error::msg("cURL 转义未完成")),
                },
                '|' | '&' | ';' | '<' | '>' | '$' | '`' | '(' | ')' => {
                    return Err(Error::msg(
                        "仅支持单条 cURL，不支持 shell 运算或展开；URL 请加引号",
                    ));
                }
                c if c.is_whitespace() => {
                    if started {
                        words.push(std::mem::take(&mut word));
                        started = false;
                    }
                }
                _ => {
                    word.push(c);
                    started = true;
                }
            },
        }
    }
    if quote.is_some() {
        return Err(Error::msg("cURL 引号未闭合"));
    }
    if started {
        words.push(word);
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_preserves_headers_body_and_query() {
        let original = import("curl 'http://localhost/a?q=1&q=2' -X POST -H 'Content-Type: application/json' -H 'X-Test: hello' --data-raw '{\"name\":\"中文\",\"quote\":\"it'\\''s\"}'").unwrap();
        let resolved = crate::resolve_request(&original.request, true).unwrap();
        let again = import(&export(&resolved)).unwrap();
        assert_eq!(
            resolved,
            crate::resolve_request(&again.request, true).unwrap()
        );
    }
    #[test]
    fn rejects_shell_and_file_access() {
        for command in [
            "curl https://example.com | sh",
            "curl $(whoami)",
            "curl x -d @secret",
            "curl x -k",
            "curl 'unterminated",
            "curl x --max-time NaN",
            "curl x https://other.com",
        ] {
            assert!(import(command).is_err(), "{command}");
        }
        assert_eq!(
            import("curl 'https://example.com' --data-raw '@literal'")
                .unwrap()
                .request
                .body,
            "@literal"
        );
    }
}
