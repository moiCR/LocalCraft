use std::sync::Arc;

use anyhow::{Context, Result};
use ui::theme::manager::ThemeManager;

use crate::{
    instance::InstancesService, java::JavaService, playit::PlayitService, software::SoftwareService,
};

pub struct AppState {
    pub theme_manager: ThemeManager,
    pub instance_service: InstancesService,
    pub java_service: Arc<JavaService>,
    pub background_runtime: tokio::runtime::Handle,
    pub software_service: SoftwareService,
    pub playit_service: Arc<PlayitService>,
}

impl gpui::Global for AppState {}

impl AppState {
    pub async fn new() -> Result<Self> {
        let background_runtime = tokio::runtime::Handle::try_current()
            .context("AppState must be initialized on the Tokio runtime")?;
        let java_service = Arc::new(JavaService::new());
        java_service.load().await?;
        Ok(Self {
            theme_manager: ThemeManager::default(),
            instance_service: InstancesService::new().await?,
            java_service,
            background_runtime,
            software_service: SoftwareService::new(),
            playit_service: Arc::new(PlayitService::new()),
        })
    }
}
