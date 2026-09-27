use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use tokio::{
    fs,
    io::AsyncWriteExt,
    sync::{Mutex, broadcast, mpsc, oneshot, watch},
};
use uuid::Uuid;

use super::{
    InstancesService, RunningServer, ServerEvent, ServerInstance, event_channel, supervisor,
};

impl ServerInstance {
    pub async fn create(
        name: String,
        version: String,
        software: String,
        ram: String,
        java_version: Option<String>,
    ) -> Result<Self> {
        Self::create_in(
            &InstancesService::directory()?,
            name,
            version,
            software,
            ram,
            java_version,
        )
        .await
    }

    async fn create_in(
        root: &Path,
        name: String,
        version: String,
        software: String,
        ram: String,
        java_version: Option<String>,
    ) -> Result<Self> {
        let instance = Self {
            id: Uuid::new_v4().to_string(),
            name,
            version,
            software,
            ram,
            java_version,
            port: 25565,
            build: String::new(),
            running: Arc::new(Mutex::new(None)),
            console: super::console::channel(),
            events: event_channel(),
        };
        instance.validate()?;
        fs::create_dir_all(root)
            .await
            .context("Could not create instances directory")?;
        let dir = root.join(&instance.id);
        fs::create_dir(&dir)
            .await
            .context("Could not create instance directory")?;
        let result = async {
            atomic_write(&dir.join("eula.txt"), b"eula=false\n").await?;
            atomic_write(
                &dir.join("config.json"),
                &serde_json::to_vec_pretty(&instance)?,
            )
            .await
        }
        .await;
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&dir).await;
            return Err(error);
        }
        Ok(instance)
    }

    pub(super) fn validate(&self) -> Result<()> {
        Uuid::parse_str(&self.id).context("Invalid instance ID")?;
        if self.name.trim().is_empty()
            || self.version.trim().is_empty()
            || self.software.trim().is_empty()
        {
            bail!("Name, version and software must not be empty");
        }
        if self
            .ram
            .parse::<u32>()
            .context("RAM must be a positive number in MiB")?
            == 0
        {
            bail!("RAM must be greater than zero");
        }
        Ok(())
    }

    pub fn directory(&self) -> Result<PathBuf> {
        self.validate()?;
        Ok(InstancesService::directory()?.join(&self.id))
    }

    pub async fn save(&self) -> Result<()> {
        atomic_write(
            &self.directory()?.join("config.json"),
            &serde_json::to_vec_pretty(self)?,
        )
        .await
    }

    pub async fn delete(&self) -> Result<()> {
        self.validate()?;
        self.stop(Duration::from_secs(30)).await?;
        Self::delete_in(&InstancesService::directory()?, &self.id).await
    }

    async fn delete_in(root: &Path, id: &str) -> Result<()> {
        Uuid::parse_str(id).context("Invalid instance ID")?;
        let directory = root.join(id);
        let metadata = match fs::symlink_metadata(&directory).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error).context("Could not inspect instance directory"),
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("Instance path is not a regular directory");
        }
        fs::remove_dir_all(&directory)
            .await
            .context("Could not delete instance files")
    }

    /// Call only after the user accepts the Minecraft EULA.
    pub async fn accept_eula(&self) -> Result<()> {
        atomic_write(&self.directory()?.join("eula.txt"), b"eula=true\n").await
    }

    /// Java is an explicit executable path; java_version is metadata, not a path.
    /// SoftwareService must install the server jar or Forge launcher before starting.
    pub async fn start(&self, java: &Path) -> Result<()> {
        let mut running = self.running.lock().await;
        if running
            .as_ref()
            .is_some_and(|server| !*server.finished.borrow())
        {
            bail!("Server is already running");
        }

        let dir = self.directory()?;
        let eula = fs::read_to_string(dir.join("eula.txt"))
            .await
            .context("Could not read EULA acceptance")?;

        if !eula.lines().any(|line| line.trim() == "eula=true") {
            bail!("Accept the Minecraft EULA before starting the server");
        }

        let arguments = if self.software.eq_ignore_ascii_case("forge") {
            crate::software::forge::launch_arguments(&dir).await?
        } else {
            if !fs::metadata(dir.join("server.jar"))
                .await
                .context("Install server.jar before starting the server")?
                .is_file()
            {
                bail!("server.jar must be a regular file");
            }
            vec![
                std::ffi::OsString::from("-jar"),
                "server.jar".into(),
                "nogui".into(),
            ]
        };
        let java = fs::canonicalize(java)
            .await
            .context("Could not resolve Java executable")?;

        let mut child = tokio::process::Command::new(java)
            .current_dir(&dir)
            .arg(format!("-Xmx{}M", self.ram))
            .args(arguments)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Could not start Java server")?;

        let stdin = child.stdin.take().context("Server stdin is unavailable")?;
        let stdout = child
            .stdout
            .take()
            .context("Server stdout is unavailable")?;

        let stderr = child
            .stderr
            .take()
            .context("Server stderr is unavailable")?;

        let (commands, receiver) = mpsc::channel(32);
        let (done, finished) = watch::channel(false);

        *running = Some(RunningServer {
            pid: child.id(),
            commands,
            finished,
        });

        tokio::spawn(supervisor::run(
            child,
            stdin,
            stdout,
            stderr,
            receiver,
            done,
            self.console.clone(),
            self.events.clone(),
        ));

        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.events.subscribe()
    }

    pub fn console(&self) -> Arc<super::console::ConsoleSnapshot> {
        self.console.borrow().clone()
    }

    pub fn subscribe_console(&self) -> watch::Receiver<Arc<super::console::ConsoleSnapshot>> {
        self.console.subscribe()
    }

    pub async fn is_running(&self) -> bool {
        self.running
            .lock()
            .await
            .as_ref()
            .is_some_and(|server| !*server.finished.borrow())
    }

    pub async fn send_command(&self, command: String) -> Result<()> {
        if command.is_empty() || command.contains(['\r', '\n']) {
            bail!("Command must contain exactly one non-empty line");
        }
        let sender = self.sender().await?;
        let (reply, response) = oneshot::channel();
        sender
            .try_send(supervisor::Request::Command(command, reply))
            .context("Server command queue is full or closed")?;
        response.await.context("Server has exited")?
    }

    async fn sender(&self) -> Result<mpsc::Sender<supervisor::Request>> {
        let running = self.running.lock().await;
        let server = running
            .as_ref()
            .filter(|server| !*server.finished.borrow())
            .context("Server is not running")?;
        Ok(server.commands.clone())
    }

    pub async fn stop(&self, timeout: Duration) -> Result<()> {
        let (sender, mut finished) = {
            let running = self.running.lock().await;
            let Some(server) = running.as_ref() else {
                return Ok(());
            };
            if *server.finished.borrow() {
                return Ok(());
            }
            (server.commands.clone(), server.finished.clone())
        };
        let _ = sender.send(supervisor::Request::Stop(timeout)).await;
        finished
            .wait_for(|done| *done)
            .await
            .context("Server supervisor failed")?;
        Ok(())
    }
}

pub(super) async fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let result = async {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .await?;
        file.write_all(bytes).await?;
        file.sync_all().await?;
        drop(file);
        fs::rename(&temporary, path).await
    }
    .await;
    if result.is_err() {
        let _ = fs::remove_file(&temporary).await;
    }
    result.with_context(|| format!("Could not save {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_and_reload() -> Result<()> {
        let root = std::env::temp_dir().join(format!("localcraft-test-{}", Uuid::new_v4()));
        let instance = ServerInstance::create_in(
            &root,
            "Test".into(),
            "1.21".into(),
            "Paper".into(),
            "1024".into(),
            None,
        )
        .await?;
        let loaded = InstancesService::load_from(&root).await?;
        assert_eq!(loaded.servers().len(), 1);
        assert_eq!(
            loaded.servers().first().map(|item| &item.id),
            Some(&instance.id)
        );
        assert_eq!(
            fs::read_to_string(root.join(&instance.id).join("eula.txt")).await?,
            "eula=false\n"
        );
        assert!(!instance.is_running().await);
        fs::remove_dir_all(root).await?;
        Ok(())
    }

    #[tokio::test]
    async fn delete_removes_only_the_instance_directory() -> Result<()> {
        let root = std::env::temp_dir().join(format!("localcraft-delete-test-{}", Uuid::new_v4()));
        let instance = ServerInstance::create_in(
            &root,
            "Delete test".into(),
            "1.21".into(),
            "Paper".into(),
            "1024".into(),
            None,
        )
        .await?;
        let instance_directory = root.join(&instance.id);

        ServerInstance::delete_in(&root, &instance.id).await?;

        assert!(!fs::try_exists(instance_directory).await?);
        assert!(fs::try_exists(&root).await?);
        fs::remove_dir_all(root).await?;
        Ok(())
    }

    #[tokio::test]
    async fn rejects_invalid_input_without_creating_files() {
        let root = std::env::temp_dir().join(format!("localcraft-test-{}", Uuid::new_v4()));
        assert!(
            ServerInstance::create_in(
                &root,
                "Test".into(),
                "1.21".into(),
                "Paper".into(),
                "0".into(),
                None
            )
            .await
            .is_err()
        );
        assert!(!root.exists());
    }
}
