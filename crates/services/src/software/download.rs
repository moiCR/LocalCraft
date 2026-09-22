use std::{
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt, sync::watch};
use uuid::Uuid;

use super::{DownloadProgress, DownloadStage, JarDownload};

pub(super) async fn save(
    client: &Client,
    jar: &JarDownload,
    expected: &str,
    destination: &Path,
    progress: &watch::Sender<DownloadProgress>,
) -> Result<()> {
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("Expected SHA256 must contain 64 hexadecimal characters");
    }
    let temporary = destination.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let result = transfer(client, jar, expected, &temporary, destination, progress).await;
    if result.is_err() {
        let _ = fs::remove_file(&temporary).await;
    }
    result
}

async fn transfer(
    client: &Client,
    jar: &JarDownload,
    expected: &str,
    temporary: &Path,
    destination: &Path,
    progress: &watch::Sender<DownloadProgress>,
) -> Result<()> {
    let mut response = client
        .get(&jar.url)
        .send()
        .await
        .context("Could not download server jar")?
        .error_for_status()?;
    let total = jar.size.or(response.content_length());
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(temporary)
        .await?;
    let mut digest = Sha256::new();
    let mut downloaded = 0_u64;
    let mut last_update = Instant::now();
    progress.send_modify(|value| {
        value.stage = DownloadStage::Downloading;
        value.total = total;
    });
    while let Some(chunk) = response
        .chunk()
        .await
        .context("Could not read download chunk")?
    {
        downloaded += chunk.len() as u64;
        if total.is_some_and(|total| downloaded > total) {
            bail!("Download exceeds its expected size");
        }
        file.write_all(&chunk).await?;
        digest.update(&chunk);
        if last_update.elapsed() >= Duration::from_millis(100) {
            progress.send_modify(|value| value.downloaded = downloaded);
            last_update = Instant::now();
        }
    }
    progress.send_modify(|value| {
        value.stage = DownloadStage::Verifying;
        value.downloaded = downloaded;
    });
    if downloaded == 0 || total.is_some_and(|total| downloaded != total) {
        bail!("Incomplete server jar download");
    }
    if !format!("{:x}", digest.finalize()).eq_ignore_ascii_case(expected) {
        bail!("Server jar SHA256 verification failed");
    }
    file.sync_all().await?;
    drop(file);
    fs::rename(temporary, destination)
        .await
        .context("Could not install verified jar")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::software::JarKind;
    use tokio::{io::AsyncReadExt, net::TcpListener};

    async fn serve(body: &'static [u8]) -> Result<(String, tokio::task::JoinHandle<Result<()>>)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut request = [0_u8; 4096];
            stream.read(&mut request).await?;
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await?;
            stream.write_all(body).await?;
            Ok(())
        });
        Ok((format!("http://{address}/server.jar"), task))
    }

    #[tokio::test]
    async fn verified_download_replaces_file_and_failure_preserves_it() -> Result<()> {
        let root = std::env::temp_dir().join(format!("localcraft-download-{}", Uuid::new_v4()));
        fs::create_dir(&root).await?;
        let target = root.join("server.jar");
        fs::write(&target, b"old").await?;
        let client = Client::new();
        let (progress, receiver) = watch::channel(DownloadProgress {
            instance_id: "test".into(),
            stage: DownloadStage::Resolving,
            downloaded: 0,
            total: None,
        });
        let (url, task) = serve(b"new jar").await?;
        let jar = JarDownload {
            url,
            sha256: None,
            size: Some(7),
            kind: JarKind::Server,
        };
        assert!(
            save(&client, &jar, &"0".repeat(64), &target, &progress)
                .await
                .is_err()
        );
        task.await??;
        assert_eq!(fs::read(&target).await?, b"old");
        let (url, task) = serve(b"new jar").await?;
        let jar = JarDownload { url, ..jar };
        let hash = format!("{:x}", Sha256::digest(b"new jar"));
        save(&client, &jar, &hash, &target, &progress).await?;
        task.await??;
        assert_eq!(fs::read(&target).await?, b"new jar");
        assert_eq!(receiver.borrow().downloaded, 7);
        let mut entries = fs::read_dir(&root).await?;
        let mut count = 0;
        while entries.next_entry().await?.is_some() {
            count += 1;
        }
        assert_eq!(count, 1);
        fs::remove_dir_all(root).await?;
        Ok(())
    }
}
