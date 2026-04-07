use std::fs::File;
use std::io::Write;

use serde_json::Value;
use tauri::{AppHandle, Emitter};

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

        let mut file = File::create(server.get_path()?.join("server.jar"))
            .map_err(|e| format!("Failed to create server jar file: {}", e))?;

        file.write_all(&bytes)
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
}
