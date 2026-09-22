use std::{collections::HashMap, ffi::OsString, path::Path, process::Stdio, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;
use tokio::{fs, process::Command};

use super::{JarDownload, JarKind, Software};

#[derive(Default)]
pub struct ForgeSoftware;
impl ForgeSoftware {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct Promotions {
    promos: HashMap<String, String>,
}

impl Software for ForgeSoftware {
    async fn get_jar(&self, client: &Client, version: &str) -> Result<JarDownload> {
        super::validate_version(version)?;
        let promotions: Promotions = client
            .get("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Invalid Forge promotions")?;
        let build = promotions
            .promos
            .get(&format!("{version}-recommended"))
            .or_else(|| promotions.promos.get(&format!("{version}-latest")))
            .context("No Forge build exists for this Minecraft version")?;
        super::validate_version(build)?;
        let forge_version = format!("{version}-{build}");
        let url = format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{forge_version}/forge-{forge_version}-installer.jar"
        );
        let checksum = client.get(format!("{url}.sha256")).send().await?;
        let sha256 = if checksum.status() == reqwest::StatusCode::NOT_FOUND {
            None
        } else {
            Some(checksum.error_for_status()?.text().await?.trim().to_owned())
        };
        Ok(JarDownload {
            url,
            sha256,
            size: None,
            kind: JarKind::ForgeInstaller {
                version: forge_version,
            },
        })
    }
}

pub(super) async fn install(
    java: &Path,
    installer: &Path,
    directory: &Path,
    version: &str,
) -> Result<()> {
    let mut child = Command::new(java)
        .arg("-jar")
        .arg(installer)
        .arg("--installServer")
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("Could not run Forge installer")?;
    let status = match tokio::time::timeout(Duration::from_secs(600), child.wait()).await {
        Ok(status) => status?,
        Err(_) => {
            child.kill().await?;
            bail!("Forge installation timed out");
        }
    };
    if !status.success() {
        bail!("Forge installer failed with exit code {:?}", status.code());
    }
    arguments_for(directory, version).await?;
    let temporary = directory.join(format!("forge-version.{}.tmp", uuid::Uuid::new_v4()));
    let result = async {
        use tokio::io::AsyncWriteExt;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .await?;
        file.write_all(version.as_bytes()).await?;
        file.sync_all().await?;
        drop(file);
        fs::rename(&temporary, directory.join("forge-version.txt")).await
    }
    .await;
    if result.is_err() {
        let _ = fs::remove_file(&temporary).await;
    }
    result.context("Could not save installed Forge version")
}

pub(crate) async fn launch_arguments(directory: &Path) -> Result<Vec<OsString>> {
    let version = fs::read_to_string(directory.join("forge-version.txt"))
        .await
        .context("Install Forge before starting the server")?;
    arguments_for(directory, version.trim()).await
}

async fn arguments_for(directory: &Path, version: &str) -> Result<Vec<OsString>> {
    super::validate_version(version)?;
    let platform = if cfg!(windows) {
        "win_args.txt"
    } else {
        "unix_args.txt"
    };
    let arguments = format!("libraries/net/minecraftforge/forge/{version}/{platform}");
    if fs::try_exists(directory.join(&arguments)).await? {
        return Ok(vec![format!("@{arguments}").into(), "nogui".into()]);
    }
    for name in [
        format!("forge-{version}.jar"),
        format!("forge-{version}-universal.jar"),
    ] {
        if fs::try_exists(directory.join(&name)).await? {
            return Ok(vec!["-jar".into(), name.into(), "nogui".into()]);
        }
    }
    bail!("Forge installation did not produce a supported server launcher")
}
