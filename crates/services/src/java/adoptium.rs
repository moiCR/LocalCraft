use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;

use super::{ArchiveFormat, Java, JavaDownload};

#[derive(Default)]
pub struct AdoptiumJava;

impl AdoptiumJava {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct Asset {
    binary: Binary,
}
#[derive(Deserialize)]
struct Binary {
    package: Package,
}
#[derive(Deserialize)]
struct Package {
    link: String,
    checksum: String,
    size: u64,
    name: String,
}

impl Java for AdoptiumJava {
    async fn get_archive(&self, client: &Client, version: u8) -> Result<JavaDownload> {
        super::validate_version(version)?;
        let (os, architecture) = platform(std::env::consts::OS, std::env::consts::ARCH)?;
        let assets: Vec<Asset> = client
            .get(format!(
                "https://api.adoptium.net/v3/assets/latest/{version}/hotspot"
            ))
            .query(&[
                ("os", os),
                ("architecture", architecture),
                ("image_type", "jre"),
                ("vendor", "eclipse"),
            ])
            .send()
            .await
            .context("Could not fetch Adoptium metadata")?
            .error_for_status()?
            .json()
            .await
            .context("Invalid Adoptium metadata")?;
        let package = assets
            .into_iter()
            .next()
            .context("No Adoptium JRE exists for this version and platform")?
            .binary
            .package;
        let format = if package.name.ends_with(".tar.gz") {
            ArchiveFormat::TarGz
        } else if package.name.ends_with(".zip") {
            ArchiveFormat::Zip
        } else {
            bail!("Unsupported Java archive format: {}", package.name);
        };
        Ok(JavaDownload {
            url: package.link,
            sha256: package.checksum,
            size: package.size,
            format,
        })
    }
}

fn platform(os: &str, arch: &str) -> Result<(&'static str, &'static str)> {
    let os = match os {
        "windows" => "windows",
        "macos" => "mac",
        "linux" => "linux",
        _ => bail!("Unsupported Java operating system: {os}"),
    };
    let arch = match arch {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        "x86" => "x86",
        "arm" => "arm",
        _ => bail!("Unsupported Java architecture: {arch}"),
    };
    Ok((os, arch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_mapping_rejects_unknown_targets() -> Result<()> {
        assert_eq!(platform("macos", "aarch64")?, ("mac", "aarch64"));
        assert_eq!(platform("linux", "x86_64")?, ("linux", "x64"));
        assert_eq!(platform("windows", "x86")?, ("windows", "x86"));
        assert!(platform("unknown", "x86_64").is_err());
        assert!(platform("linux", "unknown").is_err());
        Ok(())
    }
}
