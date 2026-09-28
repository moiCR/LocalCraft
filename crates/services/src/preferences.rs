use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt, sync::Mutex};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPreferences {
    pub dark_theme: bool,
    pub default_software: String,
    pub default_ram_mib: u32,
    pub default_java_major: Option<u8>,
    pub console_timestamps: bool,
    pub console_auto_scroll: bool,
    pub sidebar_collapsed: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            dark_theme: true,
            default_software: "Paper".into(),
            default_ram_mib: 2048,
            default_java_major: None,
            console_timestamps: false,
            console_auto_scroll: true,
            sidebar_collapsed: false,
        }
    }
}

impl AppPreferences {
    fn normalize(&mut self) {
        if !["Paper", "Purpur", "Fabric", "Forge", "Vanilla", "Pumpkin"]
            .contains(&self.default_software.as_str())
        {
            self.default_software = "Paper".into();
        }
        if !(512..=262_144).contains(&self.default_ram_mib) {
            self.default_ram_mib = 2048;
        }
        if self
            .default_java_major
            .is_some_and(|version| ![8, 17, 21, 25].contains(&version))
        {
            self.default_java_major = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pumpkin_is_a_supported_default_software_choice() {
        let mut preferences = AppPreferences {
            default_software: "Pumpkin".into(),
            ..AppPreferences::default()
        };
        preferences.normalize();
        assert_eq!(preferences.default_software, "Pumpkin");
    }
}

#[derive(Clone, Default)]
pub struct PreferencesStore {
    write_lock: Arc<Mutex<()>>,
    saved_revision: Arc<AtomicU64>,
}

impl PreferencesStore {
    pub async fn load() -> (AppPreferences, Self) {
        let preferences = match Self::path() {
            Ok(path) => match fs::read(&path).await {
                Ok(bytes) => match serde_json::from_slice::<AppPreferences>(&bytes) {
                    Ok(mut preferences) => {
                        preferences.normalize();
                        preferences
                    }
                    Err(error) => {
                        eprintln!("Could not load application preferences: {error}");
                        AppPreferences::default()
                    }
                },
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    AppPreferences::default()
                }
                Err(error) => {
                    eprintln!("Could not read application preferences: {error}");
                    AppPreferences::default()
                }
            },
            Err(error) => {
                eprintln!("Could not locate application preferences: {error:#}");
                AppPreferences::default()
            }
        };
        (preferences, Self::default())
    }

    pub async fn save(&self, preferences: AppPreferences, revision: u64) -> Result<()> {
        let _guard = self.write_lock.lock().await;
        if revision <= self.saved_revision.load(Ordering::Acquire) {
            return Ok(());
        }

        let path = Self::path()?;
        let parent = path
            .parent()
            .context("Preferences path has no parent directory")?;
        fs::create_dir_all(parent)
            .await
            .context("Could not create preferences directory")?;
        let bytes = serde_json::to_vec_pretty(&preferences)
            .context("Could not serialize application preferences")?;
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let result = async {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .await?;
            file.write_all(&bytes).await?;
            file.sync_all().await?;
            drop(file);
            fs::rename(&temporary, &path).await
        }
        .await;
        if result.is_err() {
            let _ = fs::remove_file(&temporary).await;
        }
        result.context("Could not atomically save application preferences")?;
        self.saved_revision.store(revision, Ordering::Release);
        Ok(())
    }

    fn path() -> Result<PathBuf> {
        Ok(dirs::data_dir()
            .context("Could not find data directory")?
            .join("LocalCraft")
            .join("settings.json"))
    }
}
