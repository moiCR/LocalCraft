use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::{Mutex, mpsc, watch},
};

const RELEASES_URL: &str = "https://api.github.com/repos/playit-cloud/playit-agent/releases/latest";
const API_URL: &str = "https://api.playit.gg";
const LOG_LIMIT: usize = 1_000;

#[derive(Debug, Clone)]
pub struct PlayitLog {
    pub id: u64,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct PlayitSnapshot {
    pub installed_path: Option<PathBuf>,
    pub version: Option<String>,
    pub running: bool,
    pub claim_code: Option<String>,
    pub claim_url: Option<String>,
    pub connected: bool,
    pub logs: Vec<PlayitLog>,
    pub error: Option<String>,
    log_sequence: u64,
}

pub struct PlayitService {
    process: Arc<Mutex<Option<Child>>>,
    snapshot: watch::Sender<Arc<PlayitSnapshot>>,
    operation: Mutex<()>,
}

impl Default for PlayitService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(serde::Serialize)]
struct ClaimSetupRequest<'a> {
    code: &'a str,
    agent_type: &'static str,
    version: String,
}

#[derive(serde::Serialize)]
struct ClaimExchangeRequest<'a> {
    code: &'a str,
}

#[derive(Deserialize)]
struct SecretResponse {
    secret_key: String,
}

#[derive(Deserialize)]
#[serde(tag = "status", content = "data")]
enum ClaimSetupResponse {
    #[serde(rename = "success")]
    Success(ClaimSetupStatus),
    #[serde(rename = "fail")]
    Fail(serde_json::Value),
    #[serde(rename = "error")]
    Error(serde_json::Value),
}

#[derive(Deserialize)]
enum ClaimSetupStatus {
    WaitingForUserVisit,
    WaitingForUser,
    UserAccepted,
    UserRejected,
}

#[derive(Deserialize)]
#[serde(tag = "status", content = "data")]
enum ClaimExchangeResponse {
    #[serde(rename = "success")]
    Success(SecretResponse),
    #[serde(rename = "fail")]
    Fail(serde_json::Value),
    #[serde(rename = "error")]
    Error(serde_json::Value),
}

impl PlayitService {
    pub fn new() -> Self {
        let (snapshot, _) = watch::channel(Arc::new(PlayitSnapshot::default()));
        Self {
            process: Arc::new(Mutex::new(None)),
            snapshot,
            operation: Mutex::new(()),
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<Arc<PlayitSnapshot>> {
        self.snapshot.subscribe()
    }

    pub fn snapshot(&self) -> Arc<PlayitSnapshot> {
        self.snapshot.borrow().clone()
    }

    pub fn directory() -> Result<PathBuf> {
        Ok(dirs::data_dir()
            .context("Could not find data directory")?
            .join("LocalCraft")
            .join("playit"))
    }

    pub async fn install(&self) -> Result<PathBuf> {
        let _operation = self.operation.lock().await;
        if self.process.lock().await.is_some() {
            bail!("Stop the Playit agent before updating it");
        }

        let client = client()?;
        let release = client
            .get(RELEASES_URL)
            .send()
            .await
            .context("Could not check Playit releases")?
            .error_for_status()
            .context("Playit releases request failed")?
            .json::<Release>()
            .await
            .context("Could not read Playit release metadata")?;
        if release.prerelease {
            bail!("GitHub returned a Playit pre-release instead of a stable release");
        }

        let (asset_name, extension) = platform_asset()?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .with_context(|| format!("Playit release has no {asset_name} asset"))?;
        let expected_hash = asset
            .digest
            .as_deref()
            .and_then(|value| value.strip_prefix("sha256:"))
            .context("Playit release does not provide a SHA-256 digest")?;
        if expected_hash.len() != 64 || !expected_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("Playit release contains an invalid SHA-256 digest");
        }

        let directory = Self::directory()?;
        fs::create_dir_all(&directory)
            .await
            .context("Could not create Playit installation directory")?;
        let version = release.tag_name.trim_start_matches('v');
        let mut destination =
            directory.join(format!("playit-{version}-{expected_hash}{extension}"));
        if fs::try_exists(&destination).await? {
            let existing_hash = hash_file(&destination).await?;
            if existing_hash.eq_ignore_ascii_case(expected_hash) {
                self.update_snapshot(|snapshot| {
                    snapshot.installed_path = Some(destination.clone());
                    snapshot.version = Some(version.to_owned());
                    snapshot.error = None;
                });
                return Ok(destination);
            }
            destination = directory.join(format!(
                "playit-{version}-{expected_hash}-{}{extension}",
                uuid::Uuid::new_v4()
            ));
        }

        let staging = directory.join(format!(".playit-{}.download", uuid::Uuid::new_v4()));
        let result = async {
            let mut response = client
                .get(&asset.browser_download_url)
                .send()
                .await
                .context("Could not download Playit agent")?
                .error_for_status()
                .context("Playit download failed")?;
            let mut file = fs::File::create(&staging)
                .await
                .context("Could not create Playit staging file")?;
            let mut hasher = Sha256::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .context("Could not read Playit download")?
            {
                hasher.update(&chunk);
                tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                    .await
                    .context("Could not write Playit staging file")?;
            }
            tokio::io::AsyncWriteExt::flush(&mut file).await?;
            drop(file);
            let actual_hash = format!("{:x}", hasher.finalize());
            if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                bail!("Playit agent SHA-256 verification failed");
            }
            set_executable(&staging).await?;
            fs::rename(&staging, &destination)
                .await
                .context("Could not publish Playit installation")?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if result.is_err() {
            let _ = fs::remove_file(&staging).await;
        }
        result?;

        self.update_snapshot(|snapshot| {
            snapshot.installed_path = Some(destination.clone());
            snapshot.version = Some(version.to_owned());
            snapshot.error = None;
        });
        Ok(destination)
    }

    pub async fn start(&self) -> Result<()> {
        let _operation = self.operation.lock().await;
        let mut process = self.process.lock().await;
        if let Some(child) = process.as_mut() {
            if child.try_wait()?.is_none() {
                return Ok(());
            }
            *process = None;
        }

        let snapshot = self.snapshot();
        let executable = snapshot
            .installed_path
            .clone()
            .or(discover_installation().await?)
            .context("Install the Playit agent before starting it")?;
        let secret_path = Self::secret_path()?;
        if !fs::try_exists(&secret_path).await? {
            bail!("Connect your Playit account before starting the agent");
        }
        let mut command = Command::new(&executable);
        command
            .arg("--secret-path")
            .arg(&secret_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.as_std_mut().creation_flags(0x0800_0000);
        }
        let mut child = command.spawn().context("Could not start Playit agent")?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        *process = Some(child);
        self.update_snapshot(|snapshot| {
            snapshot.installed_path = Some(executable);
            snapshot.running = true;
            snapshot.error = None;
            snapshot.logs.clear();
        });

        let (log_sender, log_receiver) = mpsc::channel(512);
        if let Some(stdout) = stdout {
            self.spawn_log_reader(BufReader::new(stdout), log_sender.clone());
        }
        if let Some(stderr) = stderr {
            self.spawn_log_reader(BufReader::new(stderr), log_sender);
        }
        self.spawn_log_batcher(log_receiver);
        self.monitor_process();
        Ok(())
    }

    pub async fn connect(&self) -> Result<()> {
        let _operation = self.operation.lock().await;
        self.snapshot()
            .installed_path
            .clone()
            .or(discover_installation().await?)
            .context("Install the Playit agent before connecting")?;
        if fs::try_exists(Self::secret_path()?).await? {
            bail!("A Playit account is already connected");
        }
        let mut process = self.process.lock().await;
        if let Some(mut child) = process.take() {
            child
                .kill()
                .await
                .context("Could not restart Playit agent")?;
            let _ = child.wait().await;
        }
        self.update_snapshot(|snapshot| snapshot.running = false);
        drop(process);

        let code = uuid::Uuid::new_v4().simple().to_string();
        let code = code
            .get(..10)
            .context("Could not generate Playit claim code")?
            .to_owned();
        let claim_url = format!("https://playit.gg/claim/{code}");
        self.update_snapshot(|snapshot| {
            snapshot.claim_code = Some(code.clone());
            snapshot.claim_url = Some(claim_url.clone());
            snapshot.connected = false;
            snapshot.error = None;
        });
        self.push_log(format!("Claim code: {code}"));
        self.push_log(format!("Approve this agent: {claim_url}"));

        let client = client()?;
        let version = self
            .snapshot()
            .version
            .clone()
            .unwrap_or_else(|| "1.0.10".to_owned());
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(600);
        loop {
            let response = client
                .post(format!("{API_URL}/claim/setup"))
                .json(&ClaimSetupRequest {
                    code: &code,
                    agent_type: "assignable",
                    version: format!("playit {version}"),
                })
                .send()
                .await
                .context("Could not check Playit claim status")?
                .error_for_status()
                .context("Playit claim status request failed")?
                .json::<ClaimSetupResponse>()
                .await
                .context("Could not parse Playit claim status")?;
            match response {
                ClaimSetupResponse::Success(ClaimSetupStatus::UserAccepted) => break,
                ClaimSetupResponse::Success(ClaimSetupStatus::UserRejected) => {
                    bail!("Playit account rejected the agent claim")
                }
                ClaimSetupResponse::Success(
                    ClaimSetupStatus::WaitingForUserVisit | ClaimSetupStatus::WaitingForUser,
                ) => {}
                ClaimSetupResponse::Fail(error) | ClaimSetupResponse::Error(error) => {
                    bail!("Playit claim setup failed: {error}")
                }
            }
            if tokio::time::Instant::now() >= deadline {
                bail!("Playit claim expired; start account connection again")
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        self.push_log("Playit claim approved. Finishing account connection.".to_owned());
        let secret = loop {
            let response = client
                .post(format!("{API_URL}/claim/exchange"))
                .json(&ClaimExchangeRequest { code: &code })
                .send()
                .await
                .context("Could not exchange Playit claim")?
                .error_for_status()
                .context("Playit claim exchange failed")?
                .json::<ClaimExchangeResponse>()
                .await
                .context("Could not parse Playit claim response")?;
            match response {
                ClaimExchangeResponse::Success(secret) => break secret.secret_key,
                ClaimExchangeResponse::Fail(error) => {
                    if tokio::time::Instant::now() >= deadline {
                        bail!("Playit claim expired: {error}")
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                ClaimExchangeResponse::Error(error) => {
                    bail!("Playit claim exchange failed: {error}")
                }
            }
        };

        let secret_path = Self::secret_path()?;
        let parent = secret_path.parent().context("Invalid Playit secret path")?;
        fs::create_dir_all(parent)
            .await
            .context("Could not create Playit secret directory")?;
        let staging = parent.join(format!(".playit-{}.tmp", uuid::Uuid::new_v4()));
        let write_result = async {
            fs::write(&staging, format!("secret_key = \"{secret}\"\n"))
                .await
                .context("Could not write Playit secret")?;
            fs::rename(&staging, &secret_path)
                .await
                .context("Could not install Playit secret")?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if write_result.is_err() {
            let _ = fs::remove_file(&staging).await;
        }
        write_result?;
        self.update_snapshot(|snapshot| {
            snapshot.claim_code = None;
            snapshot.claim_url = None;
            snapshot.connected = true;
        });
        self.push_log("Playit account connected. Starting agent.".to_owned());
        drop(_operation);
        self.start().await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let _operation = self.operation.lock().await;
        let mut process = self.process.lock().await;
        if let Some(mut child) = process.take() {
            child.kill().await.context("Could not stop Playit agent")?;
            let _ = child.wait().await;
        }
        self.update_snapshot(|snapshot| snapshot.running = false);
        Ok(())
    }

    pub async fn refresh(&self) -> Result<()> {
        let mut process = self.process.lock().await;
        if let Some(child) = process.as_mut()
            && child.try_wait()?.is_some()
        {
            *process = None;
            self.update_snapshot(|snapshot| snapshot.running = false);
        }
        if self.snapshot().installed_path.is_none()
            && let Some(path) = discover_installation().await?
        {
            let version = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| value.strip_prefix("playit-"))
                .and_then(|value| value.split('-').next())
                .map(str::to_owned);
            self.update_snapshot(|snapshot| {
                snapshot.installed_path = Some(path);
                snapshot.version = version;
            });
        }
        let connected = fs::try_exists(Self::secret_path()?).await?;
        self.update_snapshot(|snapshot| snapshot.connected = connected);
        Ok(())
    }

    fn spawn_log_reader<R>(&self, reader: BufReader<R>, sender: mpsc::Sender<String>)
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let snapshot = self.snapshot.clone();
        tokio::spawn(async move {
            let mut lines = reader.lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if sender.send(line).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        let message = format!("Could not read Playit output: {error}");
                        snapshot
                            .send_modify(|current| Arc::make_mut(current).error = Some(message));
                        break;
                    }
                }
            }
        });
    }

    fn spawn_log_batcher(&self, mut receiver: mpsc::Receiver<String>) {
        let snapshot = self.snapshot.clone();
        tokio::spawn(async move {
            let mut batch = Vec::new();
            loop {
                tokio::select! {
                    line = receiver.recv() => match line {
                        Some(line) => batch.push(line),
                        None => break,
                    },
                    _ = tokio::time::sleep(std::time::Duration::from_millis(80)) => {
                        flush_logs(&snapshot, &mut batch);
                    }
                }
            }
            flush_logs(&snapshot, &mut batch);
        });
    }

    fn monitor_process(&self) {
        let process = Arc::downgrade(&self.process);
        let snapshot = self.snapshot.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let Some(process) = process.upgrade() else {
                    break;
                };
                let mut process = process.lock().await;
                let Some(child) = process.as_mut() else {
                    break;
                };
                match child.try_wait() {
                    Ok(Some(_)) => {
                        *process = None;
                        snapshot.send_modify(|current| Arc::make_mut(current).running = false);
                        break;
                    }
                    Ok(None) => drop(process),
                    Err(error) => {
                        snapshot.send_modify(|current| {
                            Arc::make_mut(current).error =
                                Some(format!("Could not check Playit process status: {error}"));
                        });
                        break;
                    }
                }
            }
        });
    }

    fn update_snapshot(&self, update: impl FnOnce(&mut PlayitSnapshot)) {
        self.snapshot
            .send_modify(|snapshot| update(Arc::make_mut(snapshot)));
    }

    fn push_log(&self, message: String) {
        flush_logs(&self.snapshot, &mut vec![message]);
    }

    fn secret_path() -> Result<PathBuf> {
        Ok(Self::directory()?.join("playit.toml"))
    }
}

fn flush_logs(snapshot: &watch::Sender<Arc<PlayitSnapshot>>, batch: &mut Vec<String>) {
    if batch.is_empty() {
        return;
    }
    snapshot.send_modify(|current| {
        let current = Arc::make_mut(current);
        current.logs.extend(batch.drain(..).map(|message| {
            let id = current.log_sequence;
            current.log_sequence = current.log_sequence.wrapping_add(1);
            PlayitLog { id, message }
        }));
        if current.logs.len() > LOG_LIMIT {
            let excess = current.logs.len() - LOG_LIMIT;
            current.logs.drain(..excess);
        }
    });
}

async fn discover_installation() -> Result<Option<PathBuf>> {
    let directory = PlayitService::directory()?;
    if !fs::try_exists(&directory).await? {
        return Ok(None);
    }
    let extension = if cfg!(windows) { "exe" } else { "" };
    let mut entries = fs::read_dir(directory).await?;
    let mut candidates = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let matches = path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| {
                name.starts_with("playit-")
                    && path
                        .extension()
                        .and_then(|value| value.to_str())
                        .unwrap_or("")
                        == extension
            });
        if matches {
            candidates.push(path);
        }
    }
    candidates.sort();
    Ok(candidates.pop())
}

fn platform_asset() -> Result<(&'static str, &'static str)> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        Ok(("playit-windows-x86_64.exe", ".exe"))
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Ok(("playit-linux-amd64", ""))
    }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        Ok(("playit-linux-aarch64", ""))
    }

    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64")
    )))]
    {
        bail!("Automatic Playit installation is not supported on this platform")
    }
}

async fn hash_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .await
        .with_context(|| format!("Could not read {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

async fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).await?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).await?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn client() -> Result<Client> {
    Client::builder()
        .https_only(true)
        .user_agent(concat!("LocalCraft/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("Could not initialize Playit HTTP client")
}

#[cfg(test)]
mod tests {
    use super::{ClaimExchangeResponse, ClaimSetupResponse, ClaimSetupStatus};

    #[test]
    fn parses_playit_claim_status_envelope() {
        let response = serde_json::from_str::<ClaimSetupResponse>(
            r#"{"status":"success","data":"UserAccepted"}"#,
        );
        assert!(matches!(
            response,
            Ok(ClaimSetupResponse::Success(ClaimSetupStatus::UserAccepted))
        ));
    }

    #[test]
    fn parses_playit_secret_envelope() {
        let response = serde_json::from_str::<ClaimExchangeResponse>(
            r#"{"status":"success","data":{"secret_key":"aabbcc"}}"#,
        );
        assert!(matches!(
            response,
            Ok(ClaimExchangeResponse::Success(secret)) if secret.secret_key == "aabbcc"
        ));
    }
}
