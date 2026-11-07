use crate::error::{Error, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn atomic_write(path: &Path, content: &str, mode: u32) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(mode);
        #[cfg(not(unix))]
        let _ = mode;
        let mut file = options.open(&temporary)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if temporary.exists() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o700);
        fs::set_permissions(path, perms)?;
    }
    if !path.is_dir() || path.symlink_metadata()?.file_type().is_symlink() {
        return Err(Error::msg("目录不能是符号链接或普通文件"));
    }
    Ok(())
}

pub struct WriteLock {
    path: PathBuf,
}

impl WriteLock {
    pub fn acquire(directory: &Path) -> Result<Self> {
        let path = directory.join(".write-lock");
        match fs::create_dir(&path) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    let mut perms = fs::metadata(&path)?.permissions();
                    perms.set_mode(0o700);
                    fs::set_permissions(&path, perms)?;
                }
                Ok(Self { path })
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Err(Error::msg(
                "正在写入，请稍后重试；若应用异常退出，请检查存储目录中的 .write-lock",
            )),
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
    }
}

#[allow(dead_code)]
pub fn read_limited(path: &Path, max_bytes: u64) -> Result<String> {
    let meta = path.symlink_metadata()?;
    if !meta.file_type().is_file() || meta.file_type().is_symlink() || meta.len() > max_bytes {
        return Err(Error::msg("文件类型错误或过大"));
    }
    let mut file = File::open(path)?;
    let mut buf = String::new();
    std::io::Read::read_to_string(&mut file, &mut buf)?;
    Ok(buf)
}
