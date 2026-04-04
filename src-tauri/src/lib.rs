mod server;
mod java;
mod software;

use std::fs;

use directories::ProjectDirs;
use server::server_manager::{create_server, delete_server, get_servers, load_servers, get_server, ServerManager};


#[tauri::command]
fn get_installed_java() -> Result<Vec<String>, String> {
    let proj_dirs = ProjectDirs::from("com", "localcraft", "LocalCraft")
        .ok_or("No se pudo calcular la ruta de la aplicación")?;
    let java_dir = proj_dirs.data_dir().join("java");

    if !java_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    if let Ok(entries) = fs::read_dir(java_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    versions.push(name.to_string());
                }
            }
        }
    }
    Ok(versions)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        std::env::set_var("GDK_BACKEND", "x11");
    }

    let server_manager = ServerManager::new();
    ensure_app_dirs().unwrap();

    tauri::Builder::default()
        .manage(server_manager)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_prevent_default::init())
        .invoke_handler(tauri::generate_handler![
            load_servers,
            get_servers,
            delete_server,
            create_server,
            get_installed_java,
            get_server
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


pub fn ensure_app_dirs() -> Result<std::path::PathBuf, String> {
    let proj_dirs = ProjectDirs::from("com", "localcraft", "LocalCraft")
        .ok_or("No se pudo calcular la ruta de la aplicación")?;

    let data_dir = proj_dirs.data_dir();
    let servers_dir = data_dir.join("servers");
    let java_dir = data_dir.join("java");

    fs::create_dir_all(&servers_dir).map_err(|e| format!("Error creando directorios: {}", e))?;
    fs::create_dir_all(&java_dir).map_err(|e| format!("Error creando directorios: {}", e))?;

    Ok(data_dir.to_path_buf())
}
