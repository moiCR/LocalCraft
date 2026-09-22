use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use super::{JarDownload, JarKind, Software};

#[derive(Default)]
pub struct FabricSoftware;
impl FabricSoftware {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct Loader {
    loader: Version,
}
#[derive(Deserialize)]
struct Version {
    version: String,
    stable: bool,
}

impl Software for FabricSoftware {
    async fn get_jar(&self, client: &Client, version: &str) -> Result<JarDownload> {
        super::validate_version(version)?;
        let loaders: Vec<Loader> = client
            .get(format!(
                "https://meta.fabricmc.net/v2/versions/loader/{version}"
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Invalid Fabric loader metadata")?;
        let loader = loaders
            .into_iter()
            .find(|entry| entry.loader.stable)
            .context("No stable Fabric loader exists for this version")?
            .loader
            .version;
        let installers: Vec<Version> = client
            .get("https://meta.fabricmc.net/v2/versions/installer")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Invalid Fabric installer metadata")?;
        let installer = installers
            .into_iter()
            .find(|entry| entry.stable)
            .context("No stable Fabric installer is available")?
            .version;
        super::validate_version(&loader)?;
        super::validate_version(&installer)?;
        Ok(JarDownload {
            url: format!(
                "https://meta.fabricmc.net/v2/versions/loader/{version}/{loader}/{installer}/server/jar"
            ),
            sha256: None,
            size: None,
            kind: JarKind::Server,
        })
    }
}
