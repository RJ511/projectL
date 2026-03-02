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
    old.push(old_path);

    let mut new = PathBuf::from(&root);
    new.push(new_name);

    fs::rename(old, new).map_err(|e| e.to_string())
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
