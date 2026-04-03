use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use std::fs;
use uuid::Uuid;

use crate::{server::server_manager::ProgressPayload, software::software_manager::SoftwareManager};

#[derive(Serialize, Deserialize, Clone)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub version: String,
    pub software: String,
    pub ram: String,
    pub java_version: Option<String>,
}

impl Server {
    pub async fn create(
        handle : &AppHandle,
        name: String,
        version: String,
        software: String,
        java_version: String,
        ram: String,
    ) -> Result<Self, String> {
        let id = Uuid::new_v4().to_string();

        let proj_dirs = ProjectDirs::from("com", "localcraft", "LocalCraft")
            .ok_or("No se pudo determinar el directorio de inicio")?;
        let server_dir = proj_dirs.data_dir().join("servers").join(&id);

        if server_dir.exists() {
            return Err("El directorio ya existe (Colisión de UUID)".into());
        }
        fs::create_dir_all(&server_dir).map_err(|e| format!("Error al crear carpeta: {}", e))?;

        let server = Self {
            id,
            name,
            version,
            software,
            java_version: Some(java_version),
            ram,
        };

        server.save_json()?;

        fs::write(server_dir.join("eula.txt"), "eula=true\n")
            .map_err(|e| format!("Error al crear eula.txt: {}", e))?;

        if server.software == "paper" {
            SoftwareManager::get_paper_jar(handle, &server).await?;
        }

        let _ = handle.emit("creation-progress", ProgressPayload {
            process: "Server created successfully.".to_string(),
            percentage: Some(100.0),
        });

        Ok(server)
    }

    pub fn save_json(&self) -> Result<(), String> {
        let proj_dirs = ProjectDirs::from("com", "localcraft", "LocalCraft")
            .ok_or("No se pudo determinar el directorio de inicio")?;
        let server_dir = proj_dirs.data_dir().join("servers").join(&self.id);

        if !server_dir.exists() {
            return Err("El directorio no existe (Colisión de UUID)".into());
        }
        fs::create_dir_all(&server_dir).map_err(|e| format!("Error al crear carpeta: {}", e))?;

        let config_path = server_dir.join("server.json");
        let json = serde_json::to_string_pretty(&self)
            .map_err(|e| format!("Error de serialización: {}", e))?;

        fs::write(config_path, json)
            .map_err(|e| format!("Error al escribir server.json: {}", e))?;

        Ok(())
    }

    pub fn delete(&self) -> Result<(), String> {
        let proj_dirs =
            ProjectDirs::from("com", "localcraft", "LocalCraft").ok_or("Ruta no encontrada")?;

        let server_dir = proj_dirs.data_dir().join("servers").join(&self.id);

        if !server_dir.exists() {
            return Err(format!("Server directory not found {}", &self.id));
        }

        fs::remove_dir_all(server_dir)
            .map_err(|e| format!("Failed to delete server directory {}", e))?;

        Ok(())
    }

    pub fn get_path(&self) -> Result<std::path::PathBuf, String> {
        let proj_dirs =
            ProjectDirs::from("com", "localcraft", "LocalCraft").ok_or("Ruta no encontrada")?;
        Ok(proj_dirs.data_dir().join("servers").join(&self.id))
    }
}
