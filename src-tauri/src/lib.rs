mod server;
mod java;
mod software;

use std::fs;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{Manager, WindowEvent, Emitter};

use directories::ProjectDirs;
use server::server_manager::{
    create_server,
    delete_server, 
    get_servers, 
    load_servers, 
    get_server, 
    start_server, 
    send_command, 
    is_server_running,
    ServerManager
};


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
        .setup(|app| {
            let handle = app.handle();
            
            let item_home = MenuItem::with_id(app, "home", "Home", true, None::<&str>)?;
            let item_servers = MenuItem::with_id(app, "servers", "Servers", true, None::<&str>)?;
            let item_java = MenuItem::with_id(app, "java", "Java Environments", true, None::<&str>)?;
            let item_about = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let item_quit = MenuItem::with_id(app, "quit", "Quit / Cerrar", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[
                &item_home,
                &item_servers,
                &item_java,
                &item_about,
                &separator,
                &item_quit
            ])?;

            let icon = app.default_window_icon().unwrap().clone();

            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("LocalCraft")
                .menu(&menu)
                .on_menu_event(move |app_handle, event| {
                    let id = event.id.as_ref();
                    if id == "quit" {
                        let app_clone = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            println!("Shutting down all servers before exit...");
                            let manager = app_clone.state::<ServerManager>();
                            manager.shutdown_all_servers().await;
                            println!("All servers safely handled. Exiting app.");
                            app_clone.exit(0);
                        });
                    } else {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = app_handle.emit("navigate", id);
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                let _ = window.hide();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            load_servers,
            get_servers,
            delete_server,
            create_server,
            get_installed_java,
            get_server,
            start_server,
            send_command,
            is_server_running
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
