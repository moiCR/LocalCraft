pub mod adoptium;
mod cache;
mod download;
mod extract;
pub mod java_installation;

use std::{collections::HashMap, future::Future, path::PathBuf, sync::RwLock, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::Client;
use tokio::{
    fs,
    sync::{Mutex, watch},
};

use crate::instance::{InstancesService, ServerInstance};
pub use java_installation::JavaInstallation;

#[derive(Debug, Clone, Copy)]
pub enum ArchiveFormat {
    Zip,
    TarGz,
}

#[derive(Debug, Clone)]
pub struct JavaDownload {
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub format: ArchiveFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaStage {
    Resolving,
    Cached,
    Downloading,
    Verifying,
    Extracting,
    Linking,
    Complete,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct JavaProgress {
    pub version: u8,
    pub stage: JavaStage,
    pub downloaded: u64,
    pub total: Option<u64>,
}

impl JavaProgress {
    pub fn new(version: u8) -> Self {
        Self {
            version,
            stage: JavaStage::Resolving,
            downloaded: 0,
            total: None,
        }
    }
}

pub trait Java {
    fn get_archive(
        &self,
        client: &Client,
        version: u8,
    ) -> impl Future<Output = Result<JavaDownload>> + Send;
}

#[derive(Default)]
pub struct JavaService {
    installations: RwLock<HashMap<String, JavaInstallation>>,
    operations: Mutex<()>,
}

impl JavaService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn installations(&self) -> &RwLock<HashMap<String, JavaInstallation>> {
        &self.installations
    }

    pub fn directory() -> Result<PathBuf> {
        Ok(dirs::data_dir()
            .context("Could not find data directory")?
            .join("LocalCraft")
            .join("java"))
    }

    pub async fn get_archive(&self, version: u8) -> Result<JavaDownload> {
        adoptium::AdoptiumJava::new()
            .get_archive(&client()?, version)
            .await
    }

    pub async fn load(&self) -> Result<()> {
        let _operation = self.operations.lock().await;
        let root = Self::directory()?;
        let installations = cache::load(&root).await?;
        *self
            .installations
            .write()
            .map_err(|_| anyhow::anyhow!("Java installation registry is poisoned"))? =
            installations;
        Ok(())
    }

    /// Run on the dedicated Tokio runtime; cached versions are reused without network requests.
    pub async fn download(
        &self,
        version: u8,
        progress: &watch::Sender<JavaProgress>,
    ) -> Result<JavaInstallation> {
        progress.send_replace(JavaProgress::new(version));
        let result = self.install(version, progress).await;
        finish(progress, &result);
        result
    }

    pub async fn install_for_instance(
        &self,
        server: &ServerInstance,
        progress: &watch::Sender<JavaProgress>,
    ) -> Result<JavaInstallation> {
        let result = async {
            let version = server
                .java_version
                .as_deref()
                .context("Select a Java version for the instance")?
                .parse::<u8>()
                .context("Java version must be a major version number")?;
            progress.send_replace(JavaProgress::new(version));
            let running = server.running.lock().await;
            if running
                .as_ref()
                .is_some_and(|process| !*process.finished.borrow())
            {
                bail!("Stop the server before changing its Java runtime");
            }
            let installation = self.install(version, progress).await?;
            progress.send_modify(|value| value.stage = JavaStage::Linking);
            let binary = installation
                .binary_path()
                .to_str()
                .context("Java executable path is not valid UTF-8")?;
            cache::atomic_write(
                &server.directory()?.join("java_path.txt"),
                binary.as_bytes(),
            )
            .await?;
            Ok(installation)
        }
        .await;
        finish(progress, &result);
        result
    }

    async fn install(
        &self,
        version: u8,
        progress: &watch::Sender<JavaProgress>,
    ) -> Result<JavaInstallation> {
        validate_version(version)?;
        let _operation = self.operations.lock().await;
        let root = Self::directory()?;
        if let Some(installation) = cache::read(&root, version).await? {
            progress.send_modify(|value| value.stage = JavaStage::Cached);
            self.register(installation.clone())?;
            return Ok(installation);
        }
        let client = client()?;
        let archive = adoptium::AdoptiumJava::new()
            .get_archive(&client, version)
            .await?;
        let installation = self
            .install_archive(&root, version, &client, &archive, progress)
            .await?;
        self.register(installation.clone())?;
        Ok(installation)
    }

    async fn install_archive(
        &self,
        root: &std::path::Path,
        version: u8,
        client: &Client,
        archive: &JavaDownload,
        progress: &watch::Sender<JavaProgress>,
    ) -> Result<JavaInstallation> {
        fs::create_dir_all(root).await?;
        let staging = root.join(format!(".install-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&staging).await?;
        let result = async {
            let archive_path = staging.join("runtime.archive");
            download::save(client, archive, &archive_path, progress).await?;
            let extracted = staging.join("runtime");
            fs::create_dir(&extracted).await?;
            progress.send_modify(|value| value.stage = JavaStage::Extracting);
            let extraction_root = extracted.clone();
            let format = archive.format;
            let binary = tokio::task::spawn_blocking(move || {
                extract::extract(&archive_path, &extraction_root, format)
            })
            .await
            .context("Java extraction task failed")??;
            cache::write(&extracted, version, &binary, &archive.sha256).await?;
            let destination = root.join(version.to_string());
            // Publish only a complete runtime, leaving failed installs outside the cache.
            fs::rename(&extracted, &destination)
                .await
                .context("Could not publish Java installation")?;
            Ok(JavaInstallation::new(
                version,
                "Adoptium".into(),
                destination.join(binary),
            ))
        }
        .await;
        let _ = fs::remove_dir_all(staging).await;
        result
    }

    fn register(&self, installation: JavaInstallation) -> Result<()> {
        self.installations
            .write()
            .map_err(|_| anyhow::anyhow!("Java installation registry is poisoned"))?
            .insert(installation.major_version.to_string(), installation);
        Ok(())
    }

    pub async fn delete(&self, version: u8) -> Result<()> {
        validate_version(version)?;
        let _operation = self.operations.lock().await;
        let instances = InstancesService::load().await?;
        if instances.servers().iter().any(|server| {
            server
                .java_version
                .as_deref()
                .and_then(|value| value.parse::<u8>().ok())
                == Some(version)
        }) {
            bail!("Java {version} is still assigned to a server instance");
        }
        let path = Self::directory()?.join(version.to_string());
        if !fs::try_exists(&path).await? {
            bail!("Java {version} is not installed");
        }
        fs::remove_dir_all(path)
            .await
            .context("Could not delete Java installation")?;
        self.installations
            .write()
            .map_err(|_| anyhow::anyhow!("Java installation registry is poisoned"))?
            .remove(&version.to_string());
        Ok(())
    }
}

fn validate_version(version: u8) -> Result<()> {
    if version < 8 {
        bail!("Java major version must be at least 8");
    }
    Ok(())
}

fn finish(progress: &watch::Sender<JavaProgress>, result: &Result<JavaInstallation>) {
    progress.send_modify(|value| {
        value.stage = match result {
            Ok(_) => JavaStage::Complete,
            Err(error) => JavaStage::Failed(format!("{error:#}")),
        };
    });
}

fn client() -> Result<Client> {
    Client::builder()
        .https_only(true)
        .user_agent(concat!(
            "LocalCraft/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/moiCR/LocalCraft-GPUI)"
        ))
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(600))
        .build()
        .context("Could not initialize Java download client")
}
