use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use directories::ProjectDirs;
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

    pub async fn get_vanilla_jar(handle: &AppHandle, server: &Server) -> Result<(), String> {
        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Fetching Vanilla manifest...".to_string(),
                percentage: None,
            },
        );

        let manifest: Value =
            reqwest::get("https://launchermeta.mojang.com/mc/game/version_manifest.json")
                .await
                .map_err(|e| format!("Failed to fetch version manifest: {}", e))?
                .json()
                .await
                .map_err(|e| format!("Failed to parse version manifest: {}", e))?;

        let versions = manifest["versions"]
            .as_array()
            .ok_or("Invalid manifest: 'versions' not an array")?;

        let version_url = versions
            .iter()
            .find(|v| v["id"].as_str() == Some(server.version.as_str()))
            .and_then(|v| v["url"].as_str())
            .ok_or_else(|| format!("Version {} not found in manifest", server.version))?
            .to_string();

        let version_manifest: Value = reqwest::get(&version_url)
            .await
            .map_err(|e| format!("Failed to fetch version manifest: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse version manifest: {}", e))?;

        let server_jar_url = version_manifest["downloads"]["server"]["url"]
            .as_str()
            .ok_or("Server jar URL not found in version manifest")?
            .to_string();

        SoftwareManager::download_server_jar(handle, &server_jar_url, server).await?;
        Ok(())
    }

    pub async fn get_fabric_jar(handle: &AppHandle, server: &Server) -> Result<(), String> {
        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Fetching Fabric...".to_string(),
                percentage: None,
            },
        );

        let loader_url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}",
            server.version
        );
        let loaders: Value = reqwest::get(&loader_url)
            .await
            .map_err(|e| format!("Failed to fetch Fabric loaders: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse Fabric loaders: {}", e))?;

        let loader_version = loaders[0]["loader"]["version"]
            .as_str()
            .ok_or("Failed to get Fabric loader version")?
            .to_string();

        let installers: Value = reqwest::get("https://meta.fabricmc.net/v2/versions/installer")
            .await
            .map_err(|e| format!("Failed to fetch Fabric installers: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse Fabric installers: {}", e))?;

        let installer_version = installers[0]["version"]
            .as_str()
            .ok_or("Failed to get Fabric installer version")?
            .to_string();

        let download_url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/{}/server/jar",
            server.version, loader_version, installer_version
        );

        SoftwareManager::download_server_jar(handle, &download_url, server).await?;
        Ok(())
    }

    pub async fn get_forge_jar(handle: &AppHandle, server: &Server) -> Result<(), String> {
        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Fetching Forge versions...".to_string(),
                percentage: None,
            },
        );

        let promos: Value = reqwest::get(
            "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json",
        )
        .await
        .map_err(|e| format!("Failed to fetch Forge promotions: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse Forge promotions: {}", e))?;

        let promos_obj = promos["promos"]
            .as_object()
            .ok_or("Invalid Forge promotions format")?;

        let forge_version = promos_obj
            .get(&format!("{}-recommended", server.version))
            .or_else(|| promos_obj.get(&format!("{}-latest", server.version)))
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("No Forge build found for Minecraft {}", server.version))?
            .to_string();

        let mc = &server.version;
        let installer_name = format!("forge-{}-{}-installer.jar", mc, forge_version);
        let installer_url = format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{}-{}/{}",
            mc, forge_version, installer_name
        );

        // Download installer with progress
        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: format!("Downloading Forge {}...", forge_version),
                percentage: Some(0.0),
            },
        );

        let mut response = reqwest::get(&installer_url)
            .await
            .map_err(|e| format!("Failed to download Forge installer: {}", e))?;

        let total_size = response.content_length().unwrap_or(0) as f64;
        let mut bytes: Vec<u8> = Vec::new();
        let mut downloaded = 0.0_f64;

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("Failed to read Forge installer chunk: {}", e))?
        {
            bytes.extend_from_slice(&chunk);
            downloaded += chunk.len() as f64;
            if total_size > 0.0 {
                let _ = handle.emit(
                    "creation-progress",
                    ProgressPayload {
                        process: format!("Downloading Forge {}...", forge_version),
                        percentage: Some((downloaded / total_size) * 100.0),
                    },
                );
            }
        }

        let server_path = server.get_path()?;
        let installer_path = server_path.join(&installer_name);
        fs::write(&installer_path, bytes)
            .await
            .map_err(|e| format!("Failed to save Forge installer: {}", e))?;

        // Read Java path (must be installed first)
        let java_path_file = server_path.join("java_path.txt");
        let java_path = fs::read_to_string(&java_path_file)
            .await
            .map_err(|e| format!("Java not found (install Java before Forge): {}", e))?;

        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Running Forge installer (this may take a while)...".to_string(),
                percentage: None,
            },
        );

        let status = tokio::process::Command::new(java_path.trim())
            .arg("-jar")
            .arg(installer_path.as_os_str())
            .arg("--installServer")
            .current_dir(&server_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .status()
            .await
            .map_err(|e| format!("Failed to run Forge installer: {}", e))?;

        // Clean up installer regardless of success
        let _ = fs::remove_file(&installer_path).await;

        if !status.success() {
            return Err(format!(
                "Forge installer failed (exit code {:?}). Check that the MC version is correct.",
                status.code()
            ));
        }

        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Forge installed successfully.".to_string(),
                percentage: Some(100.0),
            },
        );

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
