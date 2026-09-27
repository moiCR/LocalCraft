use super::{ServerInstance, server_instance::atomic_write};
use crate::{
    java::{JavaProgress, JavaService},
    software::{DownloadProgress, JarDownload, SoftwareService},
};
use anyhow::{Context, Result, bail};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::watch;

pub fn default_port() -> u16 {
    25565
}

pub struct CreateServer {
    pub name: String,
    pub version: String,
    pub software: String,
    pub build: String,
    pub ram: u32,
    pub port: u16,
    pub java: u8,
    pub accepted_eula: bool,
    pub download: JarDownload,
}

impl CreateServer {
    pub fn validate(&self) -> Result<()> {
        validate_settings(&self.name, self.ram, self.port)?;
        if !self.accepted_eula {
            bail!("Accept the Minecraft EULA to create this server");
        }
        if self.download.sha256.as_ref().is_some_and(|checksum| {
            checksum.len() != 64 || !checksum.bytes().all(|b| b.is_ascii_hexdigit())
        }) {
            bail!("Provider SHA256 must contain 64 hexadecimal characters");
        }
        Ok(())
    }

    pub async fn create(
        self,
        java: Arc<JavaService>,
        java_progress: &watch::Sender<JavaProgress>,
        progress: &watch::Sender<DownloadProgress>,
    ) -> Result<ServerInstance> {
        self.validate()?;
        let mut server = ServerInstance::create(
            self.name,
            self.version,
            self.software,
            self.ram.to_string(),
            Some(self.java.to_string()),
        )
        .await?;
        server.port = self.port;
        server.build = self.build;
        let directory = server.directory()?;
        let result = async {
            tokio::fs::rename(
                directory.join("config.json"),
                directory.join("config.pending"),
            )
            .await?;
            let runtime = java.install_for_instance(&server, java_progress).await?;
            SoftwareService::new()
                .download_resolved(
                    &server,
                    runtime.binary_path(),
                    None,
                    progress,
                    &self.download,
                )
                .await?;
            server.write_port().await?;
            server.accept_eula().await?;
            server.save().await?;
            let _ = tokio::fs::remove_file(directory.join("config.pending")).await;
            Ok(())
        }
        .await;
        if let Err(error) = result {
            let _ = tokio::fs::remove_dir_all(directory).await;
            return Err(error);
        }
        Ok(server)
    }
}

pub fn validate_settings(name: &str, ram: u32, port: u16) -> Result<()> {
    if name.trim().is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        bail!("Use a server name between 1 and 80 characters");
    }
    if !(512..=262144).contains(&ram) {
        bail!("RAM must be between 512 and 262144 MiB");
    }
    if port == 0 {
        bail!("Port must be between 1 and 65535");
    }
    Ok(())
}

impl ServerInstance {
    pub async fn java_binary(&self) -> Result<PathBuf> {
        let path = tokio::fs::read_to_string(self.directory()?.join("java_path.txt"))
            .await
            .context("Java runtime is not linked to this server")?;
        Ok(PathBuf::from(path.trim()))
    }
    pub async fn update_settings(&mut self, name: String, ram: u32, port: u16) -> Result<()> {
        validate_settings(&name, ram, port)?;
        let lifecycle = self.running.clone();
        let running = lifecycle.lock().await;
        if running
            .as_ref()
            .is_some_and(|process| !*process.finished.borrow())
        {
            bail!("Stop the server before changing its settings");
        }
        self.name = name.trim().to_owned();
        self.ram = ram.to_string();
        self.port = port;
        self.write_port().await?;
        self.save().await
    }
    async fn write_port(&self) -> Result<()> {
        let path = self.directory()?.join("server.properties");
        let contents = match tokio::fs::read_to_string(&path).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        let mut lines = contents
            .lines()
            .filter(|line| !line.trim_start().starts_with("server-port="))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        lines.push(format!("server-port={}", self.port));
        atomic_write(&path, (lines.join("\n") + "\n").as_bytes()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::software::JarKind;
    #[test]
    fn creation_requires_eula_but_allows_missing_checksum() {
        let mut draft = CreateServer {
            name: "Test".into(),
            version: "1.21.4".into(),
            software: "Paper".into(),
            build: "100".into(),
            ram: 2048,
            port: 25565,
            java: 21,
            accepted_eula: false,
            download: JarDownload {
                url: "https://example.invalid/server.jar".into(),
                sha256: Some("a".repeat(64)),
                size: None,
                kind: JarKind::Server,
            },
        };
        assert!(draft.validate().is_err());
        draft.accepted_eula = true;
        assert!(draft.validate().is_ok());
        draft.download.sha256 = None;
        assert!(draft.validate().is_ok());
    }
    #[test]
    fn invalid_settings_are_rejected() {
        assert!(validate_settings(" ", 2048, 25565).is_err());
        assert!(validate_settings("world\nother", 2048, 25565).is_err());
        assert!(validate_settings("world", 0, 25565).is_err());
        assert!(validate_settings("world", 2048, 0).is_err());
        assert!(validate_settings("world", 2048, 25565).is_ok());
    }
}
