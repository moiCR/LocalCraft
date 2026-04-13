use directories::ProjectDirs;
use serde::Serialize;
use std::{collections::HashMap, sync::RwLock};
use tauri::{AppHandle, Emitter, State};

use crate::{
    java::java_manager::JavaManager, server::server::Server,
    software::software_manager::SoftwareManager,
};

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub process: String,
    pub percentage: Option<f64>,
}

pub struct ServerManager {
    pub servers: RwLock<HashMap<String, Server>>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_server(&self, server: Server) {
        let mut servers = self.servers.write().unwrap();
        servers.insert(server.id.clone(), server);
    }

    pub fn remove_server(&self, id: &str) {
        let mut servers = self.servers.write().unwrap();
        servers.remove(id);
    }

    pub fn get_all_servers(&self) -> Vec<Server> {
        let servers = self.servers.read().unwrap();
        servers.values().cloned().collect()
    }

    pub fn load_from_disk(&self) -> Result<(), String> {
        let proj_dirs =
            ProjectDirs::from("com", "localcraft", "LocalCraft").ok_or("Ruta no encontrada")?;
        let servers_dir = proj_dirs.data_dir().join("servers");

        if !servers_dir.exists() {
            return Ok(());
        }

        let mut cache = self.servers.write().unwrap();

        for entry in std::fs::read_dir(servers_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let config_path = entry.path().join("server.json");

            if config_path.exists() {
                let data = std::fs::read(config_path).map_err(|e| e.to_string())?;
                let server: Server = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
                cache.insert(server.id.clone(), server);
            }
        }
        Ok(())
    }

    pub async fn shutdown_all_servers(&self) {
        let servers = self.get_all_servers();
        let mut shutdown_tasks = Vec::new();

        for server in servers {
            let running = {
                let guard = server.running.lock().await;
                guard.is_some()
            };
            if running {
                println!("Deteniendo servidor: {}", server.name);
                let _ = server.send_command("stop".to_string()).await;
                let s = server.clone();
                shutdown_tasks.push(tokio::spawn(async move {
                    s.wait_until_stopped().await;
                    println!("Servidor {} detenido.", s.name);
                }));
            }
        }

        for task in shutdown_tasks {
            let _ = task.await;
        }
    }
}
#[tauri::command]
pub async fn start_server(
    id: String,
    state: State<'_, ServerManager>,
    handle: AppHandle,
) -> Result<(), String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&id).cloned()
    };

    if let Some(server) = server {
        server.start(&handle).await?;
        Ok(())
    } else {
        Err("Server not found".to_string())
    }
}

#[tauri::command]
pub fn load_servers(state: State<'_, ServerManager>) -> Result<(), String> {
    state.load_from_disk()
}

#[tauri::command]
pub fn get_servers(state: State<'_, ServerManager>) -> Result<Vec<Server>, String> {
    let is_empty = state.servers.read().unwrap().is_empty();

    if is_empty {
        println!("Loading servers from disk...");
        state.load_from_disk()?;
    }

    Ok(state.get_all_servers())
}

#[tauri::command]
pub async fn delete_server(id: String, state: State<'_, ServerManager>) -> Result<(), String> {
    let server_opt = {
        let servers = state.servers.read().unwrap();
        servers.get(&id).cloned()
    };

    if let Some(server) = server_opt {
        server.delete().await?;
        state.remove_server(&id);
        Ok(())
    } else {
        Err("Server no encontrado".to_string())
    }
}

#[tauri::command]
pub async fn create_server(
    name: String,
    version: String,
    software: String,
    java_version: String,
    ram: String,
    state: State<'_, ServerManager>,
    handle: AppHandle,
) -> Result<Server, String> {
    // 1. Create directory, save config, write eula.txt
    let new_server = Server::create(&handle, name, version, software, java_version, ram).await?;

    // 2. Install Java (must happen before Forge, which needs it to run the installer)
    JavaManager::install_java(&handle, &new_server).await?;

    // 3. Download / install the server software
    match new_server.software.as_str() {
        "forge" => SoftwareManager::get_forge_jar(&handle, &new_server).await?,
        "fabric" => SoftwareManager::get_fabric_jar(&handle, &new_server).await?,
        "paper" => SoftwareManager::get_paper_jar(&handle, &new_server).await?,
        "vanilla" => SoftwareManager::get_vanilla_jar(&handle, &new_server).await?,
        other => return Err(format!("Unknown software type: {}", other)),
    }

    let _ = handle.emit(
        "creation-progress",
        ProgressPayload {
            process: "Server created successfully.".to_string(),
            percentage: Some(100.0),
        },
    );

    state.add_server(new_server.clone());
    Ok(new_server)
}

#[tauri::command]
pub fn get_server(id: String, state: State<'_, ServerManager>) -> Result<Server, String> {
    let servers = state.servers.read().unwrap();
    servers
        .get(&id)
        .cloned()
        .ok_or("Server no encontrado".to_string())
}

#[tauri::command]
pub async fn send_command(
    id: String,
    command: String,
    state: State<'_, ServerManager>,
) -> Result<(), String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&id).cloned()
    };

    if let Some(server) = server {
        server.send_command(command).await?;
        Ok(())
    } else {
        Err("Server not found".to_string())
    }
}

#[tauri::command]
pub async fn is_server_running(
    id: String,
    state: State<'_, ServerManager>,
) -> Result<bool, String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers.get(&id).cloned()
    };

    if let Some(server) = server {
        let guard = server.running.lock().await;
        Ok(guard.is_some())
    } else {
        Err("Server not found".to_string())
    }
}

#[tauri::command]
pub fn open_folder(
    server_id: String,
    path: Option<String>,
    state: State<'_, ServerManager>,
) -> Result<(), String> {
    let server = {
        let servers = state.servers.read().unwrap();
        servers
            .get(&server_id)
            .cloned()
            .ok_or("Server no encontrado")?
    };

    let mut full_path = server.get_path()?;

    if let Some(sub_path) = path {
        full_path = full_path.join(sub_path);
    }

    if !full_path.exists() {
        return Err("La ruta no existe".to_string());
    }

    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer")
        .arg(&full_path)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&full_path)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&full_path)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn update_server(
    id: String,
    name: String,
    ram: String,
    java_version: String,
    state: State<'_, ServerManager>,
) -> Result<Server, String> {
    let mut servers = state.servers.write().unwrap();
    let server = servers
        .get_mut(&id)
        .ok_or_else(|| "Server not found".to_string())?;
    server.name = name;
    server.ram = ram;
    server.java_version = Some(java_version);
    server.save_json()?;
    Ok(server.clone())
}
