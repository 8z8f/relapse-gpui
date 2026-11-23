use std::{
    fs::{self, OpenOptions},
    os::unix::{fs::OpenOptionsExt, io::AsRawFd},
    path::PathBuf,
};

#[derive(Clone)]
pub struct Profile {
    pub name: String,
    pub running: bool,
}

fn directory() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("relapse")
            .join("profiles")
    })
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

fn running(profile: &PathBuf) -> bool {
    let Ok(file) = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(profile.join(".lock"))
    else {
        return false;
    };
    let fd = file.as_raw_fd();
    let busy = unsafe { libc::flock(fd, libc::LOCK_SH | libc::LOCK_NB) } != 0;
    busy && std::io::Error::last_os_error().raw_os_error() == Some(libc::EWOULDBLOCK)
}

pub fn create(name: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err("Use letters, digits, dots, underscores or hyphens".into());
    }
    let Some(directory) = directory() else {
        return Err("No home directory".into());
    };
    let path = directory.join(name);
    if path.exists() {
        return Err(format!("Profile \"{name}\" already exists"));
    }
    fs::create_dir_all(&path).map_err(|error| error.to_string())
}

pub fn rename(name: &str, replacement: &str) -> Result<(), String> {
    if !valid_name(replacement) {
        return Err("Use letters, digits, dots, underscores or hyphens".into());
    }
    if name == replacement {
        return Ok(());
    }
    let Some(directory) = directory() else {
        return Err("No home directory".into());
    };
    let target = directory.join(replacement);
    if target.exists() {
        return Err(format!("Profile \"{replacement}\" already exists"));
    }
    fs::rename(directory.join(name), target).map_err(|error| error.to_string())
}

pub fn delete(name: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err("Invalid profile name".into());
    }
    let Some(directory) = directory() else {
        return Err("No home directory".into());
    };
    fs::remove_dir_all(directory.join(name)).map_err(|error| error.to_string())
}

pub fn load() -> Vec<Profile> {
    let Some(directory) = directory() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(&directory) else {
        return Vec::new();
    };
    let mut profiles = entries
        .flatten()
        .filter_map(|entry| {
            if !entry.file_type().ok()?.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if !valid_name(&name) {
                return None;
            }
            let path = entry.path();
            use std::os::unix::fs::MetadataExt;
            if entry.metadata().ok()?.uid() != unsafe { libc::getuid() } {
                return None;
            }
            Some(Profile {
                running: running(&path),
                name,
            })
        })
        .collect::<Vec<_>>();
    profiles.sort_by(|left, right| left.name.cmp(&right.name));
    profiles
}
