pub mod catalog;
mod download;
pub mod fabric;
pub mod forge;
pub mod paper;
pub mod pumpkin;
pub mod vanilla;

use std::{future::Future, path::Path, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::Client;
use tokio::sync::watch;

use crate::instance::ServerInstance;

#[derive(Debug, Clone)]
pub struct JarDownload {
    pub url: String,
    pub sha256: Option<String>,
    pub size: Option<u64>,
    pub kind: JarKind,
}

#[derive(Debug, Clone)]
pub enum JarKind {
    Server,
    NativeServer { filename: String },
    ForgeInstaller { version: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStage {
    Resolving,
    Downloading,
    Verifying,
    Installing,
    Complete,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub instance_id: String,
    pub stage: DownloadStage,
    pub downloaded: u64,
    pub total: Option<u64>,
}

pub trait Software {
    fn get_jar(
        &self,
        client: &Client,
        version: &str,
    ) -> impl Future<Output = Result<JarDownload>> + Send;
}

#[derive(Default)]
pub struct SoftwareService;

impl SoftwareService {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_jar(&self, software: &str, version: &str) -> Result<JarDownload> {
        self.resolve(&client()?, software, version).await
    }

    async fn resolve(&self, client: &Client, software: &str, version: &str) -> Result<JarDownload> {
        validate_version(version)?;
        match software.to_ascii_lowercase().as_str() {
            "paper" => paper::PaperSoftware::new().get_jar(client, version).await,
            "vanilla" => {
                vanilla::VanillaSoftware::new()
                    .get_jar(client, version)
                    .await
            }
            "fabric" => fabric::FabricSoftware::new().get_jar(client, version).await,
            "forge" => forge::ForgeSoftware::new().get_jar(client, version).await,
            "pumpkin" => pumpkin::build(client, version)
                .await
                .map(|(_, download)| download),
            _ => bail!("Unsupported server software: {software}"),
        }
    }

    /// Run on Tokio. Subscribe before downloading; watch retains the latest progress.
    /// An expected SHA256 can be supplied when the provider does not publish one.
    pub async fn download(
        &self,
        server: &ServerInstance,
        java: Option<&Path>,
        expected_sha256: Option<&str>,
        progress: &watch::Sender<DownloadProgress>,
    ) -> Result<()> {
        progress.send_replace(DownloadProgress {
            instance_id: server.id.clone(),
            stage: DownloadStage::Resolving,
            downloaded: 0,
            total: None,
        });
        let result = self.install(server, java, expected_sha256, progress).await;
        progress.send_modify(|value| {
            value.stage = match &result {
                Ok(()) => DownloadStage::Complete,
                Err(error) => DownloadStage::Failed(format!("{error:#}")),
            };
        });
        result
    }

    async fn install(
        &self,
        server: &ServerInstance,
        java: Option<&Path>,
        expected_sha256: Option<&str>,
        progress: &watch::Sender<DownloadProgress>,
    ) -> Result<()> {
        // Hold the lifecycle lock so start cannot race with replacement of an installed jar.
        let running = server.running.lock().await;
        if running
            .as_ref()
            .is_some_and(|process| !*process.finished.borrow())
        {
            bail!("Stop the server before installing software");
        }
        let client = client()?;
        let jar = self
            .resolve(&client, &server.software, &server.version)
            .await?;
        self.install_resolved(server, java, expected_sha256, progress, &jar, &client)
            .await
    }

    pub async fn download_resolved(
        &self,
        server: &ServerInstance,
        java: Option<&Path>,
        expected_sha256: Option<&str>,
        progress: &watch::Sender<DownloadProgress>,
        jar: &JarDownload,
    ) -> Result<()> {
        let running = server.running.lock().await;
        if running
            .as_ref()
            .is_some_and(|process| !*process.finished.borrow())
        {
            bail!("Stop the server before installing software");
        }
        let result = self
            .install_resolved(server, java, expected_sha256, progress, jar, &client()?)
            .await;
        progress.send_modify(|value| {
            value.stage = match &result {
                Ok(()) => DownloadStage::Complete,
                Err(error) => DownloadStage::Failed(format!("{error:#}")),
            }
        });
        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn install_resolved(
        &self,
        server: &ServerInstance,
        java: Option<&Path>,
        expected_sha256: Option<&str>,
        progress: &watch::Sender<DownloadProgress>,
        jar: &JarDownload,
        client: &Client,
    ) -> Result<()> {
        let hash = jar.sha256.as_deref().or(expected_sha256);
        if let (Some(published), Some(expected)) = (jar.sha256.as_deref(), expected_sha256)
            && !published.eq_ignore_ascii_case(expected)
        {
            bail!("Expected SHA256 differs from provider metadata");
        }
        let directory = server.directory()?;
        match &jar.kind {
            JarKind::Server => {
                download::save(client, jar, hash, &directory.join("server.jar"), progress).await?
            }
            JarKind::NativeServer { filename } => {
                if Path::new(filename)
                    .file_name()
                    .and_then(|name| name.to_str())
                    != Some(filename.as_str())
                {
                    bail!("Invalid native server executable name");
                }
                download::save(client, jar, hash, &directory.join(filename), progress).await?
            }
            JarKind::ForgeInstaller { version } => {
                let java = java.context("Select a Java executable before installing Forge")?;
                let java = tokio::fs::canonicalize(java)
                    .await
                    .context("Could not resolve Java executable")?;
                let installer = directory.join("forge-installer.jar");
                download::save(client, jar, hash, &installer, progress).await?;
                progress.send_modify(|value| value.stage = DownloadStage::Installing);
                let result = forge::install(&java, &installer, &directory, version).await;
                let _ = tokio::fs::remove_file(&installer).await;
                result?;
            }
        }
        Ok(())
    }
}

fn client() -> Result<Client> {
    Client::builder()
        .user_agent(concat!(
            "LocalCraft/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/moiCR/LocalCraft-GPUI)"
        ))
        .https_only(true)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(600))
        .build()
        .context("Could not initialize software download client")
}

fn validate_version(version: &str) -> Result<()> {
    if version.is_empty()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-+".contains(&byte))
    {
        bail!("Invalid software version: {version}");
    }
    Ok(())
}
