use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, process::Stdio, sync::Arc};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct RunningServer {
    pub stdin: ChildStdin,
    pub child: Child,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub version: String,
    pub software: String,
    pub ram: String,
    pub java_version: Option<String>,

    #[serde(skip, default)]
    pub running: Arc<Mutex<Option<RunningServer>>>,
}

#[derive(Clone, Serialize)]
struct LogPayload {
    id: String,
    line: String,
}

impl Server {
    pub async fn create(
        _handle: &AppHandle,
        name: String,
        version: String,
        software: String,
        java_version: String,
        ram: String,
    ) -> Result<Self, String> {
        let id = Uuid::new_v4().to_string();

        let proj_dirs = ProjectDirs::from("com", "localcraft", "LocalCraft")
            .ok_or("Failed to determine home directory")?;
        let server_dir = proj_dirs.data_dir().join("servers").join(&id);

        if server_dir.exists() {
            return Err("Directory already exists (UUID Collision)".into());
        }
        fs::create_dir_all(&server_dir).map_err(|e| format!("Error creating directory: {}", e))?;

        let server = Self {
            id,
            name,
            version,
            software,
            java_version: Some(java_version),
            ram,
            running: Arc::new(Mutex::new(None)),
        };

        server.save_json()?;

        fs::write(server_dir.join("eula.txt"), "eula=true\n")
            .map_err(|e| format!("Error creating eula.txt: {}", e))?;

        Ok(server)
    }

    pub fn save_json(&self) -> Result<(), String> {
        let proj_dirs = ProjectDirs::from("com", "localcraft", "LocalCraft")
            .ok_or("Failed to determine home directory")?;
        let server_dir = proj_dirs.data_dir().join("servers").join(&self.id);

        if !server_dir.exists() {
            return Err("Directory does not exist (UUID Collision)".into());
        }
        fs::create_dir_all(&server_dir).map_err(|e| format!("Error creating directory: {}", e))?;

        let config_path = server_dir.join("server.json");
        let json = serde_json::to_string_pretty(&self)
            .map_err(|e| format!("Serialization error: {}", e))?;

        fs::write(config_path, json).map_err(|e| format!("Error writing server.json: {}", e))?;

        Ok(())
    }

    pub async fn start(&self, handle: &AppHandle) -> Result<(), String> {
        {
            let running_guard = self.running.lock().await;
            if running_guard.is_some() {
                return Err("Server is already running!".to_string());
            }
        }

        let dir = self.get_path()?;

        let ram_arg = format!("-Xmx{}M", self.ram);

        // Case 1: Modern Forge (1.17+) — installer creates run.bat / run.sh
        let forge_script = if cfg!(target_os = "windows") {
            dir.join("run.bat")
        } else {
            dir.join("run.sh")
        };

        // Case 2: Old Forge (pre-1.17) — installer creates forge-{mc}-{forge}.jar in server dir
        let old_forge_jar = fs::read_dir(&dir).ok().and_then(|entries| {
            entries
                .filter_map(|e| e.ok())
                .find(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    n.starts_with("forge-") && n.ends_with(".jar") && !n.contains("installer")
                })
                .map(|e| e.path())
        });

        let java_path_file = dir.join("java_path.txt");
        let java_path_str = fs::read_to_string(&java_path_file)
            .map_err(|e| format!("Failed to read java path: {}", e))?;

        let mut command = if forge_script.exists() {
            // Modern Forge: write RAM into user_jvm_args.txt so Forge picks it up,
            // then run the generated launch script.
            let jvm_args_path = dir.join("user_jvm_args.txt");
            let mut jvm_content = fs::read_to_string(&jvm_args_path).unwrap_or_default();
            if !jvm_content.contains("-Xmx") {
                jvm_content.push_str(&format!("\n{}\n", ram_arg));
                fs::write(&jvm_args_path, &jvm_content)
                    .map_err(|e| format!("Failed to write user_jvm_args.txt: {}", e))?;
            }
            if cfg!(target_os = "windows") {
                let mut cmd = Command::new("cmd");
                cmd.arg("/c").arg("run.bat").arg("nogui");
                cmd
            } else {
                let mut cmd = Command::new("bash");
                cmd.arg("run.sh").arg("nogui");
                cmd
            }
        } else if let Some(ref forge_jar) = old_forge_jar {
            // Old Forge: run the forge universal jar directly
            let mut cmd = Command::new(java_path_str.trim());
            cmd.arg(&ram_arg).arg("-jar").arg(forge_jar).arg("nogui");
            cmd
        } else {
            // Standard: Paper / Vanilla / Fabric use server.jar
            let server_jar = dir.join("server.jar");
            if !server_jar.exists() {
                return Err(
                    "server.jar not found. Please ensure the server was created correctly."
                        .to_string(),
                );
            }
            let mut cmd = Command::new(java_path_str.trim());
            cmd.arg(&ram_arg).arg("-jar").arg("server.jar").arg("nogui");
            cmd
        };
        command
            .current_dir(&dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .kill_on_drop(true);

        #[cfg(target_os = "windows")]
        {
            command.creation_flags(0x08000000);
        }

        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn server process: {}", e))?;

        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to open stderr")?;

        let running_server = RunningServer { stdin, child };

        {
            let mut running_guard = self.running.lock().await;
            *running_guard = Some(running_server);
        }

        let handle_clone = handle.clone();
        let id_clone = self.id.clone();
        let running_clone = self.running.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = handle_clone.emit(
                    "server-log",
                    LogPayload {
                        id: id_clone.clone(),
                        line,
                    },
                );
            }

            // Server stdout closed, which means it stopped.
            {
                let mut guard = running_clone.lock().await;
                *guard = None;
            }
            let _ = handle_clone.emit("server-stopped", id_clone.clone());
        });

        // Hilo para leer Stderr
        let handle_clone_err = handle.clone();
        let id_clone_err = self.id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = handle_clone_err.emit(
                    "server-log",
                    LogPayload {
                        id: id_clone_err.clone(),
                        line: format!("[ERROR] {}", line),
                    },
                );
            }

            // Opcional: Aquí podrías emitir un evento extra indicando que el proceso terminó
            let _ = handle_clone_err.emit(
                "server-log",
                LogPayload {
                    id: id_clone_err.clone(),
                    line: "Server process finished.".to_string(),
                },
            );
        });

        Ok(())
    }

    pub async fn send_command(&self, command: String) -> Result<(), String> {
        let mut running_guard = self.running.lock().await;

        if let Some(running_server) = running_guard.as_mut() {
            let cmd_with_newline = format!("{}\n", command);
            running_server
                .stdin
                .write_all(cmd_with_newline.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to stdin: {}", e))?;

            running_server
                .stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush stdin: {}", e))?;

            Ok(())
        } else {
            Err("Server is not running!".to_string())
        }
    }

    pub async fn delete(&self) -> Result<(), String> {
        let proj_dirs =
            ProjectDirs::from("com", "localcraft", "LocalCraft").ok_or("Path not found")?;

        let server_dir = proj_dirs.data_dir().join("servers").join(&self.id);

        if !server_dir.exists() {
            return Err(format!("Server directory not found {}", &self.id));
        }

        {
            let mut running_guard = self.running.lock().await;
            *running_guard = None;
        }

        fs::remove_dir_all(server_dir)
            .map_err(|e| format!("Failed to delete server directory: {}", e))?;

        Ok(())
    }

    pub fn get_path(&self) -> Result<std::path::PathBuf, String> {
        let proj_dirs =
            ProjectDirs::from("com", "localcraft", "LocalCraft").ok_or("Path not found")?;
        Ok(proj_dirs.data_dir().join("servers").join(&self.id))
    }

    pub async fn wait_until_stopped(&self) {
        loop {
            {
                let guard = self.running.lock().await;
                if guard.is_none() {
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}
