use std::{
    fs, io,
    io::Read,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;

use super::ArchiveFormat;

const MAX_EXTRACTED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;

pub(super) fn extract(
    archive: &Path,
    destination: &Path,
    format: ArchiveFormat,
) -> Result<PathBuf> {
    match format {
        ArchiveFormat::Zip => extract_zip(archive, destination)?,
        ArchiveFormat::TarGz => extract_tar(archive, destination)?,
    }
    find_java(destination)
}

fn extract_zip(path: &Path, destination: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path)?)?;
    if archive.len() > MAX_ENTRIES {
        bail!("Java archive contains too many entries");
    }
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let relative = entry.enclosed_name().context("Unsafe Java archive path")?;
        validate_path(&relative)?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("Java ZIP symlinks are not supported");
        }
        let output = destination.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        total = total
            .checked_add(entry.size())
            .context("Java archive size overflow")?;
        if total > MAX_EXTRACTED_BYTES {
            bail!("Java archive exceeds extraction limit");
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&output)?;
        let size = entry.size();
        let copied = io::copy(&mut (&mut entry).take(size + 1), &mut file)?;
        if copied != entry.size() {
            bail!("Incomplete Java ZIP entry");
        }
        restore_permissions(&output, entry.unix_mode())?;
    }
    Ok(())
}

fn extract_tar(path: &Path, destination: &Path) -> Result<()> {
    let mut archive = tar::Archive::new(GzDecoder::new(fs::File::open(path)?));
    let mut total = 0_u64;
    for (index, entry) in archive.entries()?.enumerate() {
        if index >= MAX_ENTRIES {
            bail!("Java archive contains too many entries");
        }
        let mut entry = entry?;
        let relative = entry.path()?.into_owned();
        validate_path(&relative)?;
        total = total
            .checked_add(entry.size())
            .context("Java archive size overflow")?;
        if total > MAX_EXTRACTED_BYTES {
            bail!("Java archive exceeds extraction limit");
        }
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            let target = entry.link_name()?.context("Archive link has no target")?;
            let base = if kind.is_symlink() {
                relative.parent().unwrap_or(Path::new(""))
            } else {
                Path::new("")
            };
            validate_link(base, &target)?;
        } else if !kind.is_file() && !kind.is_dir() {
            bail!("Unsupported entry in Java archive");
        }
        if !entry.unpack_in(destination)? {
            bail!("Java archive entry escapes its destination");
        }
        if kind.is_file() {
            restore_permissions(&destination.join(relative), entry.header().mode().ok())?;
        }
    }
    Ok(())
}

pub(super) fn validate_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        bail!("Unsafe path in Java installation");
    }
    Ok(())
}

fn validate_link(base: &Path, target: &Path) -> Result<()> {
    let mut depth = base
        .components()
        .filter(|part| matches!(part, Component::Normal(_)))
        .count();
    for component in target.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir if depth > 0 => depth -= 1,
            _ => bail!("Java archive link escapes its destination"),
        }
    }
    Ok(())
}

fn restore_permissions(path: &Path, mode: Option<u32>) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let executable = path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "bin")
            || mode.is_some_and(|mode| mode & 0o111 != 0);
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if executable { 0o755 } else { 0o644 }),
        )?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}

fn find_java(root: &Path) -> Result<PathBuf> {
    let name = if cfg!(windows) { "java.exe" } else { "java" };
    let mut directories = vec![root.to_path_buf()];
    let mut executable = None;
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                directories.push(entry.path());
            } else if kind.is_file()
                && entry.file_name() == name
                && entry
                    .path()
                    .parent()
                    .and_then(Path::file_name)
                    .is_some_and(|name| name == "bin")
            {
                if executable.is_some() {
                    bail!("Java archive contains multiple runtime executables");
                }
                executable = Some(entry.path().strip_prefix(root)?.to_path_buf());
            }
        }
    }
    executable.context("Java executable was not found in the archive")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_escaping_archive_paths_and_links() {
        assert!(validate_path(Path::new("../java")).is_err());
        assert!(validate_path(Path::new("/tmp/java")).is_err());
        assert!(validate_link(Path::new("jdk/lib"), Path::new("../../../outside")).is_err());
        assert!(validate_link(Path::new("jdk/lib"), Path::new("../bin/java")).is_ok());
    }

    #[test]
    fn extracts_zip_and_rejects_traversal_entries() -> Result<()> {
        use std::io::Write;
        let root = std::env::temp_dir().join(format!("localcraft-zip-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root)?;
        let source = root.join("runtime.zip");
        let mut writer = zip::ZipWriter::new(fs::File::create(&source)?);
        let name = if cfg!(windows) {
            "jdk/bin/java.exe"
        } else {
            "jdk/bin/java"
        };
        writer.start_file(name, zip::write::SimpleFileOptions::default())?;
        writer.write_all(b"java")?;
        writer.finish()?;
        let destination = root.join("extracted");
        fs::create_dir(&destination)?;
        let binary = extract(&source, &destination, ArchiveFormat::Zip)?;
        assert_eq!(fs::read(destination.join(binary))?, b"java");
        let mut writer = zip::ZipWriter::new(fs::File::create(&source)?);
        writer.start_file("../outside", zip::write::SimpleFileOptions::default())?;
        writer.write_all(b"unsafe")?;
        writer.finish()?;
        assert!(extract(&source, &destination, ArchiveFormat::Zip).is_err());
        assert!(!root.join("outside").exists());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn extracts_java_and_restores_executable_permission() -> Result<()> {
        let root =
            std::env::temp_dir().join(format!("localcraft-extract-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root)?;
        let source = root.join("runtime.tar.gz");
        let encoder = flate2::write::GzEncoder::new(
            fs::File::create(&source)?,
            flate2::Compression::default(),
        );
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o644);
        header.set_cksum();
        let name = if cfg!(windows) {
            "jdk/bin/java.exe"
        } else {
            "jdk/bin/java"
        };
        builder.append_data(&mut header, name, &b"java"[..])?;
        builder.into_inner()?.finish()?;
        let destination = root.join("extracted");
        fs::create_dir(&destination)?;
        let binary = extract(&source, &destination, ArchiveFormat::TarGz)?;
        assert_eq!(fs::read(destination.join(&binary))?, b"java");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(
                fs::metadata(destination.join(binary))?.permissions().mode() & 0o111,
                0
            );
        }
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
