use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::command;

#[derive(Serialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
}

fn read_dir_recursive(path: &PathBuf, base: &PathBuf) -> Vec<FileNode> {
    let mut nodes = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let is_dir = path.is_dir();

            let rel_path = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .trim_start_matches(|c| c == '\\' || c == '/')
                .to_string();

            let children = if is_dir {
                Some(read_dir_recursive(&path, base))
            } else {
                None
            };

            nodes.push(FileNode {
                name,
                path: rel_path,
                is_dir,
                children,
            });
        }
    }

    nodes
}

#[command]
pub fn get_tree(root: String) -> Result<Vec<FileNode>, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.exists() {
        return Err(format!("Root path does not exist: {}", root));
    }
    Ok(read_dir_recursive(&root_path, &root_path))
}
