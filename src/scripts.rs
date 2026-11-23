use std::{collections::HashSet, fs, path::PathBuf};

const DEFAULT_SCRIPT: &str = "print(\"hello from relapse\")\n";

fn directory() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("relapse-gpui")
            .join("scripts")
    })
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name != "."
        && name != ".."
        && !name.contains('/')
        && (name.ends_with(".lua") || name.ends_with(".luau") || name.ends_with(".txt"))
}

pub fn path_for(name: &str) -> Option<PathBuf> {
    if !valid_name(name) {
        return None;
    }
    Some(directory()?.join(name))
}

pub fn load() -> Vec<String> {
    let Some(dir) = directory() else {
        return vec!["script.lua".to_string()];
    };
    let _ = fs::create_dir_all(&dir);
    let mut names: Vec<String> = fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type().ok()?.is_file() && valid_name(&name) {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    names.sort();
    if names.is_empty() {
        let _ = write("script.lua", DEFAULT_SCRIPT);
        names.push("script.lua".to_string());
    }
    names
}

pub fn read(name: &str) -> Option<String> {
    fs::read_to_string(path_for(name)?).ok()
}

pub fn write(name: &str, content: &str) -> Result<(), String> {
    let path = path_for(name).ok_or_else(|| "Invalid script name".to_string())?;
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path, content).map_err(|error| error.to_string())
}

pub fn create() -> Result<String, String> {
    let existing: HashSet<String> = load().into_iter().collect();
    let mut name = String::from("script.lua");
    let mut index = 1;
    while existing.contains(&name) {
        name = format!("script-{index}.lua");
        index += 1;
    }
    write(&name, "")?;
    Ok(name)
}

pub fn import(source: &str, content: &str) -> Result<String, String> {
    let base = source.rsplit('/').next().unwrap_or(source);
    let existing: HashSet<String> = load().into_iter().collect();
    let candidate = if valid_name(base) {
        base.to_string()
    } else {
        "imported.lua".to_string()
    };
    let (stem, ext) = match candidate.rsplit_once('.') {
        Some((stem, ext)) => (stem.to_string(), ext.to_string()),
        None => (candidate.clone(), "lua".to_string()),
    };
    let mut name = candidate.clone();
    let mut index = 1;
    while existing.contains(&name) {
        name = format!("{stem}-{index}.{ext}");
        index += 1;
    }
    write(&name, content)?;
    Ok(name)
}

pub fn delete(name: &str) -> Result<(), String> {
    let path = path_for(name).ok_or_else(|| "Invalid script name".to_string())?;
    fs::remove_file(path).map_err(|error| error.to_string())
}
