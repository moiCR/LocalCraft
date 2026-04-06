use std::fs;

use serde::Serialize;
use tauri::State;

use crate::server::server_manager::ServerManager;

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[tauri::command]
pub fn read_dir(
    server_id: String,
    sub_path: Option<String>,
    state: State<'_, ServerManager>,
) -> Result<Vec<FileInfo>, String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned().ok_or("Server no encontrado")?
    };
    let base = server.get_path()?;
    let path = match sub_path {
        Some(p) if !p.is_empty() => base.join(p),
        _ => base,
    };

    let mut files = Vec::new();
    for entry in fs::read_dir(&path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let entry_path = entry.path();
        let file_name = entry_path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_string();
        let is_dir = entry_path.is_dir();
        let size = if is_dir {
            0
        } else {
            entry_path.metadata().map(|m| m.len()).unwrap_or(0)
        };
        files.push(FileInfo {
            name: file_name,
            is_dir,
            size,
        });
    }

    files.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(files)
}

#[tauri::command]
pub fn read_file(
    server_id: String,
    path: String,
    state: State<'_, ServerManager>,
) -> Result<String, String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned().ok_or("Server no encontrado")?
    };
    let full_path = server.get_path()?.join(&path);
    let content = fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
    Ok(content)
}

#[tauri::command]
pub fn write_file(
    server_id: String,
    path: String,
    content: String,
    state: State<'_, ServerManager>,
) -> Result<(), String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned().ok_or("Server no encontrado")?
    };
    let full_path = server.get_path()?.join(&path);
    fs::write(&full_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_file(
    server_id: String,
    path: String,
    state: State<'_, ServerManager>,
) -> Result<(), String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned().ok_or("Server no encontrado")?
    };
    let full_path = server.get_path()?.join(&path);
    fs::remove_file(&full_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn create_dir(
    server_id: String,
    path: String,
    state: State<'_, ServerManager>,
) -> Result<(), String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned().ok_or("Server no encontrado")?
    };
    let full_path = server.get_path()?.join(&path);
    fs::create_dir_all(&full_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_dir(
    server_id: String,
    path: String,
    state: State<'_, ServerManager>,
) -> Result<(), String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned().ok_or("Server no encontrado")?
    };
    let full_path = server.get_path()?.join(&path);
    fs::remove_dir_all(&full_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rename_file(
    server_id: String,
    path: String,
    new_path: String,
    state: State<'_, ServerManager>,
) -> Result<(), String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned().ok_or("Server no encontrado")?
    };
    let base = server.get_path()?;
    let old = base.join(&path);
    let new = base.join(&new_path);
    fs::rename(&old, &new).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_file_binary(
    server_id: String,
    path: String,
    data: Vec<u8>,
    state: State<'_, ServerManager>,
) -> Result<(), String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&server_id).cloned().ok_or("Server no encontrado")?
    };
    let full_path = server.get_path()?.join(&path);
    fs::write(&full_path, data).map_err(|e| e.to_string())?;
    Ok(())
}
