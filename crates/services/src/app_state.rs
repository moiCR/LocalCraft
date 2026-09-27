use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use ui::theme::manager::ThemeManager;

use crate::{
    instance::InstancesService,
    java::JavaService,
    playit::PlayitService,
    preferences::{AppPreferences, PreferencesStore},
    software::SoftwareService,
};

pub struct AppState {
    pub theme_manager: ThemeManager,
    pub instance_service: InstancesService,
    pub java_service: Arc<JavaService>,
    pub background_runtime: tokio::runtime::Handle,
    pub software_service: SoftwareService,
    pub playit_service: Arc<PlayitService>,
    pub preferences: AppPreferences,
    pub preferences_store: PreferencesStore,
    pub preferences_revision: AtomicU64,
}

impl gpui::Global for AppState {}

impl AppState {
    pub async fn new() -> Result<Self> {
        let background_runtime = tokio::runtime::Handle::try_current()
            .context("AppState must be initialized on the Tokio runtime")?;
        let (preferences, preferences_store) = PreferencesStore::load().await;
        let java_service = Arc::new(JavaService::new());
        java_service.load().await?;
        let mut theme_manager = ThemeManager::default();
        theme_manager.set_appearance(if preferences.dark_theme {
            ui::theme::Appearance::Dark
        } else {
            ui::theme::Appearance::Light
        });
        Ok(Self {
            theme_manager,
            instance_service: InstancesService::new().await?,
            java_service,
            background_runtime,
            software_service: SoftwareService::new(),
            playit_service: Arc::new(PlayitService::new()),
            preferences,
            preferences_store,
            preferences_revision: AtomicU64::new(0),
        })
    }

    pub fn save_preferences(&self) {
        let revision = self
            .preferences_revision
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        let preferences = self.preferences.clone();
        let store = self.preferences_store.clone();
        self.background_runtime.spawn(async move {
            if let Err(error) = store.save(preferences, revision).await {
                eprintln!("Could not save application preferences: {error:#}");
            }
        });
    }
}
