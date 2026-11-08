//! A deliberately small, non-executing POSIX cURL importer/exporter.
use crate::{Authentication, Error, Pair, ResolvedRequest, Result, TemplateInput, new_template};

pub fn export(request: &ResolvedRequest) -> String {
    let mut parts = vec![format!(
        "curl -X {} {}",
        quote(&request.method),
        quote(&request.url)
    )];
    for header in &request.headers {
        let value = if header.value.is_empty() {
            format!("{};", header.key)
        } else {
            format!("{}: {}", header.key, header.value)
        };
        parts.push(format!("-H {}", quote(&value)));
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
    let mut body = String::new();
    let mut has_data = false;
    let mut json = false;
    let mut get = false;
    let mut head = false;
    let mut auth = Authentication::Headers;
    for (flag, value) in arguments(&words[1..])? {
        match flag {
            "--request" => method = Some(value.to_uppercase()),
            "--head" => head = true,
            "--no-head" => head = false,
            "--get" => get = true,
            "--no-get" => get = false,
            "--header" => {
                if value.starts_with('@') {
                    return Err(Error::msg("不读取 cURL 引用的文件；请粘贴请求头"));
                }
                let (key, value) = if let Some((key, value)) = value.split_once(':') {
                    if value.trim().is_empty() {
                        return Err(Error::msg(
                            "暂不支持移除默认请求头；发送空值请使用 -H 'Name;'",
                        ));
                    }
                    (key, value.trim_start())
                } else if let Some(key) = value.strip_suffix(';') {
                    (key, "")
                } else {
                    return Err(Error::msg("请求头应为 Name: Value"));
                };
                input.request.headers.push(Pair {
                    key: key.trim().into(),
                    value: value.into(),
                    enabled: true,
                });
            }
            "--data" | "--data-binary" | "--data-raw" | "--data-urlencode" | "--json" => {
                if flag != "--data-raw" && flag != "--data-urlencode" && value.starts_with('@') {
                    return Err(Error::msg("不读取 cURL 引用的文件；请粘贴正文"));
                }
                has_data = true;
                json |= flag == "--json";
                if !body.is_empty() && flag != "--json" {
                    body.push('&');
                }
                if flag == "--data-urlencode" {
                    body.push_str(&encode_data(value)?);
                } else {
                    body.push_str(value);
                }
            }
            "--user" => {
                let (username, password) = value.split_once(':').ok_or_else(|| {
                    Error::msg("Basic 认证需使用 用户名:密码；导入不支持交互式密码输入")
                })?;
                auth = Authentication::Basic {
                    username: username.into(),
                    password: password.into(),
                };
            }
            "--url" => {
                if !input.request.url.is_empty() {
                    return Err(Error::msg("只支持一个 URL"));
                }
                input.request.url = value.into();
            }
            "--max-time" => {
                let seconds: f64 = value.parse().map_err(|_| Error::msg("超时应为秒数"))?;
                if !seconds.is_finite() || !(1.0..=600.0).contains(&seconds) {
                    return Err(Error::msg("超时必须为 1–600 秒"));
                }
                input.request.timeout_ms = (seconds * 1000.0) as u64;
            }
            "--silent" | "--show-error" | "--globoff" | "--basic" => {}
            _ => unreachable!("arguments only returns supported flags"),
        }
    }
    if input.request.url.is_empty() {
        return Err(Error::msg("cURL 缺少 URL"));
    }
    if head && method.as_ref().is_some_and(|method| method != "HEAD") {
        return Err(Error::msg("暂不支持 --head 与其他显式请求方法组合"));
    }
    input.request.method = method.unwrap_or_else(|| {
        if head {
            "HEAD"
        } else if get || !has_data {
            "GET"
        } else {
            "POST"
        }
        .into()
    });
    if has_data && !get && matches!(input.request.method.as_str(), "GET" | "HEAD") {
        return Err(Error::msg(
            "GET / HEAD 不支持正文；查询参数请使用 -G / --get",
        ));
    }
    if get {
        if !body.is_empty() {
            // Keep existing query spelling, duplicate keys and the fragment intact.
            // Raw delimiters/whitespace would otherwise silently change the request.
            if body
                .chars()
                .any(|c| !c.is_ascii() || c.is_ascii_whitespace() || c.is_control() || c == '#')
            {
                return Err(Error::msg(
                    "GET 查询数据含未编码字符；请使用 --data-urlencode",
                ));
            }
            let (base, fragment) = input
                .request
                .url
                .split_once('#')
                .map_or((input.request.url.as_str(), None), |(base, fragment)| {
                    (base, Some(fragment))
                });
            let separator = if base.contains('?') {
                if base.ends_with('?') || base.ends_with('&') {
                    ""
                } else {
                    "&"
                }
            } else {
                "?"
            };
            input.request.url = format!(
                "{base}{separator}{body}{}",
                fragment.map(|f| format!("#{f}")).unwrap_or_default()
            );
        }
    } else {
        input.request.body = body;
    }
    if !has_header(&input.request.headers, "authorization") {
        input.request.auth = auth;
    }
    if json {
        default_header(
            &mut input.request.headers,
            "Content-Type",
            "application/json",
        );
        default_header(&mut input.request.headers, "Accept", "application/json");
    } else if has_data && !get {
        default_header(
            &mut input.request.headers,
            "Content-Type",
            "application/x-www-form-urlencoded",
        );
    }
    crate::templates::validate_template(&input)
}

fn has_header(headers: &[Pair], name: &str) -> bool {
    headers.iter().any(|h| h.key.eq_ignore_ascii_case(name))
}

fn default_header(headers: &mut Vec<Pair>, key: &str, value: &str) {
    if !has_header(headers, key) {
        headers.push(Pair {
            key: key.into(),
            value: value.into(),
            enabled: true,
        });
    }
}

fn encode_data(value: &str) -> Result<String> {
    let (name, content) = if let Some((name, content)) = value.split_once('=') {
        (name, content)
    } else if value.contains('@') {
        return Err(Error::msg(
            "不读取 cURL 引用的文件；请用 name=内容 或 =内容",
        ));
    } else {
        ("", value)
    };
    let mut encoded = if name.is_empty() {
        String::new()
    } else {
        format!("{name}=")
    };
    const HEX: &[u8] = b"0123456789ABCDEF";
    for byte in content.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            _ => {
                encoded.push('%');
                encoded.push(HEX[(byte >> 4) as usize] as char);
                encoded.push(HEX[(byte & 15) as usize] as char);
            }
        }
    }
    Ok(encoded)
}

// Normalize compact options without executing or expanding any argument.
fn arguments<'a>(words: &'a [String]) -> Result<Vec<(&'static str, &'a str)>> {
    fn option(flag: &str) -> Result<(&'static str, bool)> {
        Ok(match flag {
            "X" | "request" => ("--request", true),
            "H" | "header" => ("--header", true),
            "d" | "data" => ("--data", true),
            "data-raw" => ("--data-raw", true),
            "data-binary" => ("--data-binary", true),
            "data-urlencode" => ("--data-urlencode", true),
            "json" => ("--json", true),
            "u" | "user" => ("--user", true),
            "url" => ("--url", true),
            "m" | "max-time" => ("--max-time", true),
            "I" | "head" => ("--head", false),
            "no-head" => ("--no-head", false),
            "G" | "get" => ("--get", false),
            "no-get" => ("--no-get", false),
            "s" | "silent" => ("--silent", false),
            "S" | "show-error" => ("--show-error", false),
            "g" | "globoff" => ("--globoff", false),
            "basic" => ("--basic", false),
            // Do not echo unknown arguments: they can contain credentials.
            _ => return Err(Error::msg("含有暂不支持的 cURL 参数，请检查导入说明")),
        })
    }
    let mut args = Vec::new();
    let mut words = words.iter();
    let mut positional = false;
    while let Some(word) = words.next() {
        let mut push = |flag, takes_value, inline: Option<&'a str>| -> Result<()> {
            let value = if takes_value {
                inline
                    .or_else(|| words.next().map(String::as_str))
                    .ok_or_else(|| Error::msg(format!("{flag} 缺少值")))?
            } else if inline.is_some() {
                return Err(Error::msg(format!("{flag} 不接受值")));
            } else {
                ""
            };
            args.push((flag, value));
            Ok(())
        };
        if positional || !word.starts_with('-') || word == "-" {
            args.push(("--url", word.as_str()));
        } else if word == "--" {
            positional = true;
        } else if let Some(long) = word.strip_prefix("--") {
            let (name, inline) = long
                .split_once('=')
                .map_or((long, None), |(n, v)| (n, Some(v)));
            if name.len() == 1 {
                return Err(Error::msg("长参数需使用完整名称，如 --request"));
            }
            let (flag, takes_value) = option(name)?;
            push(flag, takes_value, inline)?;
        } else {
            for (offset, c) in word[1..].char_indices() {
                let (flag, takes_value) = option(c.encode_utf8(&mut [0; 4]))?;
                let remaining = &word[1 + offset + c.len_utf8()..];
                push(
                    flag,
                    takes_value,
                    (takes_value && !remaining.is_empty()).then_some(remaining),
                )?;
                if takes_value {
                    break;
                }
            }
        }
    }
    Ok(args)
}

fn tokenize(source: &str) -> Result<Vec<String>> {
    if source.contains('\0') {
        return Err(Error::msg("cURL 不能包含空字符"));
    }
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

    fn resolved(command: &str) -> ResolvedRequest {
        crate::resolve_request(&import(command).unwrap().request, true).unwrap()
    }

    fn header<'a>(request: &'a ResolvedRequest, name: &str) -> Option<&'a str> {
        request
            .headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case(name))
            .map(|h| h.value.as_str())
    }

    #[test]
    fn compact_options_preserve_values_and_empty_headers() {
        let request = resolved(
            "curl -sSgXPOST --url='https://example.com/echo' -H'X-Test: a=b' --header='X-Empty;' -d'a=b' -m1.5",
        );
        assert_eq!(request.method, "POST");
        assert_eq!(request.timeout_ms, 1500);
        assert_eq!(request.body, "a=b");
        assert_eq!(header(&request, "x-test"), Some("a=b"));
        assert_eq!(header(&request, "x-empty"), Some(""));
        assert_eq!(resolved(&export(&request)), request);
        assert_eq!(resolved("curl -sS -- https://example.com").method, "GET");
    }

    #[test]
    fn json_defaults_and_concatenation_match_curl() {
        let request = resolved("curl https://example.com --json '{\"name\":' --json '\"中文\"}'");
        assert_eq!(request.method, "POST");
        assert_eq!(request.body, "{\"name\":\"中文\"}");
        assert_eq!(header(&request, "content-type"), Some("application/json"));
        assert_eq!(header(&request, "accept"), Some("application/json"));
        for (args, expected) in [
            ("-d '' -d b", "b"),
            ("-d a -d ''", "a&"),
            ("-d a --json b", "ab"),
            ("--json a -d b", "a&b"),
        ] {
            assert_eq!(
                resolved(&format!("curl https://example.com {args}")).body,
                expected
            );
        }
        for args in [
            "-H 'accept: text/plain' -H 'content-type: custom/json' --json '{}'",
            "--json '{}' -H 'accept: text/plain' -H 'content-type: custom/json'",
        ] {
            let request = resolved(&format!("curl https://example.com {args}"));
            assert_eq!(header(&request, "accept"), Some("text/plain"));
            assert_eq!(header(&request, "content-type"), Some("custom/json"));
            assert_eq!(request.headers.len(), 2);
        }
    }

    #[test]
    fn urlencode_preserves_field_names_and_encodes_utf8_values() {
        let request = resolved(
            "curl https://example.com --data-urlencode 'q=中文 &+~*' --data-urlencode 'q=' --data-urlencode '=a=b@c' --data-urlencode 'name@file=value' --data-urlencode plain",
        );
        assert_eq!(
            request.body,
            "q=%E4%B8%AD%E6%96%87+%26%2B~%2A&q=&a%3Db%40c&name@file=value&plain"
        );
        assert_eq!(
            header(&request, "content-type"),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(resolved(&export(&request)), request);
    }

    #[test]
    fn get_moves_data_to_query_without_losing_duplicates_or_fragment() {
        let request = resolved(
            "curl 'https://example.com/echo?fixed=%2f#section' -G --data-urlencode 'q=中文 &+' --data-urlencode q=second",
        );
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.url,
            "https://example.com/echo?fixed=%2f&q=%E4%B8%AD%E6%96%87+%26%2B&q=second#section"
        );
        assert!(request.body.is_empty());
        assert!(header(&request, "content-type").is_none());
        assert_eq!(
            resolved("curl https://example.com -IG -d a=1").method,
            "HEAD"
        );
        assert_eq!(
            resolved("curl https://example.com -GXPOST -d a=1").method,
            "POST"
        );
        assert_eq!(
            resolved("curl https://example.com -G --no-get -d a=1").method,
            "POST"
        );
        assert_eq!(
            resolved("curl https://example.com -I --no-head").method,
            "GET"
        );
        for (url, expected) in [
            ("https://example.com/x?", "https://example.com/x?a=1"),
            (
                "https://example.com/x?a=0&",
                "https://example.com/x?a=0&a=1",
            ),
            ("https://example.com/x&", "https://example.com/x&?a=1"),
        ] {
            assert_eq!(resolved(&format!("curl '{url}' -G -d a=1")).url, expected);
        }
    }

    #[test]
    fn basic_auth_uses_last_value_unless_an_explicit_header_overrides_it() {
        let input =
            import("curl https://example.com -uignored:test-only --user='sample:test:only'")
                .unwrap();
        assert_eq!(
            input.request.auth,
            Authentication::Basic {
                username: "sample".into(),
                password: "test:only".into()
            }
        );
        let request = crate::resolve_request(&input.request, true).unwrap();
        assert_eq!(
            header(&request, "authorization"),
            Some("Basic c2FtcGxlOnRlc3Q6b25seQ==")
        );
        for args in [
            "-usample:test-only -H 'Authorization: explicit'",
            "-H 'Authorization: explicit' -usample:test-only",
        ] {
            let input = import(&format!("curl https://example.com {args}")).unwrap();
            assert_eq!(input.request.auth, Authentication::Headers);
            assert_eq!(
                header(
                    &crate::resolve_request(&input.request, true).unwrap(),
                    "authorization"
                ),
                Some("explicit")
            );
        }
    }

    #[test]
    fn rejects_lossy_or_unsupported_imports_without_echoing_values() {
        for args in [
            "-XGET -d a=1",
            "-I -d a=1",
            "-IXPOST",
            "-G -d 'a b'",
            "-G -d 'a#b'",
            "-H 'Accept:'",
            "--json @private-file",
            "--data-urlencode @private-file",
            "--data-urlencode name@private-file",
            "-H @private-file",
            "-usample",
            "--user",
            "-sSX",
            "--get=yes",
            "--X POST",
            "-k",
            "-L",
            "--compressed",
            "--header='X-Test: ok' https://second.example.com",
            "--unknown=private-value",
        ] {
            assert!(
                import(&format!("curl https://example.com {args}")).is_err(),
                "{args}"
            );
        }
        let error = import("curl https://example.com --unknown=private-value")
            .unwrap_err()
            .to_string();
        assert!(!error.contains("private-value"));
        assert!(import("curl https://example.com --json 'a\0b'").is_err());
        assert_eq!(import("curl -sS").unwrap_err().to_string(), "cURL 缺少 URL");
    }

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
