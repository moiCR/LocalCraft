#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::path::PathBuf;
use std::{
    fs::File,
    io::Read,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;
#[cfg(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "windows", target_arch = "x86_64")
))]
use tokio::process::Command;
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;

const RELEASE_URL: &str =
    "https://github.com/moiCR/LocalCraft/releases/latest/download/latest.json";
const RELEASE_DOWNLOAD_PREFIX: &str = "https://github.com/moiCR/LocalCraft/releases/download/";
const RELEASE_PAGE_PREFIX: &str = "https://github.com/moiCR/LocalCraft/releases/tag/v";
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDk3RjQxQTc3MTREMjU1NDMKUldSRFZkSVVkeHIwbHp5cExQT1FDOC9YazRQVXZoajlLZUEwOU1NNkMxbTBNRndKUDA0UmxseVgK";
const MAX_UPDATE_BYTES: u64 = 1_073_741_824;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UpdatePhase {
    #[default]
    Downloading = 0,
    Verifying = 1,
    Installing = 2,
}

#[derive(Default)]
pub struct UpdateProgress {
    pub downloaded_bytes: AtomicU64,
    pub total_bytes: AtomicU64,
    phase: AtomicU8,
}

impl UpdateProgress {
    pub fn downloaded(&self) -> u64 {
        self.downloaded_bytes.load(Ordering::Relaxed)
    }

    pub fn total(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }

    pub fn phase(&self) -> UpdatePhase {
        match self.phase.load(Ordering::Relaxed) {
            1 => UpdatePhase::Verifying,
            2 => UpdatePhase::Installing,
            _ => UpdatePhase::Downloading,
        }
    }

    pub fn set_phase(&self, phase: UpdatePhase) {
        self.phase.store(phase as u8, Ordering::Relaxed);
    }

    pub fn reset(&self, total: u64) {
        self.downloaded_bytes.store(0, Ordering::Relaxed);
        self.total_bytes.store(total, Ordering::Relaxed);
        self.set_phase(UpdatePhase::Downloading);
    }
}

#[derive(Clone, Debug)]
pub struct AvailableUpdate {
    pub version: String,
    pub notes: Option<String>,
    pub release_page: String,
    pub expected_size: Option<u64>,
    package: Option<SignedPackage>,
}

impl AvailableUpdate {
    pub fn can_auto_install(&self) -> bool {
        self.package.is_some()
    }
}

#[derive(Clone, Debug)]
struct SignedPackage {
    url: String,
    signature: String,
    filename: String,
    target: UpdateTarget,
}

#[derive(Clone, Debug)]
enum UpdateTarget {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    Windows,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    LinuxAppImage(PathBuf),
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    LinuxDeb,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    LinuxRpm,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    LinuxUnrecognized,
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64")
    )))]
    Unsupported,
}

#[derive(Deserialize)]
struct ReleaseManifest {
    version: String,
    notes: Option<String>,
    platforms: std::collections::HashMap<String, PlatformRelease>,
}

#[derive(Deserialize)]
struct PlatformRelease {
    url: String,
    signature: String,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Clone)]
pub struct UpdaterService {
    client: reqwest::Client,
}

impl UpdaterService {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("LocalCraft/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("Could not initialize update client")?;
        Ok(Self { client })
    }

    pub async fn check(&self, current_version: &str) -> Result<Option<AvailableUpdate>> {
        let target = detect_target().await?;
        let response = self
            .client
            .get(RELEASE_URL)
            .send()
            .await
            .context("Could not check for LocalCraft updates")?
            .error_for_status()
            .context("Update service returned an unsuccessful response")?;
        let manifest: ReleaseManifest = response
            .json()
            .await
            .context("Could not read the update manifest")?;
        let clean_remote = manifest.version.trim().trim_start_matches(['v', 'V']);
        let remote = semver::Version::parse(clean_remote)
            .context("Update manifest contains an invalid version")?;
        let clean_current = current_version.trim().trim_start_matches(['v', 'V']);
        let current = semver::Version::parse(clean_current)
            .context("Current application version is invalid")?;
        if remote <= current {
            return Ok(None);
        }

        let release_page = format!("{RELEASE_PAGE_PREFIX}{clean_remote}");
        let package = if let Some(platform) = target.resolve_platform(&manifest.platforms) {
            let filename = validate_release_url(&platform.url, clean_remote)?;
            if platform.signature.trim().is_empty() {
                bail!("Update manifest has no package signature");
            }
            if platform
                .size
                .is_some_and(|size| size == 0 || size > MAX_UPDATE_BYTES)
            {
                bail!("Update manifest contains an invalid package size");
            }
            Some((
                SignedPackage {
                    url: platform.url.clone(),
                    signature: platform.signature.clone(),
                    filename,
                    target,
                },
                platform.size,
            ))
        } else {
            None
        };

        let (package, expected_size) = match package {
            Some((package, expected_size)) => (Some(package), expected_size),
            None => (None, None),
        };

        Ok(Some(AvailableUpdate {
            version: clean_remote.to_owned(),
            notes: manifest.notes,
            release_page,
            expected_size,
            package,
        }))
    }

    pub async fn download_and_install(
        &self,
        update: &AvailableUpdate,
        progress: Arc<UpdateProgress>,
    ) -> Result<()> {
        let Some(package) = &update.package else {
            bail!(
                "Automatic updates are unavailable for this installation. Download the update from {}",
                update.release_page
            );
        };

        decode_public_key(PUBLIC_KEY)?;
        progress.reset(update.expected_size.unwrap_or(0));

        let directory = std::env::temp_dir().join("LocalCraft").join("updates");
        create_private_dir(&directory).await?;
        cleanup_stale_files(&directory).await;

        let package_path = directory.join(&package.filename);
        download_and_verify_to_path(
            &self.client,
            &package.url,
            &package_path,
            PUBLIC_KEY,
            &package.signature,
            &update.version,
            &package.filename,
            update.expected_size,
            Arc::clone(&progress),
        )
        .await?;

        progress.set_phase(UpdatePhase::Installing);
        let install_result =
            install_package(&package.target, &update.release_page, &package_path).await;
        #[cfg(not(windows))]
        let _ = fs::remove_file(&package_path).await;
        #[cfg(windows)]
        if install_result.is_err() {
            let _ = fs::remove_file(&package_path).await;
        }
        install_result
    }
}

async fn create_private_dir(directory: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder
        .create(directory)
        .await
        .context("Could not create update directory")
}

async fn cleanup_stale_files(directory: &Path) {
    let Ok(mut entries) = fs::read_dir(directory).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if fs::metadata(&path)
            .await
            .is_ok_and(|metadata| metadata.is_file())
        {
            let _ = fs::remove_file(&path).await;
        }
    }
}

async fn download_stream_to_part(
    client: &reqwest::Client,
    url: &str,
    temporary: &Path,
    expected_size: Option<u64>,
    progress: &UpdateProgress,
) -> Result<()> {
    let mut response = client
        .get(url)
        .send()
        .await
        .context("Could not download the LocalCraft update")?
        .error_for_status()
        .context("Update download returned an unsuccessful response")?;

    let content_length = response.content_length();
    if let Some(length) = content_length {
        if length > MAX_UPDATE_BYTES {
            bail!("Update package exceeds the maximum supported size");
        }
        if let Some(expected) = expected_size
            && expected != length
        {
            bail!("Update package size does not match the manifest");
        }
        progress.total_bytes.store(length, Ordering::Relaxed);
    }

    let max_allowed = expected_size
        .unwrap_or(MAX_UPDATE_BYTES)
        .min(MAX_UPDATE_BYTES);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)
        .await
        .context("Could not create temporary update file")?;
    let mut downloaded = 0_u64;
    while let Some(chunk) = response
        .chunk()
        .await
        .context("Update download was interrupted")?
    {
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        if downloaded > max_allowed {
            bail!("Update package exceeds the expected size");
        }
        file.write_all(&chunk)
            .await
            .context("Could not write the update package")?;
        progress
            .downloaded_bytes
            .store(downloaded, Ordering::Relaxed);
    }

    let required_size = expected_size.or(content_length);
    if required_size.is_some_and(|total| total != downloaded) {
        bail!("Update download is incomplete");
    }

    file.sync_all()
        .await
        .context("Could not finish writing the update package")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn download_and_verify_to_path(
    client: &reqwest::Client,
    url: &str,
    package_path: &Path,
    public_key_text: &str,
    signature_text: &str,
    version: &str,
    expected_filename: &str,
    expected_size: Option<u64>,
    progress: Arc<UpdateProgress>,
) -> Result<()> {
    progress.reset(expected_size.unwrap_or(0));

    if let Some(parent) = package_path.parent() {
        create_private_dir(parent).await?;
    }

    let temporary = package_path.with_extension(format!("{}.part", Uuid::new_v4()));
    let stage_result = async {
        download_stream_to_part(client, url, &temporary, expected_size, &progress).await?;

        progress.set_phase(UpdatePhase::Verifying);
        let verify_path = temporary.clone();
        let key = public_key_text.to_owned();
        let signature = signature_text.to_owned();
        let version = version.to_owned();
        let filename = expected_filename.to_owned();
        tokio::task::spawn_blocking(move || {
            verify_signature(&verify_path, &key, &signature, &version, &filename)
        })
        .await
        .context("Update verification worker stopped unexpectedly")??;

        fs::rename(&temporary, package_path)
            .await
            .context("Could not stage verified update package")?;
        Ok(())
    }
    .await;

    if stage_result.is_err() {
        let _ = fs::remove_file(&temporary).await;
    }
    stage_result
}

fn validate_release_url(url: &str, version: &str) -> Result<String> {
    let prefix = format!("{RELEASE_DOWNLOAD_PREFIX}v{version}/");
    let Some(filename) = url.strip_prefix(&prefix) else {
        bail!("Update package URL does not match the LocalCraft release version");
    };
    if filename.is_empty()
        || matches!(filename, "." | "..")
        || filename.contains(['/', '\\', '?', '#'])
        || filename.chars().any(char::is_control)
    {
        bail!("Update package URL contains an invalid file name");
    }
    Ok(filename.to_owned())
}

impl UpdateTarget {
    fn manifest_keys(&self) -> &'static [&'static str] {
        match self {
            #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
            Self::Windows => &["windows-x86_64-nsis", "windows-x86_64"],
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxAppImage(_) => &["linux-x86_64-appimage", "linux-x86_64"],
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxDeb => &["linux-x86_64-deb"],
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxRpm => &["linux-x86_64-rpm"],
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxUnrecognized => &[],
            #[cfg(not(any(
                all(target_os = "windows", target_arch = "x86_64"),
                all(target_os = "linux", target_arch = "x86_64")
            )))]
            Self::Unsupported => &[],
        }
    }

    fn resolve_platform<'a>(
        &self,
        platforms: &'a std::collections::HashMap<String, PlatformRelease>,
    ) -> Option<&'a PlatformRelease> {
        self.manifest_keys()
            .iter()
            .find_map(|key| platforms.get(*key))
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
async fn detect_target() -> Result<UpdateTarget> {
    Ok(UpdateTarget::Windows)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
async fn detect_target() -> Result<UpdateTarget> {
    if let Some(appimage) = std::env::var_os("APPIMAGE").filter(|value| !value.is_empty()) {
        let appimage = PathBuf::from(appimage);
        let has_extension = appimage
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("AppImage"));
        let is_file = fs::metadata(&appimage)
            .await
            .is_ok_and(|metadata| metadata.is_file());
        if has_extension || is_file {
            return Ok(UpdateTarget::LinuxAppImage(appimage));
        }
    }

    let executable =
        std::env::current_exe().context("Could not locate the LocalCraft executable")?;
    if let Some(directory) = executable.parent() {
        match fs::read_to_string(directory.join("package-format")).await {
            Ok(marker) => match marker.trim() {
                "deb" => return Ok(UpdateTarget::LinuxDeb),
                "rpm" => return Ok(UpdateTarget::LinuxRpm),
                _ => {}
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Could not read package-format marker"),
        }
    }

    if package_manager_owns("dpkg-query", &["-S"], &executable).await? {
        return Ok(UpdateTarget::LinuxDeb);
    }
    if package_manager_owns("rpm", &["-qf"], &executable).await? {
        return Ok(UpdateTarget::LinuxRpm);
    }
    Ok(UpdateTarget::LinuxUnrecognized)
}

#[cfg(not(any(
    all(target_os = "windows", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64")
)))]
async fn detect_target() -> Result<UpdateTarget> {
    Ok(UpdateTarget::Unsupported)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
async fn package_manager_owns(
    manager: &str,
    arguments: &[&str],
    executable: &Path,
) -> Result<bool> {
    match Command::new(manager)
        .args(arguments)
        .arg(executable)
        .output()
        .await
    {
        Ok(output) => Ok(output.status.success()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("Could not query {manager}")),
    }
}

async fn install_package(
    target: &UpdateTarget,
    release_page: &str,
    package_path: &Path,
) -> Result<()> {
    match target {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        UpdateTarget::Windows => {
            let _ = release_page;
            Command::new(package_path)
                .args(["/P", "/UPDATE", "/R"])
                .spawn()
                .context("Could not start the LocalCraft installer")?;
            Ok(())
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        UpdateTarget::LinuxAppImage(appimage) => replace_appimage(package_path, appimage)
            .await
            .with_context(|| {
                format!("Could not update the AppImage. Download it manually from {release_page}")
            }),
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        UpdateTarget::LinuxDeb => {
            run_elevated_installer("apt-get", &["install", "--yes"], package_path)
                .await
                .with_context(|| {
                    format!(
                        "Could not install the Debian update. Download it manually from {release_page}"
                    )
                })?;
            restart_application().await.with_context(|| {
                format!(
                    "The update was installed. Start LocalCraft again or download it from {release_page}"
                )
            })
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        UpdateTarget::LinuxRpm => {
            let (manager, arguments): (&str, &[&str]) = if executable_available("dnf").await {
                ("dnf", &["install", "--assumeyes", "--nogpgcheck"])
            } else if executable_available("yum").await {
                ("yum", &["install", "--assumeyes", "--nogpgcheck"])
            } else if executable_available("zypper").await {
                (
                    "zypper",
                    &["--no-gpg-checks", "--non-interactive", "install"],
                )
            } else {
                bail!(
                    "No supported RPM package manager is installed. Download the update from {release_page}"
                );
            };
            run_elevated_installer(manager, arguments, package_path)
                .await
                .with_context(|| {
                    format!(
                        "Could not install the RPM update. Download it manually from {release_page}"
                    )
                })?;
            restart_application().await.with_context(|| {
                format!(
                    "The update was installed. Start LocalCraft again or download it from {release_page}"
                )
            })
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        UpdateTarget::LinuxUnrecognized => bail!(
            "Automatic updates are unavailable for this Linux installation. Download the update from {release_page}"
        ),
        #[cfg(not(any(
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "x86_64")
        )))]
        UpdateTarget::Unsupported => {
            let _ = package_path;
            bail!(
                "Automatic updates are unavailable for this platform. Download the update from {release_page}"
            )
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
async fn replace_appimage(download: &Path, appimage: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let destination = fs::canonicalize(appimage)
        .await
        .context("Could not locate the current AppImage file")?;
    let parent = destination
        .parent()
        .context("AppImage path has no parent directory")?;
    let update_id = Uuid::new_v4();
    let staged = parent.join(format!(".LocalCraft-{update_id}.AppImage"));
    let backup = parent.join(format!(".LocalCraft-{update_id}.backup"));
    fs::copy(download, &staged)
        .await
        .context("Could not stage the AppImage update")?;
    if let Err(error) = fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).await {
        let _ = fs::remove_file(&staged).await;
        return Err(error).context("Could not make the updated AppImage executable");
    }

    if let Err(error) = fs::rename(&destination, &backup).await {
        let _ = fs::remove_file(&staged).await;
        return Err(error).context("Could not prepare the current AppImage for replacement");
    }
    if let Err(error) = fs::rename(&staged, &destination).await {
        let restore_result = fs::rename(&backup, &destination).await;
        let _ = fs::remove_file(&staged).await;
        if let Err(restore_error) = restore_result {
            return Err(restore_error).context(format!(
                "Could not restore the previous AppImage after replacement failed: {error}"
            ));
        }
        return Err(error).context("Could not replace the current AppImage");
    }

    if let Err(error) = Command::new(&destination).spawn() {
        let _ = fs::remove_file(&destination).await;
        let restore_result = fs::rename(&backup, &destination).await;
        if let Err(restore_error) = restore_result {
            return Err(restore_error).context(format!(
                "Could not restore the previous AppImage after restart failed: {error}"
            ));
        }
        return Err(error).context("Could not restart LocalCraft after the AppImage update");
    }
    let _ = fs::remove_file(&backup).await;
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
async fn executable_available(executable: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    for directory in std::env::split_paths(&path) {
        if fs::metadata(directory.join(executable))
            .await
            .is_ok_and(|metadata| metadata.is_file())
        {
            return true;
        }
    }
    false
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
async fn run_elevated_installer(
    manager: &str,
    arguments: &[&str],
    package_path: &Path,
) -> Result<()> {
    if !executable_available("pkexec").await {
        bail!("System authentication helper pkexec is not installed");
    }
    if !executable_available(manager).await {
        bail!("Package manager {manager} is not installed");
    }
    let status = Command::new("pkexec")
        .arg(manager)
        .args(arguments)
        .arg(package_path)
        .status()
        .await
        .context("Could not request system authentication for the update")?;
    if !status.success() {
        bail!("System package installation was canceled or failed");
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
async fn restart_application() -> Result<()> {
    let executable =
        std::env::current_exe().context("Could not locate the LocalCraft executable")?;
    Command::new(executable)
        .spawn()
        .context("Could not restart LocalCraft after package installation")?;
    Ok(())
}

fn decode_public_key(key_text: &str) -> Result<PublicKey> {
    let trimmed = key_text.trim();
    if trimmed.is_empty() {
        bail!("Updater public key is empty");
    }
    if trimmed.starts_with("untrusted comment:") {
        return PublicKey::decode(trimmed).context("Could not parse the updater public key");
    }
    if let Ok(public_key) = PublicKey::from_base64(trimmed) {
        return Ok(public_key);
    }
    let decoded_bytes = STANDARD
        .decode(trimmed)
        .context("Could not decode the updater public key")?;
    let decoded_text =
        String::from_utf8(decoded_bytes).context("Updater public key is not valid UTF-8")?;
    let decoded_trimmed = decoded_text.trim();
    if decoded_trimmed.starts_with("untrusted comment:") {
        PublicKey::decode(decoded_trimmed).context("Could not parse the updater public key")
    } else {
        PublicKey::from_base64(decoded_trimmed).context("Could not parse the updater public key")
    }
}

fn decode_signature(signature_text: &str) -> Result<Signature> {
    let trimmed = signature_text.trim();
    if trimmed.starts_with("untrusted comment:") {
        return Signature::decode(trimmed).context("Could not parse the update package signature");
    }
    let decoded_bytes = STANDARD
        .decode(trimmed)
        .context("Could not decode the update package signature")?;
    let decoded_text =
        String::from_utf8(decoded_bytes).context("Update package signature is not valid UTF-8")?;
    Signature::decode(decoded_text.trim()).context("Could not parse the update package signature")
}

fn validate_trusted_comment(
    trusted_comment: &str,
    version: &str,
    expected_filename: &str,
) -> Result<()> {
    let tokens: Vec<&str> = trusted_comment.split_whitespace().collect();
    let expected_version = format!("version:{version}");
    let expected_app_version = format!("app-version:{version}");
    if !tokens
        .iter()
        .any(|field| *field == expected_version || *field == expected_app_version)
    {
        bail!("Update signature does not match the manifest version");
    }
    let expected_file = format!("file:{expected_filename}");
    if tokens
        .iter()
        .filter(|field| field.starts_with("file:"))
        .any(|field| *field != expected_file)
    {
        bail!("Update signature does not match the expected package file");
    }
    Ok(())
}

fn verify_signature_stream(
    package_path: &Path,
    public_key: &PublicKey,
    signature: &Signature,
) -> Result<()> {
    let mut verifier = public_key
        .verify_stream(signature)
        .context("Could not verify the update package")?;
    let mut file = File::open(package_path).context("Could not open the update package")?;
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .context("Could not read the update package")?;
        if read == 0 {
            break;
        }
        verifier.update(&buffer[..read]);
    }
    verifier
        .finalize()
        .context("Update package signature is invalid")?;
    Ok(())
}

fn verify_signature(
    package_path: &Path,
    public_key_text: &str,
    signature_text: &str,
    version: &str,
    expected_filename: &str,
) -> Result<()> {
    let public_key = decode_public_key(public_key_text)?;
    let signature = decode_signature(signature_text)?;
    validate_trusted_comment(signature.trusted_comment(), version, expected_filename)?;
    verify_signature_stream(package_path, &public_key, &signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PUBLIC_KEY_B64: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const TEST_SIGNATURE_RAW: &str = "untrusted comment: signature from minisign secret key\n\
        RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\n\
        trusted comment: timestamp:1556193335\tfile:test\n\
        y/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==";
    const LIVE_SIGNATURE_B64: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSRFZkSVVkeHIwbDFnWit4UnBJdjlWeW52b2JQbW5nU0dmUjBmazdBajIzaUUzVUZuOU0zWEZyMlBFMzlKNUNMZmtpSFFwRVUxdGZSUmpMOFJOeWYzM3ZndkdzU0tRdkFZPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkwNTY5NTk2CWZpbGU6TG9jYWxDcmFmdF8xLjAuMF9hbWQ2NC5BcHBJbWFnZQl2ZXJzaW9uOjEuMC4wCjFWVXlvV29LbG1TZjZZb0tQczhacUZ4bExDdFJSOVZOMmZiU2t6WWk5dkg2eW1nT2xwR3FEbkQyZ3I1enVPY3VCWmdUYUR5ZHZLS3lCUUhiQ0V3MURBPT0K";

    #[test]
    fn decodes_raw_and_base64_public_keys_and_signatures() -> Result<()> {
        let embedded = decode_public_key(PUBLIC_KEY)?;
        let raw_two_line = format!(
            "untrusted comment: minisign public key: E7620F1842B4E81F\n{TEST_PUBLIC_KEY_B64}"
        );
        let from_raw_lines = decode_public_key(&raw_two_line)?;
        let from_raw_b64 = decode_public_key(TEST_PUBLIC_KEY_B64)?;
        let wrapped_b64 = STANDARD.encode(raw_two_line.as_bytes());
        let from_wrapped = decode_public_key(&wrapped_b64)?;
        assert_eq!(from_raw_lines, from_wrapped);
        assert!(from_raw_b64.untrusted_comment().is_none());

        let sig_raw = decode_signature(TEST_SIGNATURE_RAW)?;
        let sig_b64 = decode_signature(&STANDARD.encode(TEST_SIGNATURE_RAW.as_bytes()))?;
        assert_eq!(sig_raw.trusted_comment(), sig_b64.trusted_comment());
        assert!(from_raw_b64.verify_stream(&sig_b64).is_ok());
        assert!(from_wrapped.verify_stream(&sig_raw).is_ok());

        let live_sig = decode_signature(LIVE_SIGNATURE_B64)?;
        assert!(
            validate_trusted_comment(
                live_sig.trusted_comment(),
                "1.0.0",
                "LocalCraft_1.0.0_amd64.AppImage"
            )
            .is_ok()
        );
        assert!(embedded.verify_stream(&live_sig).is_ok());
        Ok(())
    }

    #[test]
    fn verifies_prehashed_signature_stream_and_trusted_comments() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("lc-updater-sig-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir)?;
        let file = dir.join("payload.bin");
        std::fs::write(&file, b"test")?;

        let key = decode_public_key(TEST_PUBLIC_KEY_B64)?;
        let sig = decode_signature(TEST_SIGNATURE_RAW)?;
        verify_signature_stream(&file, &key, &sig)?;

        std::fs::write(&file, b"tampered")?;
        assert!(verify_signature_stream(&file, &key, &sig).is_err());

        let embedded = decode_public_key(PUBLIC_KEY)?;
        assert!(embedded.verify_stream(&sig).is_err());

        std::fs::remove_dir_all(&dir)?;

        assert!(
            validate_trusted_comment(
                "timestamp:1790570130\tfile:LocalCraft_1.1.0_x64-setup.exe\tversion:1.1.0",
                "1.1.0",
                "LocalCraft_1.1.0_x64-setup.exe",
            )
            .is_ok()
        );
        assert!(
            validate_trusted_comment(
                "timestamp:1790570130\tfile:LocalCraft_1.1.0_x64-setup.exe\tapp-version:1.1.0",
                "1.1.0",
                "LocalCraft_1.1.0_x64-setup.exe",
            )
            .is_ok()
        );
        assert!(
            validate_trusted_comment("version:1.1.0", "1.1.0", "LocalCraft_1.1.0_x64-setup.exe")
                .is_ok()
        );
        assert!(
            validate_trusted_comment(
                "timestamp:1790570130\tfile:LocalCraft_1.1.0_x64-setup.exe\tversion:1.0.9",
                "1.1.0",
                "LocalCraft_1.1.0_x64-setup.exe",
            )
            .is_err()
        );
        assert!(
            validate_trusted_comment(
                "timestamp:1790570130\tfile:Other.exe\tversion:1.1.0",
                "1.1.0",
                "LocalCraft_1.1.0_x64-setup.exe",
            )
            .is_err()
        );
        assert!(
            validate_trusted_comment(
                "timestamp:1790570130\tfile:LocalCraft.exe",
                "1.1.0",
                "LocalCraft.exe"
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn validates_release_urls_and_filenames() {
        assert_eq!(
            validate_release_url(
                "https://github.com/moiCR/LocalCraft/releases/download/v1.1.0/LocalCraft_1.1.0_x64-setup.exe",
                "1.1.0"
            )
            .ok()
            .as_deref(),
            Some("LocalCraft_1.1.0_x64-setup.exe")
        );
        assert!(
            validate_release_url(
                "https://github.com/moiCR/LocalCraft/releases/download/v1.0.0/LocalCraft_1.0.0_x64-setup.exe",
                "1.1.0"
            )
            .is_err()
        );
        assert!(
            validate_release_url(
                "https://evil.example.com/moiCR/LocalCraft/releases/download/v1.1.0/LocalCraft.exe",
                "1.1.0"
            )
            .is_err()
        );
        assert!(
            validate_release_url(
                "https://github.com/moiCR/LocalCraft/releases/download/v1.1.0/../evil.exe",
                "1.1.0"
            )
            .is_err()
        );
        assert!(
            validate_release_url(
                "https://github.com/moiCR/LocalCraft/releases/download/v1.1.0/sub/file.exe",
                "1.1.0"
            )
            .is_err()
        );
    }

    async fn serve_package(response: &'static [u8]) -> Result<String> {
        use tokio::{io::AsyncReadExt, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut request = [0; 4096];
            let _ = stream.read(&mut request).await?;
            stream.write_all(response).await?;
            stream.shutdown().await
        });
        Ok(format!("http://{address}/test"))
    }

    #[tokio::test]
    async fn downloads_atomically_with_progress_and_cleans_up_failures() -> Result<()> {
        let directory = std::env::temp_dir().join(format!("lc-updater-dl-{}", Uuid::new_v4()));
        create_private_dir(&directory).await?;
        let part_path = directory.join("test.part");
        let target = directory.join("test");
        let client = reqwest::Client::new();
        let progress = Arc::new(UpdateProgress::default());

        let url =
            serve_package(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest")
                .await?;
        download_stream_to_part(&client, &url, &part_path, Some(4), &progress).await?;
        assert_eq!(fs::read(&part_path).await?, b"test");
        assert_eq!(progress.downloaded(), 4);
        assert_eq!(progress.total(), 4);
        assert_eq!(progress.phase(), UpdatePhase::Downloading);
        fs::remove_file(&part_path).await?;

        let sig_missing_version = "untrusted comment: signature from minisign secret key\n\
            RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\n\
            trusted comment: timestamp:1556193335\tfile:test\n\
            y/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==";
        let url =
            serve_package(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest")
                .await?;
        let error = download_and_verify_to_path(
            &client,
            &url,
            &target,
            TEST_PUBLIC_KEY_B64,
            sig_missing_version,
            "1.1.0",
            "test",
            Some(4),
            Arc::clone(&progress),
        )
        .await;
        assert!(error.is_err());
        assert_eq!(progress.downloaded(), 4);
        assert_eq!(progress.total(), 4);
        assert_eq!(progress.phase(), UpdatePhase::Verifying);
        assert!(!fs::try_exists(&target).await?);
        assert!(
            fs::read_dir(&directory)
                .await?
                .next_entry()
                .await?
                .is_none()
        );

        for (response, expected) in [
            (
                &b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nshort"[..],
                Some(10),
            ),
            (
                &b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest"[..],
                Some(10),
            ),
            (
                &b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\ntoo-long"[..],
                Some(4),
            ),
            (
                &b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"[..],
                Some(4),
            ),
        ] {
            let url = serve_package(response).await?;
            assert!(
                download_and_verify_to_path(
                    &client,
                    &url,
                    &target,
                    TEST_PUBLIC_KEY_B64,
                    sig_missing_version,
                    "1.1.0",
                    "test",
                    expected,
                    Arc::new(UpdateProgress::default()),
                )
                .await
                .is_err()
            );
            assert!(!fs::try_exists(&target).await?);
            assert!(
                fs::read_dir(&directory)
                    .await?
                    .next_entry()
                    .await?
                    .is_none()
            );
        }

        fs::remove_dir(&directory).await?;
        Ok(())
    }
}
