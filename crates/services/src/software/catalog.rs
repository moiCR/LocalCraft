use super::{JarDownload, JarKind, SoftwareService, client, validate_version};
use anyhow::{Context, Result, bail};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct BuildOption {
    pub label: String,
    pub download: JarDownload,
}

async fn json(client: &reqwest::Client, url: &str) -> Result<Value> {
    client
        .get(url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("Invalid software catalog response")
}
fn text(value: &Value, field: &str) -> Result<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("Missing {field} in software metadata"))
}
fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
fn version_key(version: &str) -> Vec<u32> {
    version
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect()
}

impl SoftwareService {
    pub async fn versions(&self, software: &str) -> Result<Vec<String>> {
        let client = client()?;
        let mut versions = match software {
            "Paper" => {
                let data = json(&client, "https://fill.papermc.io/v3/projects/paper").await?;
                data.get("versions")
                    .and_then(Value::as_object)
                    .context("Missing Paper versions")?
                    .values()
                    .flat_map(|group| strings(Some(group)))
                    .collect()
            }
            "Purpur" => strings(
                json(&client, "https://api.purpurmc.org/v2/purpur")
                    .await?
                    .get("versions"),
            ),
            "Fabric" => json(&client, "https://meta.fabricmc.net/v2/versions/game")
                .await?
                .as_array()
                .context("Missing Fabric versions")?
                .iter()
                .filter(|entry| entry.get("stable").and_then(Value::as_bool) == Some(true))
                .filter_map(|entry| {
                    entry
                        .get("version")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect(),
            "Forge" => json(
                &client,
                "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json",
            )
            .await?
            .get("promos")
            .and_then(Value::as_object)
            .context("Missing Forge versions")?
            .keys()
            .filter_map(|key| key.rsplit_once('-').map(|(version, _)| version.to_owned()))
            .collect(),
            "Vanilla" => json(
                &client,
                "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
            )
            .await?
            .get("versions")
            .and_then(Value::as_array)
            .context("Missing Minecraft versions")?
            .iter()
            .filter(|entry| entry.get("type").and_then(Value::as_str) == Some("release"))
            .filter_map(|entry| entry.get("id").and_then(Value::as_str).map(str::to_owned))
            .collect(),
            "Pumpkin" => super::pumpkin::versions(&client).await?,
            _ => bail!("Unsupported software: {software}"),
        };
        versions.sort_by_key(|version| std::cmp::Reverse(version_key(version)));
        versions.dedup();
        if versions.is_empty() {
            bail!("No versions are available for {software}");
        }
        Ok(versions)
    }

    pub async fn builds(
        &self,
        software: &str,
        version: &str,
    ) -> Result<(Option<u8>, Vec<BuildOption>)> {
        validate_version(version)?;
        let client = client()?;
        if software == "Pumpkin" {
            let (label, download) = super::pumpkin::build(&client, version).await?;
            return Ok((None, vec![BuildOption { label, download }]));
        }
        let manifest = json(
            &client,
            "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
        )
        .await?;
        let release_url = manifest
            .get("versions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|entry| entry.get("id").and_then(Value::as_str) == Some(version))
            .and_then(|entry| entry.get("url"))
            .and_then(Value::as_str)
            .context("Minecraft version metadata is missing")?;
        let release = json(&client, release_url).await?;
        let java = release
            .get("javaVersion")
            .and_then(|v| v.get("majorVersion"))
            .and_then(Value::as_u64)
            .and_then(|v| u8::try_from(v).ok())
            .unwrap_or(8);
        let builds = match software {
            "Paper" => {
                let data = json(
                    &client,
                    &format!("https://fill.papermc.io/v3/projects/paper/versions/{version}/builds"),
                )
                .await?;
                let mut entries = data
                    .as_array()
                    .context("Missing Paper builds")?
                    .iter()
                    .filter(|build| build.get("channel").and_then(Value::as_str) == Some("STABLE"))
                    .collect::<Vec<_>>();
                entries.sort_by_key(|entry| {
                    std::cmp::Reverse(entry.get("id").and_then(Value::as_u64).unwrap_or(0))
                });
                entries
                    .into_iter()
                    .map(|entry| {
                        let download = entry
                            .get("downloads")
                            .and_then(|v| v.get("server:default"))
                            .context("Missing Paper download")?;
                        Ok(BuildOption {
                            label: entry.get("id").context("Missing build ID")?.to_string(),
                            download: JarDownload {
                                url: text(download, "url")?,
                                sha256: Some(text(
                                    download.get("checksums").context("Missing checksums")?,
                                    "sha256",
                                )?),
                                size: download.get("size").and_then(Value::as_u64),
                                kind: JarKind::Server,
                            },
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
            }
            "Purpur" => {
                let data = json(
                    &client,
                    &format!("https://api.purpurmc.org/v2/purpur/{version}"),
                )
                .await?;
                let build = text(
                    data.get("builds").context("Missing Purpur builds")?,
                    "latest",
                )?;
                vec![BuildOption {
                    label: build.clone(),
                    download: JarDownload {
                        url: format!(
                            "https://api.purpurmc.org/v2/purpur/{version}/{build}/download"
                        ),
                        sha256: None,
                        size: None,
                        kind: JarKind::Server,
                    },
                }]
            }
            "Fabric" => {
                let installers =
                    json(&client, "https://meta.fabricmc.net/v2/versions/installer").await?;
                let installer = installers
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|v| v.get("stable").and_then(Value::as_bool) == Some(true))
                    .context("No stable Fabric installer")?;
                let installer = text(installer, "version")?;
                let loaders = json(
                    &client,
                    &format!("https://meta.fabricmc.net/v2/versions/loader/{version}"),
                )
                .await?;
                let mut builds = loaders.as_array().context("Missing Fabric loaders")?.iter()
                    .filter_map(|entry| entry.get("loader"))
                    .filter(|entry| entry.get("stable").and_then(Value::as_bool) == Some(true))
                    .map(|loader| {
                        let loader = text(loader, "version")?;
                        validate_version(&loader)?;
                        validate_version(&installer)?;
                        Ok(BuildOption { label: loader.clone(), download: JarDownload {
                            url: format!("https://meta.fabricmc.net/v2/versions/loader/{version}/{loader}/{installer}/server/jar"),
                            sha256: None, size: None, kind: JarKind::Server,
                        }})
                    }).collect::<Result<Vec<_>>>()?;
                builds.sort_by_key(|build| std::cmp::Reverse(version_key(&build.label)));
                builds
            }
            "Forge" | "Vanilla" => {
                let download = self.get_jar(software, version).await?;
                let label = match &download.kind {
                    JarKind::ForgeInstaller { version } => version.clone(),
                    JarKind::NativeServer { filename } => filename.clone(),
                    JarKind::Server => "Official release".into(),
                };
                vec![BuildOption { label, download }]
            }
            _ => bail!("Unsupported software: {software}"),
        };
        if builds.is_empty() {
            bail!("No stable builds are available for this version");
        }
        Ok((Some(java), builds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_sort_numerically() {
        assert!(version_key("1.21.10") > version_key("1.21.9"));
        assert!(version_key("26.1") > version_key("1.21.11"));
    }
    #[tokio::test]
    #[ignore = "Requires the official provider APIs"]
    async fn live_provider_catalogs() -> Result<()> {
        for software in ["Paper", "Purpur", "Fabric", "Forge", "Vanilla", "Pumpkin"] {
            let service = SoftwareService::new();
            let versions = service.versions(software).await?;
            let version = versions
                .iter()
                .find(|v| v.as_str() == "1.21.4")
                .or_else(|| versions.first())
                .context("Empty catalog")?;
            let (java, builds) = service.builds(software, version).await?;
            assert!(java.is_none_or(|java| java >= 8));
            assert!(!builds.is_empty());
            assert!(
                builds
                    .iter()
                    .all(|build| build.download.url.starts_with("https://"))
            );
            let runtime = java.map_or_else(|| "native".to_owned(), |java| java.to_string());
            eprintln!(
                "{software}: {} versions, {} builds for {version}, runtime {runtime}",
                versions.len(),
                builds.len()
            );
        }
        Ok(())
    }
}
