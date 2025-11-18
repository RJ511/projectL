#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::fs;
use std::path::PathBuf;


// -----------------------------
// Estrutura do file tree
// -----------------------------
#[derive(Serialize)]
struct FileNode {
    name: String,
    path: String,
    is_dir: bool,
    children: Option<Vec<FileNode>>,
}

// -----------------------------
// Lê diretórios recursivamente
// -----------------------------
fn read_dir_recursive(path: &PathBuf, base: &PathBuf) -> Vec<FileNode> {
    let mut nodes = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let is_dir = path.is_dir();

            let relPath = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            let children = if is_dir {
                Some(read_dir_recursive(&path, base))
            } else {
                None
            };

            nodes.push(FileNode {
                name,
                path: relPath,
                is_dir,
                children,
            });
        }
    }

    nodes
}

// -----------------------------
// GET TREE
// -----------------------------
#[tauri::command]
fn get_tree(root: String) -> Result<Vec<FileNode>, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.exists() {
        return Err(format!("Root path does not exist: {}", root));
    }
    Ok(read_dir_recursive(&root_path, &root_path))
}

// -----------------------------
// CREATE FILE
// -----------------------------
#[tauri::command]
fn create_file(root: String, name: String) -> Result<(), String> {
    let mut path = PathBuf::from(root);
    path.push(name);

    if path.exists() {
        return Err("File already exists".into());
    }

    fs::write(path, "").map_err(|e| e.to_string())
}

// -----------------------------
// RENAME FILE
// -----------------------------
#[tauri::command]
fn rename_file(root: String, old_path: String, new_name: String) -> Result<(), String> {
    let mut old = PathBuf::from(&root);
    old.push(old_path);

    let mut new = PathBuf::from(&root);
    new.push(new_name);

    fs::rename(old, new).map_err(|e| e.to_string())
}

// -----------------------------
// READ FILE
// -----------------------------
#[tauri::command]
fn read_file(root: String, relPath: String) -> Result<String, String> {
    let mut path = PathBuf::from(root);
    path.push(relPath);

    fs::read_to_string(path).map_err(|e| e.to_string())
}

// -----------------------------
// WRITE FILE
// -----------------------------
#[tauri::command]
fn write_file(root: String, relPath: String, content: String) -> Result<(), String> {
    let mut path = PathBuf::from(root);
    path.push(relPath);

    fs::write(path, content).map_err(|e| e.to_string())
}

// -----------------------------
// MAIN — TAURI 2 CORRETO
// -----------------------------
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_tree,
            read_file,
            write_file,
            create_file,
            rename_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
