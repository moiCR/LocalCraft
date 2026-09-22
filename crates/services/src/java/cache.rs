use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};

use super::{extract, java_installation::JavaInstallation};

#[derive(Serialize, Deserialize)]
struct Manifest {
    major_version: u8,
    binary_path: PathBuf,
    archive_sha256: String,
    os: String,
    architecture: String,
}

pub(super) async fn read(root: &Path, version: u8) -> Result<Option<JavaInstallation>> {
    let directory = root.join(version.to_string());
    if !fs::try_exists(&directory).await? {
        return Ok(None);
    }
    let manifest: Manifest = serde_json::from_slice(
        &fs::read(directory.join("installation.json"))
            .await
            .context("Java cache is incomplete; remove this version before reinstalling")?,
    )?;
    if manifest.major_version != version
        || manifest.os != std::env::consts::OS
        || manifest.architecture != std::env::consts::ARCH
    {
        bail!("Cached Java version or platform does not match");
    }
    if manifest.archive_sha256.len() != 64
        || !manifest
            .archive_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("Cached Java archive checksum is invalid");
    }
    extract::validate_path(&manifest.binary_path)?;
    let root = fs::canonicalize(&directory).await?;
    let binary = fs::canonicalize(directory.join(manifest.binary_path))
        .await
        .context("Cached Java executable is missing")?;
    if !binary.starts_with(&root) || !fs::metadata(&binary).await?.is_file() {
        bail!("Invalid cached Java executable");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&binary).await?.permissions().mode() & 0o111 == 0 {
            bail!("Cached Java binary is not executable");
        }
    }
    Ok(Some(JavaInstallation::new(
        version,
        "Adoptium".into(),
        binary,
    )))
}

pub(super) async fn load(root: &Path) -> Result<HashMap<String, JavaInstallation>> {
    let mut entries = match fs::read_dir(root).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => return Err(error.into()),
    };
    let mut installations = HashMap::new();
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_dir() {
            continue;
        }
        let Some(version) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u8>().ok())
        else {
            continue;
        };
        super::validate_version(version)?;
        if let Some(installation) = read(root, version).await? {
            installations.insert(version.to_string(), installation);
        }
    }
    Ok(installations)
}

pub(super) async fn write(
    directory: &Path,
    version: u8,
    binary_path: &Path,
    sha256: &str,
) -> Result<()> {
    let manifest = Manifest {
        major_version: version,
        binary_path: binary_path.to_path_buf(),
        archive_sha256: sha256.into(),
        os: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
    };
    atomic_write(
        &directory.join("installation.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )
    .await
}

pub(super) async fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = async {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
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
    result.with_context(|| format!("Could not write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cache_survives_reload_and_rejects_missing_binary() -> Result<()> {
        let root =
            std::env::temp_dir().join(format!("localcraft-java-cache-{}", uuid::Uuid::new_v4()));
        let relative = Path::new(if cfg!(windows) {
            "jdk/bin/java.exe"
        } else {
            "jdk/bin/java"
        });
        let runtime = root.join("21");
        let binary = runtime.join(relative);
        fs::create_dir_all(binary.parent().context("Missing binary parent")?).await?;
        fs::write(&binary, b"test executable").await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).await?;
        }
        write(&runtime, 21, relative, &"a".repeat(64)).await?;
        let installed = load(&root).await?;
        assert_eq!(installed.len(), 1);
        assert_eq!(installed.get("21").map(|java| java.major_version), Some(21));
        assert!(read(&root, 17).await?.is_none());
        fs::remove_file(&binary).await?;
        assert!(read(&root, 21).await.is_err());
        fs::remove_dir_all(root).await?;
        Ok(())
    }

    #[tokio::test]
    async fn incomplete_cache_is_not_reused() -> Result<()> {
        let root =
            std::env::temp_dir().join(format!("localcraft-java-cache-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("21")).await?;
        assert!(read(&root, 21).await.is_err());
        fs::remove_dir_all(root).await?;
        Ok(())
    }
}
