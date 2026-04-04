use std::{collections::HashMap, sync::RwLock};
use directories::ProjectDirs;
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::{java::java_manager::JavaManager, server::server::Server};

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
pub fn delete_server(id: String, state: State<'_, ServerManager>) -> Result<(), String> {
    let server_opt = {
        let servers = state.servers.read().unwrap();
        servers.get(&id).cloned()
    };
    
    if let Some(server) = server_opt {
        server.delete()?;
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
    let new_server = Server::create(&handle, name, version, software, java_version, ram).await?;
    JavaManager::install_java(&handle, &new_server).await?;
    state.add_server(new_server.clone());
    Ok(new_server)
}

#[tauri::command]
pub fn get_server(id: String, state: State<'_, ServerManager>) -> Result<Server, String> {
    let servers = state.servers.read().unwrap();
    servers.get(&id).cloned().ok_or("Server no encontrado".to_string())
}
