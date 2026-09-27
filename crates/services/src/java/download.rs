use std::{
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt, sync::watch};

use super::{JavaDownload, JavaProgress, JavaStage};

pub(super) async fn save(
    client: &Client,
    archive: &JavaDownload,
    path: &Path,
    progress: &watch::Sender<JavaProgress>,
) -> Result<()> {
    if archive.sha256.len() != 64 || !archive.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("Invalid Java SHA256 in provider metadata");
    }
    let mut response = client
        .get(&archive.url)
        .send()
        .await
        .context("Could not download Java archive")?
        .error_for_status()?;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .await?;
    let mut digest = Sha256::new();
    let mut downloaded = 0_u64;
    let mut last_update = Instant::now();
    progress.send_modify(|value| {
        value.stage = JavaStage::Downloading;
        value.total = Some(archive.size);
    });
    while let Some(chunk) = response
        .chunk()
        .await
        .context("Could not read Java download chunk")?
    {
        downloaded += chunk.len() as u64;
        if downloaded > archive.size {
            bail!("Java download exceeds its expected size");
        }
        file.write_all(&chunk).await?;
        digest.update(&chunk);
        if last_update.elapsed() >= Duration::from_millis(100) {
            progress.send_modify(|value| value.downloaded = downloaded);
            last_update = Instant::now();
        }
    }
    progress.send_modify(|value| {
        value.stage = JavaStage::Verifying;
        value.downloaded = downloaded;
    });
    if downloaded == 0 || downloaded != archive.size {
        bail!("Incomplete Java download");
    }
    if !format!("{:x}", digest.finalize()).eq_ignore_ascii_case(&archive.sha256) {
        bail!("Java archive SHA256 verification failed");
    }
    file.sync_all().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::ArchiveFormat;
    use super::*;
    use tokio::{io::AsyncReadExt, net::TcpListener};

    #[tokio::test]
    async fn streamed_download_checks_digest_and_reports_bytes() -> Result<()> {
        let root =
            std::env::temp_dir().join(format!("localcraft-java-download-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).await?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}/java", listener.local_addr()?);
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await?;
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    if request.len() >= 8192 {
                        bail!("Test request header is too large");
                    }
                    request.push(socket.read_u8().await?);
                }
                socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\njava",
                    )
                    .await?;
            }
            Ok::<_, anyhow::Error>(())
        });
        let mut archive = JavaDownload {
            url,
            sha256: "0".repeat(64),
            size: 4,
            format: ArchiveFormat::Zip,
        };
        let (progress, updates) = watch::channel(JavaProgress::new(21));
        let client = Client::new();
        assert!(
            save(&client, &archive, &root.join("bad"), &progress)
                .await
                .is_err()
        );
        archive.sha256 = format!("{:x}", Sha256::digest(b"java"));
        save(&client, &archive, &root.join("good"), &progress).await?;
        assert_eq!(fs::read(root.join("good")).await?, b"java");
        assert_eq!(updates.borrow().downloaded, 4);
        server.await??;
        fs::remove_dir_all(root).await?;
        Ok(())
    }
}
