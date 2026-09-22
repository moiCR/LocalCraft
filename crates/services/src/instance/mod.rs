pub mod server_instance;
mod supervisor;

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use tokio::fs;

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast, mpsc, watch};

pub const CONSOLE_CAPACITY: usize = 2_000;

#[derive(Debug, Clone)]
pub enum ServerEvent {
    Console(Vec<String>),
    Exited(Option<i32>),
    Error(String),
}

pub struct RunningServer {
    pub pid: Option<u32>,
    commands: mpsc::Sender<supervisor::Request>,
    pub(crate) finished: watch::Receiver<bool>,
}

fn event_channel() -> broadcast::Sender<ServerEvent> {
    broadcast::channel(32).0
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerInstance {
    pub id: String,
    pub name: String,
    pub version: String,
    pub software: String,
    /// Maximum heap in MiB.
    pub ram: String,
    pub java_version: Option<String>,
    #[serde(skip, default)]
    pub(crate) running: Arc<Mutex<Option<RunningServer>>>,
    #[serde(skip, default)]
    console: Arc<Mutex<VecDeque<String>>>,
    #[serde(skip, default = "event_channel")]
    events: broadcast::Sender<ServerEvent>,
}

pub struct InstancesService {
    servers: Vec<ServerInstance>,
}

impl InstancesService {
    pub fn servers(&self) -> &[ServerInstance] {
        &self.servers
    }

    pub async fn new() -> Result<Self> {
        Self::load().await
    }

    pub fn runtime() -> Result<tokio::runtime::Runtime> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("localcraft-server")
            .build()
            .context("Could not initialize the server runtime")
    }

    pub async fn shutdown(&self, timeout: Duration) -> Result<()> {
        let mut tasks = tokio::task::JoinSet::new();
        for instance in &self.servers {
            let instance = instance.clone();
            tasks.spawn(async move { instance.stop(timeout).await });
        }
        let mut errors = Vec::new();
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => errors.push(error.to_string()),
                Err(error) => errors.push(error.to_string()),
            }
        }
        if !errors.is_empty() {
            bail!("Could not shut down all servers: {}", errors.join("; "));
        }
        Ok(())
    }

    pub fn directory() -> Result<PathBuf> {
        Ok(dirs::data_dir()
            .context("Could not find data directory")?
            .join("LocalCraft")
            .join("instances"))
    }

    pub async fn load() -> Result<Self> {
        Self::load_from(&Self::directory()?).await
    }

    pub(super) async fn load_from(root: &Path) -> Result<Self> {
        let mut entries = match fs::read_dir(root).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    servers: Vec::new(),
                });
            }
            Err(error) => return Err(error).context("Could not list instances"),
        };
        let mut instances = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }
            let path = entry.path().join("config.json");
            let bytes = fs::read(&path)
                .await
                .with_context(|| format!("Could not read {}", path.display()))?;
            let instance: ServerInstance = serde_json::from_slice(&bytes)
                .with_context(|| format!("Invalid configuration: {}", path.display()))?;
            instance.validate()?;
            if entry.file_name() != std::ffi::OsStr::new(&instance.id) {
                bail!("Instance ID does not match its directory");
            }
            instances.push(instance);
        }
        instances.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        Ok(Self { servers: instances })
    }
}
