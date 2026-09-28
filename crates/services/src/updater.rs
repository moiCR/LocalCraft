#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::path::PathBuf;
use std::{fs::File, io::Read, path::Path};

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

#[derive(Clone, Debug)]
pub struct AvailableUpdate {
    pub version: String,
    pub notes: Option<String>,
    url: String,
    signature: String,
    release_page: String,
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
        let remote = semver::Version::parse(&manifest.version)
            .context("Update manifest contains an invalid version")?;
        let current = semver::Version::parse(current_version)
            .context("Current application version is invalid")?;
        if remote <= current {
            return Ok(None);
        }

        let platform = target.platform(&manifest.platforms)?;
        if !platform.url.starts_with(RELEASE_DOWNLOAD_PREFIX) {
            bail!("Update package URL does not point to the LocalCraft releases");
        }
        if platform.signature.trim().is_empty() {
            bail!("Update manifest has no package signature");
        }

        Ok(Some(AvailableUpdate {
            version: manifest.version.clone(),
            notes: manifest.notes,
            url: platform.url.clone(),
            signature: platform.signature.clone(),
            release_page: format!("{RELEASE_PAGE_PREFIX}{}", manifest.version),
            target,
        }))
    }

    pub async fn download_and_install(&self, update: &AvailableUpdate) -> Result<()> {
        let Some(extension) = update.target.file_extension() else {
            bail!(
                "Automatic updates are unavailable for this installation. Download the update from {}",
                update.release_page
            );
        };

        let directory = std::env::temp_dir().join("LocalCraft").join("updates");
        fs::create_dir_all(&directory)
            .await
            .context("Could not create update directory")?;
        let package_path = directory.join(format!("{}.{}", Uuid::new_v4(), extension));
        let download_result = self.download_and_verify(update, &package_path).await;
        if let Err(error) = download_result {
            let _ = fs::remove_file(&package_path).await;
            return Err(error);
        }

        let install_result = install_package(update, &package_path).await;
        let _ = fs::remove_file(&package_path).await;
        install_result
    }

    async fn download_and_verify(
        &self,
        update: &AvailableUpdate,
        package_path: &Path,
    ) -> Result<()> {
        let mut response = self
            .client
            .get(&update.url)
            .send()
            .await
            .context("Could not download the LocalCraft update")?
            .error_for_status()
            .context("Update download returned an unsuccessful response")?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_UPDATE_BYTES)
        {
            bail!("Update package exceeds the maximum supported size");
        }

        let mut file = fs::File::create(package_path)
            .await
            .context("Could not create the update package file")?;
        let mut downloaded = 0_u64;
        while let Some(chunk) = response
            .chunk()
            .await
            .context("Update download was interrupted")?
        {
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            if downloaded > MAX_UPDATE_BYTES {
                bail!("Update package exceeds the maximum supported size");
            }
            file.write_all(&chunk)
                .await
                .context("Could not write the update package")?;
        }
        file.flush()
            .await
            .context("Could not finish writing the update package")?;
        drop(file);

        let package_path = package_path.to_owned();
        let signature = update.signature.clone();
        let version = update.version.clone();
        tokio::task::spawn_blocking(move || verify_signature(&package_path, &signature, &version))
            .await
            .context("Update verification worker stopped unexpectedly")??;
        Ok(())
    }
}

impl UpdateTarget {
    fn platform<'a>(
        &self,
        platforms: &'a std::collections::HashMap<String, PlatformRelease>,
    ) -> Result<&'a PlatformRelease> {
        let platform = match self {
            #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
            Self::Windows => platforms
                .get("windows-x86_64-nsis")
                .or_else(|| platforms.get("windows-x86_64"))
                .context("Update manifest has no Windows x86_64 package")?,
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxAppImage(_) => platforms
                .get("linux-x86_64-appimage")
                .or_else(|| platforms.get("linux-x86_64"))
                .context("Update manifest has no Linux AppImage package")?,
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxDeb => platforms
                .get("linux-x86_64-deb")
                .context("Update manifest has no Linux Debian package")?,
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxRpm => platforms
                .get("linux-x86_64-rpm")
                .context("Update manifest has no Linux RPM package")?,
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxUnrecognized => platforms
                .get("linux-x86_64-appimage")
                .or_else(|| platforms.get("linux-x86_64"))
                .context("Update manifest has no Linux package")?,
            #[cfg(not(any(
                all(target_os = "windows", target_arch = "x86_64"),
                all(target_os = "linux", target_arch = "x86_64")
            )))]
            Self::Unsupported => bail!("Automatic updates are unavailable for this platform"),
        };
        Ok(platform)
    }

    fn file_extension(&self) -> Option<&'static str> {
        match self {
            #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
            Self::Windows => Some("exe"),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxAppImage(_) => Some("AppImage"),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxDeb => Some("deb"),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxRpm => Some("rpm"),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            Self::LinuxUnrecognized => None,
            #[cfg(not(any(
                all(target_os = "windows", target_arch = "x86_64"),
                all(target_os = "linux", target_arch = "x86_64")
            )))]
            Self::Unsupported => None,
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
async fn detect_target() -> Result<UpdateTarget> {
    Ok(UpdateTarget::Windows)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
async fn detect_target() -> Result<UpdateTarget> {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        let appimage = PathBuf::from(appimage);
        if appimage
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("AppImage"))
        {
            return Ok(UpdateTarget::LinuxAppImage(appimage));
        }
    }

    let executable =
        std::env::current_exe().context("Could not locate the LocalCraft executable")?;
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

async fn install_package(update: &AvailableUpdate, package_path: &Path) -> Result<()> {
    match &update.target {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        UpdateTarget::Windows => {
            let mut installer = Command::new(package_path)
                .args(["/P", "/UPDATE", "/R"])
                .spawn()
                .context("Could not start the LocalCraft installer")?;
            let installer_for_cleanup = package_path.to_owned();
            tokio::spawn(async move {
                let _ = installer.wait().await;
                let _ = fs::remove_file(installer_for_cleanup).await;
            });
            Ok(())
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        UpdateTarget::LinuxAppImage(appimage) => {
            let install_result = replace_appimage(package_path, appimage).await;
            install_result.with_context(|| {
                format!(
                    "Could not update the AppImage. Download it manually from {}",
                    update.release_page
                )
            })
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        UpdateTarget::LinuxDeb => {
            let install_result =
                run_elevated_installer("apt-get", &["install", "--yes"], package_path).await;
            install_result.with_context(|| {
                format!(
                    "Could not install the Debian update. Download it manually from {}",
                    update.release_page
                )
            })?;
            restart_application().await.with_context(|| {
                format!(
                    "The update was installed. Start LocalCraft again or download it from {}",
                    update.release_page
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
                    "No supported RPM package manager is installed. Download the update from {}",
                    update.release_page
                );
            };
            let install_result = run_elevated_installer(manager, arguments, package_path).await;
            install_result.with_context(|| {
                format!(
                    "Could not install the RPM update. Download it manually from {}",
                    update.release_page
                )
            })?;
            restart_application().await.with_context(|| {
                format!(
                    "The update was installed. Start LocalCraft again or download it from {}",
                    update.release_page
                )
            })
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        UpdateTarget::LinuxUnrecognized => bail!(
            "Automatic updates are unavailable for this Linux installation. Download the update from {}",
            update.release_page
        ),
        #[cfg(not(any(
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "x86_64")
        )))]
        UpdateTarget::Unsupported => bail!("Automatic updates are unavailable for this platform"),
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

fn verify_signature(package_path: &Path, signature_text: &str, version: &str) -> Result<()> {
    let public_key_bytes = STANDARD
        .decode(PUBLIC_KEY)
        .context("Could not decode the updater public key")?;
    let public_key_text =
        String::from_utf8(public_key_bytes).context("Updater public key is not valid UTF-8")?;
    let public_key =
        PublicKey::decode(&public_key_text).context("Could not parse the updater public key")?;
    let signature = Signature::decode(signature_text)
        .context("Could not parse the update package signature")?;
    if !signature
        .trusted_comment()
        .split_whitespace()
        .any(|field| field == format!("app-version:{version}"))
    {
        bail!("Update signature does not match the manifest version");
    }
    let mut verifier = public_key
        .verify_stream(&signature)
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
