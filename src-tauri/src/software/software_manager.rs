use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use serde_json::Value;
use tauri::{AppHandle, Emitter};
use directories::ProjectDirs;

use crate::server::{server::Server, server_manager::ProgressPayload};

pub struct SoftwareManager {}

impl SoftwareManager {
    async fn download_server_jar(
        handle: &AppHandle,
        download_url: &String,
        server: &Server,
    ) -> Result<(), String> {
        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Downloading server jar...".to_string(),
                percentage: None,
            },
        );

        let mut response = reqwest::get(download_url)
            .await
            .map_err(|e| format!("Failed to download server jar: {}", e))?;

        let total_size = response.content_length().unwrap_or(0) as f64;
        let mut bytes = Vec::new();
        let mut downloaded = 0.0;

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("Failed to get download chunk: {}", e))?
        {
            bytes.extend_from_slice(&chunk);
            downloaded += chunk.len() as f64;

            if total_size > 0.0 {
                let pct = (downloaded / total_size) * 100.0;
                let _ = handle.emit(
                    "creation-progress",
                    ProgressPayload {
                        process: "Downloading server jar...".to_string(),
                        percentage: Some(pct),
                    },
                );
            }
        }

        let mut file = fs::File::create(server.get_path()?.join("server.jar"))
            .await
            .map_err(|e| format!("Failed to create server jar file: {}", e))?;

        file.write_all(&bytes)
            .await
            .map_err(|e| format!("Failed to write server jar file: {}", e))?;

        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Server jar downloaded successfully.".to_string(),
                percentage: Some(100.0),
            },
        );

        Ok(())
    }

    pub async fn get_paper_jar(handle: &AppHandle, server: &Server) -> Result<(), String> {
        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Fetching Paper...".to_string(),
                percentage: None,
            },
        );

        let api_url = format!(
            "https://api.papermc.io/v2/projects/paper/versions/{}",
            server.version
        );

        let resp: Value = reqwest::get(&api_url)
            .await
            .map_err(|e| format!("Failed to fetch Paper API: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let builds = resp["builds"]
            .as_array()
            .ok_or("Invalid response format: 'builds' is not an array")?;

        let latest_build = builds.last().ok_or("No builds found for this version")?;

        let download_url = format!(
            "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}/downloads/paper-{}-{}.jar",
            server.version, latest_build, server.version, latest_build
        );

        SoftwareManager::download_server_jar(handle, &download_url, server).await?;
        Ok(())
    }

    pub async fn download_playit(handle: &AppHandle) -> Result<PathBuf, String> {
        let proj_dirs = ProjectDirs::from("com", "localcraft", "LocalCraft")
            .ok_or("Failed to determine app directory")?;
        let app_dir = proj_dirs.data_dir();
        let tools_dir = app_dir.join("tools");

        fs::create_dir_all(&tools_dir)
            .await
            .map_err(|e| format!("Failed to create tools directory: {}", e))?;

        let exe_path = tools_dir.join("playit.exe");
        if exe_path.exists() {
            return Ok(exe_path);
        }

        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Downloading playit.exe...".to_string(),
                percentage: None,
            },
        );

        let url = "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-windows-x86_64.exe";

        let mut response = reqwest::get(url)
            .await
            .map_err(|e| format!("Failed to download playit: {}", e))?;

        let total_size = response.content_length().unwrap_or(0) as f64;
        let mut bytes = Vec::new();
        let mut downloaded = 0.0;

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("Failed to get download chunk: {}", e))?
        {
            bytes.extend_from_slice(&chunk);
            downloaded += chunk.len() as f64;

            if total_size > 0.0 {
                let pct = (downloaded / total_size) * 100.0;
                let _ = handle.emit(
                    "creation-progress",
                    ProgressPayload {
                        process: "Downloading playit.exe...".to_string(),
                        percentage: Some(pct),
                    },
                );
            }
        }

        fs::write(&exe_path, bytes)
            .await
            .map_err(|e| format!("Failed to write playit.exe: {}", e))?;

        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Playit.exe downloaded successfully.".to_string(),
                percentage: Some(100.0),
            },
        );

        Ok(exe_path)
    }
}

#[tauri::command]
pub async fn download_playit_command(handle: AppHandle) -> Result<String, String> {
    let path = SoftwareManager::download_playit(&handle).await?;
    Ok(path.to_string_lossy().to_string())
}
