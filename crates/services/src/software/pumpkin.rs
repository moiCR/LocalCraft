use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;

use super::{JarDownload, JarKind, validate_version};

const RELEASE_URL: &str = "https://api.github.com/repos/Pumpkin-MC/Pumpkin/releases/latest";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

pub fn is_pumpkin(software: &str) -> bool {
    software.eq_ignore_ascii_case("pumpkin")
}

pub fn executable_name() -> &'static str {
    if cfg!(windows) {
        "pumpkin-server.exe"
    } else {
        "pumpkin-server"
    }
}

pub async fn versions(client: &Client) -> Result<Vec<String>> {
    let release = latest_release(client).await?;
    Ok(vec![minecraft_version(&release.tag_name)?.to_owned()])
}

pub async fn build(client: &Client, version: &str) -> Result<(String, JarDownload)> {
    let release = latest_release(client).await?;
    build_from_release(
        release,
        version,
        std::env::consts::OS,
        std::env::consts::ARCH,
        target_env(),
    )
}

fn build_from_release(
    release: Release,
    version: &str,
    os: &str,
    arch: &str,
    environment: &str,
) -> Result<(String, JarDownload)> {
    let supported_version = minecraft_version(&release.tag_name)?.to_owned();
    if version != supported_version.as_str() {
        bail!("Pumpkin only supports Minecraft {supported_version}");
    }

    let (asset_name, filename) = target_asset(os, arch, environment)?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .with_context(|| format!("Pumpkin release has no asset for this system: {asset_name}"))?;
    let digest = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .context("Pumpkin release asset has no SHA256 digest")?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("Pumpkin release asset has an invalid SHA256 digest");
    }
    if asset.size == 0 {
        bail!("Pumpkin release asset has an invalid size");
    }
    if !asset.browser_download_url.starts_with("https://") {
        bail!("Pumpkin release asset has an invalid download URL");
    }

    Ok((
        release.tag_name,
        JarDownload {
            url: asset.browser_download_url.clone(),
            sha256: Some(digest.to_owned()),
            size: Some(asset.size),
            kind: JarKind::NativeServer {
                filename: filename.to_owned(),
            },
        },
    ))
}

async fn latest_release(client: &Client) -> Result<Release> {
    client
        .get(RELEASE_URL)
        .header("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("Invalid Pumpkin release metadata")
}

fn minecraft_version(tag: &str) -> Result<&str> {
    let (_, release_metadata) = tag
        .split_once('+')
        .context("Pumpkin release tag has no Minecraft version")?;
    let (version, _) = release_metadata
        .split_once('-')
        .context("Pumpkin release tag has an invalid Minecraft version")?;
    validate_version(version)?;
    Ok(version)
}

fn target_env() -> &'static str {
    if cfg!(target_env = "musl") {
        "musl"
    } else {
        "gnu"
    }
}

fn target_asset(os: &str, arch: &str, environment: &str) -> Result<(&'static str, &'static str)> {
    let (asset, filename) = match (os, arch) {
        ("windows", "x86_64") => ("pumpkin-X64-Windows.exe", "pumpkin-server.exe"),
        ("windows", "aarch64") => ("pumpkin-ARM64-Windows.exe", "pumpkin-server.exe"),
        ("linux", "x86_64") if environment == "musl" => {
            ("pumpkin-X64-Linux-musl", "pumpkin-server")
        }
        ("linux", "aarch64") if environment == "musl" => {
            ("pumpkin-ARM64-Linux-musl", "pumpkin-server")
        }
        ("linux", "x86_64") => ("pumpkin-X64-Linux", "pumpkin-server"),
        ("linux", "aarch64") => ("pumpkin-ARM64-Linux", "pumpkin-server"),
        ("macos", "aarch64") => ("pumpkin-ARM64-macOS", "pumpkin-server"),
        _ => bail!("Pumpkin has no release asset for {os}/{arch}"),
    };
    Ok((asset, filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minecraft_version_from_release_tag() -> Result<()> {
        assert_eq!(minecraft_version("0.2.0+26.3-26.51")?, "26.3");
        assert!(minecraft_version("nightly").is_err());
        Ok(())
    }

    #[test]
    fn selects_published_assets_for_supported_targets() -> Result<()> {
        assert_eq!(
            target_asset("windows", "x86_64", "gnu")?,
            ("pumpkin-X64-Windows.exe", "pumpkin-server.exe")
        );
        assert_eq!(
            target_asset("linux", "aarch64", "musl")?,
            ("pumpkin-ARM64-Linux-musl", "pumpkin-server")
        );
        assert_eq!(
            target_asset("macos", "aarch64", "gnu")?,
            ("pumpkin-ARM64-macOS", "pumpkin-server")
        );
        assert!(target_asset("macos", "x86_64", "gnu").is_err());
        assert!(target_asset("freebsd", "x86_64", "gnu").is_err());
        Ok(())
    }

    #[test]
    fn recognizes_pumpkin_case_insensitively() {
        assert!(is_pumpkin("Pumpkin"));
        assert!(is_pumpkin("pumpkin"));
        assert!(!is_pumpkin("Paper"));
    }

    #[test]
    fn release_fixture_resolves_digest_size_and_target_asset() -> Result<()> {
        let release: Release = serde_json::from_value(serde_json::json!({
            "tag_name": "0.2.0+26.3-26.51",
            "assets": [{
                "name": "pumpkin-X64-Windows.exe",
                "browser_download_url": "https://github.com/Pumpkin-MC/Pumpkin/releases/download/stable/pumpkin-X64-Windows.exe",
                "size": 1234,
                "digest": format!("sha256:{}", "a".repeat(64))
            }]
        }))?;
        let (label, download) = build_from_release(release, "26.3", "windows", "x86_64", "gnu")?;
        let expected_digest = "a".repeat(64);
        assert_eq!(label, "0.2.0+26.3-26.51");
        assert_eq!(download.sha256.as_deref(), Some(expected_digest.as_str()));
        assert_eq!(download.size, Some(1234));
        assert!(matches!(download.kind, JarKind::NativeServer { .. }));
        Ok(())
    }

    #[test]
    fn release_fixture_rejects_missing_assets_and_versions() -> Result<()> {
        let release: Release = serde_json::from_value(serde_json::json!({
            "tag_name": "0.2.0+26.3-26.51",
            "assets": []
        }))?;
        assert!(build_from_release(release, "26.3", "windows", "x86_64", "gnu").is_err());

        let release: Release = serde_json::from_value(serde_json::json!({
            "tag_name": "0.2.0+26.3-26.51",
            "assets": []
        }))?;
        assert!(build_from_release(release, "26.2", "windows", "x86_64", "gnu").is_err());

        let release: Release = serde_json::from_value(serde_json::json!({
            "tag_name": "0.2.0+26.3-26.51",
            "assets": [{
                "name": "pumpkin-X64-Windows.exe",
                "browser_download_url": "https://github.com/Pumpkin-MC/Pumpkin/releases/download/stable/pumpkin-X64-Windows.exe",
                "size": 1234
            }]
        }))?;
        assert!(build_from_release(release, "26.3", "windows", "x86_64", "gnu").is_err());
        Ok(())
    }
}
