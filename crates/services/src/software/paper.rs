use std::collections::HashMap;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use super::{JarDownload, JarKind, Software};

#[derive(Default)]
pub struct PaperSoftware;

impl PaperSoftware {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct Build {
    id: u64,
    channel: String,
    downloads: HashMap<String, Download>,
}

#[derive(Deserialize)]
struct Download {
    url: String,
    checksums: Checksums,
    size: u64,
}

#[derive(Deserialize)]
struct Checksums {
    sha256: String,
}

impl Software for PaperSoftware {
    async fn get_jar(&self, client: &Client, version: &str) -> Result<JarDownload> {
        super::validate_version(version)?;
        let builds: Vec<Build> = client
            .get(format!(
                "https://fill.papermc.io/v3/projects/paper/versions/{version}/builds"
            ))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Invalid Paper build metadata")?;
        let build = builds
            .into_iter()
            .filter(|build| build.channel == "STABLE")
            .max_by_key(|build| build.id)
            .context("No stable Paper build exists for this version")?;
        let download = build
            .downloads
            .into_iter()
            .find(|(key, _)| key == "server:default")
            .map(|(_, value)| value)
            .context("Paper server download is missing")?;
        Ok(JarDownload {
            url: download.url,
            sha256: Some(download.checksums.sha256),
            size: Some(download.size),
            kind: JarKind::Server,
        })
    }
}
