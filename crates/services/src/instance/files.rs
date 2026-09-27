use std::{
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result, bail};
use tokio::{
    fs::{self, File, OpenOptions},
    io::copy,
};

use super::{ServerInstance, server_instance::atomic_write};

pub const MAX_EDIT_SIZE: u64 = 2 * 1024 * 1024;

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

pub async fn list(server: &ServerInstance, relative: &Path) -> Result<Vec<FileEntry>> {
    let root = root(server).await?;
    let directory = existing_path(&root, relative).await?;
    let metadata = fs::metadata(&directory)
        .await
        .with_context(|| format!("Could not inspect {}", directory.display()))?;
    if !metadata.is_dir() {
        bail!("Selected path is not a directory");
    }

    let mut entries = fs::read_dir(&directory)
        .await
        .with_context(|| format!("Could not list {}", directory.display()))?;
    let mut result = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let metadata = fs::symlink_metadata(entry.path())
            .await
            .with_context(|| format!("Could not inspect {}", entry.path().display()))?;
        let is_symlink = metadata.file_type().is_symlink();
        let is_directory = !is_symlink && metadata.is_dir();
        result.push(FileEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: relative.join(entry.file_name()),
            is_directory,
            is_symlink,
            size: metadata.len(),
            modified: metadata.modified().ok(),
        });
    }
    result.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(result)
}

pub async fn read_text(server: &ServerInstance, relative: &Path) -> Result<String> {
    let root = root(server).await?;
    let path = existing_path(&root, relative).await?;
    let metadata = fs::symlink_metadata(&path)
        .await
        .with_context(|| format!("Could not inspect {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("Selected path is not a regular file");
    }
    if metadata.len() > MAX_EDIT_SIZE {
        bail!("Files larger than 2 MiB cannot be opened in the editor");
    }
    fs::read_to_string(&path)
        .await
        .with_context(|| format!("File is not readable as UTF-8 text: {}", path.display()))
}

pub async fn save_text(server: &ServerInstance, relative: &Path, contents: &str) -> Result<()> {
    let root = root(server).await?;
    let path = existing_path(&root, relative).await?;
    let metadata = fs::symlink_metadata(&path)
        .await
        .with_context(|| format!("Could not inspect {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("Only regular files inside the instance can be edited");
    }
    if contents.len() as u64 > MAX_EDIT_SIZE {
        bail!("Edited file exceeds the 2 MiB limit");
    }
    atomic_write(&path, contents.as_bytes()).await
}

pub async fn upload_file(server: &ServerInstance, directory: &Path, source: &Path) -> Result<()> {
    let root = root(server).await?;
    let directory = existing_path(&root, directory).await?;
    let directory_metadata = fs::metadata(&directory)
        .await
        .with_context(|| format!("Could not inspect {}", directory.display()))?;
    if !directory_metadata.is_dir() {
        bail!("Upload destination is not a directory");
    }

    let source_metadata = fs::symlink_metadata(source)
        .await
        .with_context(|| format!("Could not inspect upload source {}", source.display()))?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
        bail!("Only regular files can be uploaded");
    }
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .context("Upload source has an invalid file name")?;
    validate_name(name)?;

    let target = directory.join(name);
    let mut input = File::open(source)
        .await
        .with_context(|| format!("Could not open upload source {}", source.display()))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .await
        .with_context(|| format!("Could not create upload target {}", target.display()))?;

    let result = async {
        copy(&mut input, &mut output)
            .await
            .with_context(|| format!("Could not copy upload source to {}", target.display()))?;
        output
            .sync_all()
            .await
            .with_context(|| format!("Could not flush uploaded file {}", target.display()))
    }
    .await;

    if let Err(error) = result {
        drop(output);
        if let Err(cleanup_error) = fs::remove_file(&target).await {
            return Err(error).with_context(|| {
                format!(
                    "Could not remove incomplete upload {}: {cleanup_error}",
                    target.display()
                )
            });
        }
        return Err(error);
    }

    Ok(())
}

pub async fn create_directory(server: &ServerInstance, parent: &Path, name: &str) -> Result<()> {
    validate_name(name)?;
    let root = root(server).await?;
    let parent = existing_path(&root, parent).await?;
    let path = parent.join(name);
    ensure_missing(&path).await?;
    fs::create_dir(&path)
        .await
        .with_context(|| format!("Could not create directory {}", path.display()))
}

pub async fn rename(server: &ServerInstance, relative: &Path, name: &str) -> Result<()> {
    validate_name(name)?;
    let root = root(server).await?;
    if relative.as_os_str().is_empty() {
        bail!("Cannot rename the instance root");
    }
    let source = existing_path(&root, relative).await?;
    let metadata = fs::symlink_metadata(&source)
        .await
        .with_context(|| format!("Could not inspect {}", source.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("Symbolic links cannot be renamed");
    }
    let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
    let parent = existing_path(&root, parent_relative).await?;
    let target = parent.join(name);
    ensure_missing(&target).await?;
    fs::rename(&source, &target)
        .await
        .with_context(|| format!("Could not rename {}", source.display()))
}

pub async fn delete(server: &ServerInstance, relative: &Path) -> Result<()> {
    let root = root(server).await?;
    if relative.as_os_str().is_empty() {
        bail!("Cannot delete the instance root");
    }
    let path = existing_path(&root, relative).await?;
    let metadata = fs::symlink_metadata(&path)
        .await
        .with_context(|| format!("Could not inspect {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        fs::remove_file(&path)
            .await
            .with_context(|| format!("Could not remove symbolic link {}", path.display()))
    } else if metadata.is_dir() {
        fs::remove_dir_all(&path)
            .await
            .with_context(|| format!("Could not delete directory {}", path.display()))
    } else {
        fs::remove_file(&path)
            .await
            .with_context(|| format!("Could not delete file {}", path.display()))
    }
}

async fn root(server: &ServerInstance) -> Result<PathBuf> {
    fs::canonicalize(server.directory()?)
        .await
        .context("Could not resolve instance directory")
}

async fn existing_path(root: &Path, relative: &Path) -> Result<PathBuf> {
    validate_relative(relative)?;
    let path = root.join(relative);
    let canonical = fs::canonicalize(&path)
        .await
        .with_context(|| format!("Could not resolve {}", path.display()))?;
    if !canonical.starts_with(root) {
        bail!("Path escapes the instance directory");
    }
    Ok(path)
}

fn validate_relative(path: &Path) -> Result<()> {
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("Invalid instance-relative path");
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', '\0'])
        || Path::new(name).components().count() != 1
        || !matches!(
            Path::new(name).components().next(),
            Some(Component::Normal(_))
        )
    {
        bail!("Enter a valid file or directory name");
    }
    Ok(())
}

async fn ensure_missing(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path).await {
        Ok(_) => bail!("An item with that name already exists"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("Could not inspect {}", path.display())),
    }
}
