use std::fs;
use std::path::PathBuf;
use tauri::command;

#[command]
pub fn create_folder(root: String, name: String) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Folder name cannot be empty".into());
    }

    let mut path = PathBuf::from(root);
    path.push(name);

    if path.exists() {
        return Err("Folder already exists".into());
    }

    fs::create_dir_all(path).map_err(|e| e.to_string())
}

#[command]
pub fn create_file(root: String, name: String) -> Result<(), String> {
    let mut path = PathBuf::from(root);
    path.push(name);

    if path.exists() {
        return Err("File already exists".into());
    }

    fs::write(path, "").map_err(|e| e.to_string())
}

#[command]
pub fn rename_file(root: String, old_path: String, new_name: String) -> Result<(), String> {
    let mut old = PathBuf::from(&root);
    old.push(&old_path);

    if !old.exists() {
        return Err("Source path does not exist".into());
    }

    if new_name.trim().is_empty() {
        return Err("New name cannot be empty".into());
    }

    let old_rel = PathBuf::from(&old_path);
    let parent_rel = old_rel.parent().map(|p| p.to_path_buf()).unwrap_or_default();

    let mut new = PathBuf::from(&root);
    new.push(parent_rel);
    new.push(new_name);

    if new.exists() {
        return Err("Target path already exists".into());
    }

    fs::rename(old, new).map_err(|e| e.to_string())
}

#[command]
pub fn delete_path(root: String, rel_path: String) -> Result<(), String> {
    if rel_path.trim().is_empty() {
        return Err("Path cannot be empty".into());
    }

    let mut path = PathBuf::from(root);
    path.push(rel_path);

    if !path.exists() {
        return Err("Path does not exist".into());
    }

    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())
    }
}

#[command]
pub fn read_file(root: String, rel_path: String) -> Result<String, String> {
    let mut path = PathBuf::from(root);
    path.push(rel_path);

    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[command]
pub fn write_file(root: String, rel_path: String, content: String) -> Result<(), String> {
    let mut path = PathBuf::from(root);
    path.push(rel_path);

    fs::write(path, content).map_err(|e| e.to_string())
}
