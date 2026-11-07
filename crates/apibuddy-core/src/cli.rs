use crate::environments::{Environment, apply_environment};
use crate::error::{Error, Result};
use crate::http::{Engine, Outcome};
use crate::resolve::resolve_request;
use crate::services::Service;
use crate::store::{EnvironmentStore, ServiceStore};
use crate::templates::{
    METHODS, Method, Pair, RequestDraft, SavedTemplate, Variable, new_template,
    referenced_variables,
};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use url::Url;

pub fn service_commands(templates: &[SavedTemplate]) -> Vec<(String, SavedTemplate)> {
    let reserved = ["help", "list", "request", "run", "show", "environments"];
    let stems: Vec<String> = templates
        .iter()
        .map(|template| {
            let mut stem = slugify_command(&template.name);
            if stem.is_empty() {
                stem = "api".into();
            }
            if reserved.contains(&stem.as_str()) {
                stem = format!("api-{stem}");
            }
            stem
        })
        .collect();
    let mut counts = HashMap::<String, usize>::new();
    for stem in &stems {
        *counts.entry(stem.clone()).or_default() += 1;
    }
    let mut used = HashSet::new();
    templates
        .iter()
        .zip(stems)
        .map(|(template, mut name)| {
            if counts.get(&name).copied().unwrap_or(0) > 1 {
                name = format!("{name}-{}", template.id);
                while used.contains(&name) {
                    name.push_str("-api");
                }
            }
            used.insert(name.clone());
            (name, template.clone())
        })
        .collect()
}

fn slugify_command(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in name.trim().chars() {
        if ch.is_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.extend(ch.to_lowercase());
            dash = false;
        } else if !dash {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

pub fn command_variables(template: &SavedTemplate) -> Vec<CommandVariable> {
    let defaults: HashMap<_, _> = template
        .request
        .variables
        .iter()
        .map(|row| (row.key.clone(), row.value.clone()))
        .collect();
    let mut names = referenced_variables(&template.request);
    for key in defaults.keys() {
        if !names.contains(key) {
            names.push(key.clone());
        }
    }
    names.sort();
    names
        .into_iter()
        .map(|name| CommandVariable {
            name: name.clone(),
            has_default: defaults.get(&name).is_some_and(|value| !value.is_empty()),
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct CommandVariable {
    pub name: String,
    pub has_default: bool,
}

pub fn reserved_options() -> HashSet<&'static str> {
    [
        "help",
        "dry-run",
        "environment",
        "env",
        "var",
        "vars-file",
        "data",
        "header",
        "query",
    ]
    .into_iter()
    .collect()
}

pub fn command_example(executable: &str, name: &str, template: &SavedTemplate) -> String {
    let mut parts = vec![executable.to_string(), shell_quote(name)];
    for variable in command_variables(template) {
        if reserved_options().contains(variable.name.as_str()) {
            parts.push(format!("--var {}=<值>", variable.name));
        } else if variable.has_default {
            parts.push(format!("[--{} <值>]", variable.name));
        } else {
            parts.push(format!("--{} <值>", variable.name));
        }
    }
    parts.join(" ")
}

pub fn run(args: &[String]) -> i32 {
    match run_inner(args) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error}");
            1
        }
    }
}

fn run_inner(args: &[String]) -> Result<i32> {
    let services = ServiceStore::open()?;
    let catalog = services.list()?;
    if args.is_empty() || (args.len() == 1 && (args[0] == "--help" || args[0] == "-h")) {
        print_services(&catalog);
        return Ok(0);
    }

    let slug = &args[0];
    let matches: Vec<_> = catalog
        .services
        .iter()
        .filter(|service| service.slug == *slug)
        .cloned()
        .collect();
    if matches.is_empty() {
        return Err(Error::msg(format!("未知服务标识 {slug}")));
    }
    if matches.len() > 1 {
        return Err(Error::msg(format!("服务标识 {slug} 重复")));
    }
    let service = matches.into_iter().next().unwrap();
    let rest = &args[1..];
    if rest.is_empty() || (rest.len() == 1 && (rest[0] == "--help" || rest[0] == "-h")) {
        let library = services.templates(&service.id)?.list()?;
        print_help(&service, &library.templates, &library.warnings);
        return Ok(0);
    }

    if rest[0] == "environments" {
        let profiles = EnvironmentStore::for_service(&services, &service.id)?.read()?;
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "serviceId": service.id,
                "environments": profiles.profiles.iter().map(|profile| json!({
                    "id": profile.id,
                    "name": profile.name,
                    "variables": profile.variables,
                })).collect::<Vec<_>>(),
            }))?
        );
        return Ok(0);
    }

    if rest[0] == "request" {
        let (flags, environment) = environment_options(&rest[1..], &services, &service.id)?;
        let (draft, dry_run) = raw_request(&service, &flags)?;
        return execute(&service, draft, dry_run, None, environment.as_ref());
    }

    let library = services.templates(&service.id)?.list()?;
    let endpoint = &rest[0];
    let commands = service_commands(&library.templates);
    let entry = commands
        .into_iter()
        .find(|(name, _)| name == endpoint)
        .ok_or_else(|| {
            Error::msg(format!(
                "没有接口 {endpoint}，执行 apibuddy {} --help 查看全部接口",
                service.slug
            ))
        })?;
    let (command_name, template) = entry;
    if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_endpoint_help(&service, &command_name, &template);
        return Ok(0);
    }
    let (flags, environment) = environment_options(&rest[1..], &services, &service.id)?;
    let options = options(&flags, &template, environment.as_ref())?;
    let mut draft = template.request.clone();
    draft.variables = options.variables;
    execute(
        &service,
        draft,
        options.dry_run,
        Some(template.id),
        environment.as_ref(),
    )
}

fn execute(
    service: &Service,
    draft: RequestDraft,
    dry_run: bool,
    template_id: Option<String>,
    environment: Option<&Environment>,
) -> Result<i32> {
    let draft = apply_environment(&draft, environment)?;
    let resolved = resolve_request(&draft, template_id.is_some())?;
    if dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "dryRun": true,
                "serviceId": service.id,
                "templateId": template_id,
                "request": {
                    "method": resolved.method,
                    "url": resolved.url,
                    "headers": resolved.headers,
                    "body": resolved.body,
                    "timeoutMs": resolved.timeout_ms,
                    "stream": resolved.stream,
                }
            }))?
        );
        return Ok(0);
    }
    let engine = Engine::new();
    let response = engine.send_blocking(resolved)?;
    let ok = matches!(response.outcome, Outcome::Completed);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": ok,
            "serviceId": service.id,
            "templateId": template_id,
            "status": response.status,
            "statusText": response.status_text,
            "url": response.url,
            "headers": response.headers,
            "body": response.body,
            "bytes": response.bytes,
            "outcome": match response.outcome {
                Outcome::Completed => "completed",
                Outcome::Cancelled => "cancelled",
                Outcome::TimedOut => "timed-out",
                Outcome::Failed(_) => "failed",
            },
            "error": match &response.outcome {
                Outcome::Failed(message) => Some(message.clone()),
                _ => None,
            }
        }))?
    );
    Ok(if ok { 0 } else { 2 })
}

struct Options {
    variables: Vec<Variable>,
    dry_run: bool,
}

fn options(
    args: &[String],
    template: &SavedTemplate,
    environment: Option<&Environment>,
) -> Result<Options> {
    let draft = apply_environment(&template.request, environment)?;
    let mut values: HashMap<_, _> = draft
        .variables
        .iter()
        .map(|row| (row.key.clone(), row.value.clone()))
        .collect();
    let allowed: HashSet<_> = values
        .keys()
        .cloned()
        .chain(referenced_variables(&template.request))
        .collect();
    let mut dry_run = false;
    let mut index = 0;
    while index < args.len() {
        let flag = &args[index];
        if flag == "--dry-run" {
            dry_run = true;
            index += 1;
            continue;
        }
        if !matches!(flag.as_str(), "--var" | "--env" | "--vars-file") {
            if !flag.starts_with("--") {
                return Err(Error::msg(format!("未知选项 {flag}")));
            }
            let (key, value) =
                if let Some((key, value)) = flag.strip_prefix("--").unwrap().split_once('=') {
                    (key.to_string(), value.to_string())
                } else {
                    let key = flag[2..].to_string();
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| {
                            Error::msg(format!("{flag} 缺少值；以 -- 开头的值请使用 --{key}=值"))
                        })?
                        .clone();
                    if value.starts_with("--") {
                        return Err(Error::msg(format!(
                            "{flag} 缺少值；以 -- 开头的值请使用 --{key}=值"
                        )));
                    }
                    (key, value)
                };
            set_var(&mut values, &allowed, &key, &value)?;
            index += 1;
            continue;
        }
        index += 1;
        let value = args
            .get(index)
            .ok_or_else(|| Error::msg(format!("{flag} 缺少值")))?;
        if flag == "--vars-file" {
            let meta = fs::symlink_metadata(value)?;
            if !meta.is_file() || meta.len() > 65_536 {
                return Err(Error::msg("变量文件需为不超过 64 KiB 的普通 JSON 文件"));
            }
            let data: serde_json::Value = serde_json::from_str(&fs::read_to_string(value)?)?;
            let object = data
                .as_object()
                .ok_or_else(|| Error::msg("变量文件需为字符串键值对象"))?;
            for (key, entry) in object {
                let entry = entry
                    .as_str()
                    .ok_or_else(|| Error::msg(format!("变量 {key} 必须是字符串")))?;
                set_var(&mut values, &allowed, key, entry)?;
            }
        } else {
            let (key, entry) = key_value(value)?;
            if flag == "--env" {
                if !env_name_ok(&entry) {
                    return Err(Error::msg("环境变量名称无效"));
                }
                let from_env =
                    env::var(&entry).map_err(|_| Error::msg(format!("环境变量 {entry} 未设置")))?;
                set_var(&mut values, &allowed, &key, &from_env)?;
            } else {
                set_var(&mut values, &allowed, &key, &entry)?;
            }
        }
        index += 1;
    }
    Ok(Options {
        variables: values
            .into_iter()
            .map(|(key, value)| Variable { key, value })
            .collect(),
        dry_run,
    })
}

fn set_var(
    values: &mut HashMap<String, String>,
    allowed: &HashSet<String>,
    key: &str,
    value: &str,
) -> Result<()> {
    if !allowed.contains(key) {
        return Err(Error::msg(format!("模板没有变量 {key}")));
    }
    values.insert(key.to_string(), value.to_string());
    Ok(())
}

fn environment_options(
    args: &[String],
    services: &ServiceStore,
    service_id: &str,
) -> Result<(Vec<String>, Option<Environment>)> {
    let mut flags = Vec::new();
    let mut name: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--environment" || arg.starts_with("--environment=") {
            if name.is_some() {
                return Err(Error::msg("只可选择一个环境"));
            }
            name = Some(if arg == "--environment" {
                index += 1;
                args.get(index)
                    .cloned()
                    .ok_or_else(|| Error::msg("--environment 缺少环境名称"))?
            } else {
                arg["--environment=".len()..].to_string()
            });
            if name
                .as_ref()
                .is_some_and(|value| value.is_empty() || value.starts_with("--"))
            {
                return Err(Error::msg("--environment 缺少环境名称"));
            }
        } else {
            flags.push(arg.clone());
            if matches!(
                arg.as_str(),
                "--var" | "--env" | "--vars-file" | "--data" | "--header" | "--query"
            ) && index + 1 < args.len()
            {
                index += 1;
                flags.push(args[index].clone());
            }
        }
        index += 1;
    }
    let Some(name) = name else {
        return Ok((flags, None));
    };
    let profiles = EnvironmentStore::for_service(services, service_id)?.read()?;
    let environment = profiles
        .profiles
        .into_iter()
        .find(|item| item.name == name || item.id == name)
        .ok_or_else(|| Error::msg(format!("没有环境 {name}，执行 environments 查看可用环境")))?;
    Ok((flags, Some(environment)))
}

fn raw_request(service: &Service, args: &[String]) -> Result<(RequestDraft, bool)> {
    let method = args.first().ok_or_else(|| {
        Error::msg(format!(
            "用法：request GET /path；方法支持 {}",
            METHODS.join(", ")
        ))
    })?;
    let path = args
        .get(1)
        .ok_or_else(|| Error::msg("用法：request GET /path；接口路径需以 / 开头"))?;
    if !METHODS.contains(&method.as_str()) {
        return Err(Error::msg(format!(
            "用法：request GET /path；方法支持 {}",
            METHODS.join(", ")
        )));
    }
    if service.base_url.is_empty() {
        return Err(Error::msg("此服务尚未设置默认地址，请使用已保存的接口命令"));
    }
    if !path.starts_with('/') || path.starts_with("//") || path.contains('\\') {
        return Err(Error::msg("接口路径需以 / 开头，例如 /orders"));
    }
    let base = Url::parse(&service.base_url)
        .map_err(|_| Error::msg("服务地址需为有效的 HTTP/HTTPS URL"))?;
    let prefix = base.path().trim_end_matches('/');
    let url = Url::parse(&format!(
        "{}{}{}",
        base.origin().ascii_serialization(),
        prefix,
        path
    ))
    .map_err(|_| Error::msg("接口路径无效"))?;
    if url.origin() != base.origin() {
        return Err(Error::msg("接口路径不能更换服务地址"));
    }
    let mut draft = new_template().request;
    draft.method = method.clone();
    draft.url = url.to_string();
    draft.variables.clear();
    let mut dry_run = false;
    let mut index = 2;
    while index < args.len() {
        let flag = &args[index];
        if flag == "--dry-run" {
            dry_run = true;
            index += 1;
            continue;
        }
        if !matches!(flag.as_str(), "--data" | "--header" | "--query") {
            return Err(Error::msg(format!("未知选项 {flag}")));
        }
        index += 1;
        let value = args
            .get(index)
            .ok_or_else(|| Error::msg(format!("{flag} 缺少值")))?;
        match flag.as_str() {
            "--data" => draft.body = value.clone(),
            "--query" => {
                let (key, entry) = key_value(value)?;
                draft.query.push(Pair {
                    key,
                    value: entry,
                    enabled: true,
                });
            }
            "--header" => {
                let separator = value
                    .find(':')
                    .filter(|index| *index > 0)
                    .ok_or_else(|| Error::msg("请求头使用 '名称: 值' 格式"))?;
                draft.headers.push(Pair {
                    key: value[..separator].trim().to_string(),
                    value: value[separator + 1..].trim().to_string(),
                    enabled: true,
                });
            }
            _ => {}
        }
        index += 1;
    }
    if matches!(method.as_str(), "GET" | "HEAD") && !draft.body.is_empty() {
        return Err(Error::msg(format!("{method} 不支持 --data 请求体")));
    }
    let _method: Method = method.clone();
    Ok((draft, dry_run))
}

fn key_value(value: &str) -> Result<(String, String)> {
    let separator = value
        .find('=')
        .filter(|index| *index > 0)
        .ok_or_else(|| Error::msg("选项需使用 key=value 格式"))?;
    Ok((
        value[..separator].to_string(),
        value[separator + 1..].to_string(),
    ))
}

fn env_name_ok(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn print_services(catalog: &crate::services::ServiceCatalog) {
    println!("ApiBuddy · apibuddy <服务> <接口> [--参数 值]");
    println!();
    println!("服务：");
    for service in &catalog.services {
        println!("  apibuddy {}    {}", service.slug, service.name);
    }
    println!();
    println!("apibuddy <服务> --help  查看该服务全部接口和参数。");
    println!("APIBUDDY_DIR 自定义存储目录。新增服务和接口保存后即可调用。");
    for warning in &catalog.warnings {
        println!("{warning}");
    }
}

fn print_help(service: &Service, templates: &[SavedTemplate], warnings: &[String]) {
    let executable = format!("apibuddy {}", service.slug);
    println!("{} · {executable}", service.name);
    println!();
    println!("全部已保存接口（{}）：", templates.len());
    let commands = service_commands(templates);
    if commands.is_empty() {
        println!("  尚未保存接口；可先使用下面的 request 命令。");
    } else {
        for (name, template) in &commands {
            println!("  {}", command_example(&executable, name, template));
            println!(
                "    {}  {}",
                template.request.method,
                if template.description.is_empty() {
                    &template.name
                } else {
                    &template.description
                }
            );
        }
    }
    println!();
    println!("任意接口：");
    println!("  {executable} request GET /path");
    println!(
        "  {executable} request POST /path --data '{{\"name\":\"example\"}}' --header 'Content-Type: application/json'"
    );
    println!();
    println!("<参数名> 换成实际值；默认值可省略。接口命令加 --help 查看参数，--dry-run 仅预览。");
    println!("--environment 名称 使用桌面环境；{executable} environments 查看环境。");
    println!("APIBUDDY_DIR 自定义存储目录。返回 JSON。");
    for warning in warnings {
        println!("{warning}");
    }
}

fn print_endpoint_help(service: &Service, name: &str, template: &SavedTemplate) {
    let executable = format!("apibuddy {}", service.slug);
    println!("{}", command_example(&executable, name, template));
    println!(
        "{}",
        if template.description.is_empty() {
            &template.name
        } else {
            &template.description
        }
    );
    println!();
    println!("参数：");
    for variable in command_variables(template) {
        let flag = if reserved_options().contains(variable.name.as_str()) {
            format!("--var {}=<值>", variable.name)
        } else {
            format!("--{} <值>", variable.name)
        };
        let hint = if variable.has_default {
            "可选，使用已保存默认值"
        } else {
            "无默认值"
        };
        println!("  {flag}  {hint}");
    }
    println!("  --environment 名称   使用桌面保存的环境");
    println!("  --env 变量名=ENV_NAME  从环境变量读取凭据");
    println!("  --dry-run             预览，不发送请求");
}

/// Install the current CLI binary into `$APIBUDDY_DIR/bin/apibuddy` and write Agent markdown.
pub fn install_cli(
    services: &ServiceStore,
    service_id: &str,
) -> Result<crate::services::AgentBundle> {
    install_cli_with_environment(services, service_id, None)
}

pub fn install_cli_with_environment(
    services: &ServiceStore,
    service_id: &str,
    environment_id: Option<&str>,
) -> Result<crate::services::AgentBundle> {
    let service = services.get(service_id)?;
    let environments = EnvironmentStore::for_service(services, service_id)?.read()?;
    let environment = crate::select_environment(&environments, environment_id)?;
    let root = services.root.clone();
    let bin_dir = root.join("bin");
    let cli_dir = root.join("cli");
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&cli_dir)?;
    let cli_path = bin_dir.join("apibuddy");
    let current = resolve_cli_binary()?;
    fs::copy(&current, &cli_path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&cli_path)?.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(&cli_path, perms)?;
    }
    let library = services.templates(service_id)?.list()?;
    let executable = format!("apibuddy {}", service.slug);
    let mut commands = Vec::new();
    let mut lines = vec![
        format!("# {}", service.name),
        String::new(),
        "把下面初始化命令粘贴到终端，然后按示例调用已保存接口。".into(),
        String::new(),
        "```sh".into(),
        format!(
            "export APIBUDDY_DIR={}",
            crate::curl::quote(&root.to_string_lossy())
        ),
        "export PATH=\"$APIBUDDY_DIR/bin:$PATH\"".into(),
        "```".into(),
        String::new(),
        "## 接口".into(),
        String::new(),
    ];
    for (name, template) in service_commands(&library.templates) {
        let example = environment_example(&executable, &name, &template, environment)?;
        lines.push(format!("### {name}"));
        lines.push(String::new());
        lines.push(format!(
            "{} {}",
            template.request.method,
            if template.description.is_empty() {
                &template.name
            } else {
                &template.description
            }
        ));
        lines.push(String::new());
        lines.push("```sh".into());
        lines.push(example.clone());
        lines.push("```".into());
        lines.push(String::new());
        commands.push(crate::services::AgentCommand {
            name: name.clone(),
            method: template.request.method.clone(),
            description: template.description.clone(),
            example,
        });
    }
    let markdown = lines.join("\n");
    let markdown_path = cli_dir.join(format!("{}.md", service.slug));
    fs::write(&markdown_path, &markdown)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&markdown_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&markdown_path, perms)?;
    }
    let setup = format!(
        "export APIBUDDY_DIR={}\nexport PATH=\"$APIBUDDY_DIR/bin:$PATH\"",
        crate::curl::quote(&root.to_string_lossy())
    );
    Ok(crate::services::AgentBundle {
        cli_path: cli_path.display().to_string(),
        markdown_path: markdown_path.display().to_string(),
        markdown,
        service_id: service.id,
        setup,
        environment_name: environment.map(|profile| profile.name.clone()),
        commands,
        warnings: library.warnings,
    })
}

fn environment_example(
    executable: &str,
    name: &str,
    template: &SavedTemplate,
    environment: Option<&Environment>,
) -> Result<String> {
    let Some(environment) = environment else {
        return Ok(command_example(executable, name, template));
    };
    let mut effective = template.clone();
    effective.request = apply_environment(&template.request, Some(environment))?;
    let mut parts = vec![
        executable.to_string(),
        shell_quote(name),
        "--environment".into(),
        crate::curl::quote(&environment.name),
    ];
    for variable in command_variables(&effective)
        .into_iter()
        .filter(|v| !v.has_default)
    {
        parts.push(if reserved_options().contains(variable.name.as_str()) {
            format!("--var {}=<值>", variable.name)
        } else {
            format!("--{} <值>", variable.name)
        });
    }
    Ok(parts.join(" "))
}

pub fn data_root_display() -> Result<PathBuf> {
    crate::paths::data_directory()
}

fn resolve_cli_binary() -> Result<PathBuf> {
    let current = env::current_exe()?;
    let name = current
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if name == "apibuddy" || name.starts_with("apibuddy-") && !name.contains("app") {
        return Ok(current);
    }
    if let Some(dir) = current.parent() {
        let sibling = dir.join("apibuddy");
        if sibling.exists() {
            return Ok(sibling);
        }
    }
    Err(Error::msg(
        "未找到 apibuddy CLI；请先 cargo build -p apibuddy-cli，或从该二进制运行生成",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_example_uses_environment_without_exposing_values() {
        let input = new_template();
        let template = SavedTemplate {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            revision: 1,
            created_at: String::new(),
            updated_at: String::new(),
            name: "get-order".into(),
            description: String::new(),
            request: input.request,
        };
        let profile = Environment {
            id: uuid::Uuid::new_v4().to_string(),
            name: "local demo".into(),
            variables: vec![Variable {
                key: "base_url".into(),
                value: "http://localhost:1234".into(),
            }],
        };
        let example =
            environment_example("apibuddy orders", "get-order", &template, Some(&profile)).unwrap();
        assert_eq!(
            example,
            "apibuddy orders get-order --environment 'local demo'"
        );
        assert!(!example.contains("localhost"));
    }
}
