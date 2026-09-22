use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use super::{JarDownload, JarKind, Software};

#[derive(Default)]
pub struct VanillaSoftware;

impl VanillaSoftware {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct Manifest {
    versions: Vec<Version>,
}
#[derive(Deserialize)]
struct Version {
    id: String,
    url: String,
}
#[derive(Deserialize)]
struct Release {
    downloads: Downloads,
}
#[derive(Deserialize)]
struct Downloads {
    server: Option<Download>,
}
#[derive(Deserialize)]
struct Download {
    url: String,
    size: u64,
}

impl Software for VanillaSoftware {
    async fn get_jar(&self, client: &Client, version: &str) -> Result<JarDownload> {
        super::validate_version(version)?;
        let manifest: Manifest = client
            .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Invalid Minecraft version manifest")?;
        let version = manifest
            .versions
            .into_iter()
            .find(|entry| entry.id == version)
            .context("Minecraft version was not found")?;
        let release: Release = client
            .get(version.url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Invalid Minecraft release metadata")?;
        let server = release
            .downloads
            .server
            .context("This Minecraft version has no server jar")?;
        // Mojang publishes SHA1; it cannot substitute for the required trusted SHA256.
        Ok(JarDownload {
            url: server.url,
            sha256: None,
            size: Some(server.size),
            kind: JarKind::Server,
        })
    }
}
