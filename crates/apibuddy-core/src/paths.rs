use crate::error::{Error, Result};
use std::env;
use std::path::{Path, PathBuf};

/// Resolve the ApiBuddy data root.
///
/// Priority: `APIBUDDY_DIR` > `APIABC_DIR` > `APIX_DIR`.
/// When unset, prefer an existing `~/.apibuddy`, `~/.apiabc`, or `~/.apix`, else `~/.apibuddy`.
pub fn data_directory() -> Result<PathBuf> {
    data_directory_with(
        &env::vars().collect::<Vec<_>>(),
        home_dir()?,
        env::current_dir()?,
    )
}

pub fn data_directory_with(
    env_pairs: &[(String, String)],
    home: PathBuf,
    cwd: PathBuf,
) -> Result<PathBuf> {
    let lookup = |name: &str| {
        env_pairs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    };

    for name in ["APIBUDDY_DIR", "APIABC_DIR", "APIX_DIR"] {
        if let Some(configured) = lookup(name) {
            if configured.trim().is_empty() {
                return Err(Error::msg(format!("{name} 不能为空")));
            }
            let expanded = expand_path(&configured, &home);
            return Ok(if expanded.is_absolute() {
                expanded
            } else {
                cwd.join(expanded)
            });
        }
    }

    let candidates = [
        home.join(".apibuddy"),
        home.join(".apiabc"),
        home.join(".apix"),
    ];
    Ok(candidates
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| home.join(".apibuddy")))
}

fn expand_path(configured: &str, home: &Path) -> PathBuf {
    if configured == "~" {
        home.to_path_buf()
    } else if let Some(rest) = configured.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(configured)
    }
}

fn home_dir() -> Result<PathBuf> {
    env::home_dir().ok_or_else(|| Error::msg("无法确定用户主目录"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn prefers_explicit_apibuddy_dir() {
        let home = env::temp_dir().join("apibuddy-home-test");
        let cwd = env::temp_dir();
        let env = vec![("APIBUDDY_DIR".into(), "~/data".into())];
        let path = data_directory_with(&env, home.clone(), cwd).unwrap();
        assert_eq!(path, home.join("data"));
    }

    #[test]
    fn falls_back_to_existing_legacy_dir() {
        let root = std::env::temp_dir().join(format!("apibuddy-path-{}", uuid::Uuid::new_v4()));
        let legacy = root.join(".apix");
        fs::create_dir_all(&legacy).unwrap();
        let path = data_directory_with(&[], root.clone(), root.clone()).unwrap();
        assert_eq!(path, legacy);
        let _ = fs::remove_dir_all(root);
    }
}
