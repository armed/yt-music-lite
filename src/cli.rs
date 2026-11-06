use std::{fs, path::PathBuf};

const LINK_NAME: &str = "ytm-lite";

pub fn link_path() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    Some(home.join(".local/bin").join(LINK_NAME))
}

fn current_exe() -> Result<PathBuf, String> {
    std::env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|err| err.to_string())
}

pub fn is_installed() -> bool {
    let (Some(link), Ok(exe)) = (link_path(), current_exe()) else {
        return false;
    };
    fs::read_link(link).is_ok_and(|target| target == exe)
}

#[cfg(unix)]
pub fn install() -> Result<(), String> {
    let link = link_path().ok_or("HOME is not set")?;
    let exe = current_exe()?;
    if let Some(dir) = link.parent() {
        fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    match fs::symlink_metadata(&link) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fs::remove_file(&link).map_err(|err| err.to_string())?;
        }
        Ok(_) => return Err(format!("{} exists and is not a symlink", link.display())),
        Err(_) => {}
    }
    std::os::unix::fs::symlink(exe, link).map_err(|err| err.to_string())
}

#[cfg(not(unix))]
pub fn install() -> Result<(), String> {
    Err("not supported on this platform".into())
}

pub fn uninstall() -> Result<(), String> {
    let Some(link) = link_path() else {
        return Ok(());
    };
    match fs::symlink_metadata(&link) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fs::remove_file(&link).map_err(|err| err.to_string())
        }
        Ok(_) => Err(format!("{} is not a symlink, leaving it alone", link.display())),
        Err(_) => Ok(()),
    }
}
