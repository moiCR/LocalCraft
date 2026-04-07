use std::env;
use std::fs;
use std::path::Path;

use directories::ProjectDirs;
use tauri::{AppHandle, Emitter};

use crate::server::{server::Server, server_manager::ProgressPayload};

pub struct JavaManager {}

impl JavaManager {
    pub async fn install_java(handle: &AppHandle, server: &Server) -> Result<String, String> {
        let current_server = server.clone();

        let java_version = current_server
            .java_version
            .ok_or("Java version not found in server config")?;
        let server_id = current_server.id.to_string();

        let project_dirs = ProjectDirs::from("com", "localcraft", "LocalCraft")
            .ok_or("Failed to resolve project directories")?;

        let data_dir = project_dirs.data_dir();
        let global_java_dir = data_dir.join("java");
        let java_version_dir = global_java_dir.join(java_version.to_string());

        let server_dir = data_dir.join("servers").join(&server_id);
        let server_java_path_file = server_dir.join("java_path.txt");

        let os_name = env::consts::OS;
        let arch = env::consts::ARCH;

        let adoptium_os = match os_name {
            "windows" => "windows",
            "macos" => "mac",
            "linux" => "linux",
            _ => "linux",
        };

        let adoptium_arch = match arch {
            "x86_64" => "x64",
            "aarch64" => "aarch64",
            _ => "x64",
        };

        let java_exe_name = if os_name == "windows" {
            "java.exe"
        } else {
            "java"
        };

        if server_java_path_file.exists() {
            let saved_path = fs::read_to_string(&server_java_path_file).unwrap_or_default();
            if Path::new(&saved_path).exists() && saved_path.contains(&java_version.to_string()) {
                let _ = handle.emit(
                    "creation-progress",
                    ProgressPayload {
                        process: format!("Using cached Java {}...", java_version),
                        percentage: Some(100.0),
                    },
                );
                return Ok(saved_path);
            }
        }

        if java_version_dir.exists() {
            let mut java_exe_path = None;
            for entry in walkdir::WalkDir::new(&java_version_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name() == std::ffi::OsStr::new(java_exe_name) {
                    java_exe_path = Some(entry.path().to_string_lossy().to_string());
                    break;
                }
            }
            if let Some(path) = java_exe_path {
                fs::create_dir_all(&server_dir).unwrap_or_default();
                fs::write(&server_java_path_file, &path).unwrap_or_default();
                let _ = handle.emit(
                    "creation-progress",
                    ProgressPayload {
                        process: format!("Using cached Java {}...", java_version),
                        percentage: Some(100.0),
                    },
                );
                return Ok(path);
            }
        }

        let java_url = format!(
            "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jre/hotspot/normal/eclipse",
            java_version, adoptium_os, adoptium_arch
        );

        let mut response = reqwest::get(&java_url)
            .await
            .map_err(|e| format!("Failed to download Java: {}", e))?;

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
                let percentage = (downloaded / total_size) * 100.0;
                let _ = handle.emit(
                    "creation-progress",
                    ProgressPayload {
                        process: format!("Downloading Java {}...", java_version),
                        percentage: Some(percentage),
                    },
                );
            }
        }

        fs::create_dir_all(&java_version_dir).unwrap_or_default();
        let mut java_exe_path = None;

        if os_name == "windows" {
            let cursor = std::io::Cursor::new(bytes);
            let mut archive = zip::ZipArchive::new(cursor)
                .map_err(|e| format!("Failed to read zip archive: {}", e))?;
            let total_files = archive.len() as f64;

            for i in 0..archive.len() {
                if i % 50 == 0 || i == archive.len() - 1 {
                    let pct = ((i + 1) as f64 / total_files) * 100.0;
                    let _ = handle.emit(
                        "creation-progress",
                        ProgressPayload {
                            process: "Extracting Java environment...".to_string(),
                            percentage: Some(pct),
                        },
                    );
                }

                let mut file = archive
                    .by_index(i)
                    .map_err(|e| format!("Zip extraction error: {}", e))?;

                let outpath = match file.enclosed_name() {
                    Some(path) => java_version_dir.join(path),
                    None => continue,
                };

                if (*file.name()).ends_with('/') {
                    fs::create_dir_all(&outpath).unwrap_or_default();
                } else {
                    if let Some(p) = outpath.parent() {
                        fs::create_dir_all(p).unwrap_or_default();
                    }
                    let mut outfile = std::fs::File::create(&outpath).unwrap();
                    std::io::copy(&mut file, &mut outfile).unwrap();

                    if outpath.file_name() == Some(std::ffi::OsStr::new(java_exe_name)) {
                        java_exe_path = Some(outpath.to_string_lossy().to_string());
                    }
                }
            }
        } else {
            use flate2::read::GzDecoder;
            use tar::Archive;

            #[cfg(unix)]
            use std::os::unix::fs::PermissionsExt;

            let cursor = std::io::Cursor::new(bytes);
            let tar = GzDecoder::new(cursor);
            let mut archive = Archive::new(tar);

            let _ = handle.emit(
                "creation-progress",
                ProgressPayload {
                    process: "Extracting Java environment...".to_string(),
                    percentage: None,
                },
            );

            for file_result in archive.entries().map_err(|e| format!("Tar error: {}", e))? {
                let mut file = file_result.map_err(|e| format!("Archive entry error: {}", e))?;
                let path = file
                    .path()
                    .map_err(|e| format!("Path extraction error: {}", e))?;
                let outpath = java_version_dir.join(path);

                if file.header().entry_type().is_dir() {
                    fs::create_dir_all(&outpath).unwrap_or_default();
                } else {
                    if let Some(p) = outpath.parent() {
                        fs::create_dir_all(p).unwrap_or_default();
                    }
                    file.unpack(&outpath)
                        .map_err(|e| format!("Unpack file error: {}", e))?;

                    if outpath.file_name() == Some(std::ffi::OsStr::new(java_exe_name)) {
                        java_exe_path = Some(outpath.to_string_lossy().to_string());

                        #[cfg(unix)]
                        {
                            if let Ok(metadata) = fs::metadata(&outpath) {
                                let mut perms = metadata.permissions();
                                perms.set_mode(0o755);
                                let _ = fs::set_permissions(&outpath, perms);
                            }
                        }
                    }
                }
            }
        }

        let final_path = java_exe_path.ok_or("Java executable not found in downloaded archive")?;

        fs::create_dir_all(&server_dir).unwrap_or_default();
        fs::write(&server_java_path_file, &final_path).unwrap_or_default();

        let _ = handle.emit(
            "creation-progress",
            ProgressPayload {
                process: "Java linked successfully!".to_string(),
                percentage: Some(100.0),
            },
        );

        Ok(final_path)
    }
}
