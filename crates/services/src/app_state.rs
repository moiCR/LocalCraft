use anyhow::Result;
use ui::theme::manager::ThemeManager;

use crate::{instance::InstancesService, java::JavaService, software::SoftwareService};

pub struct AppState {
    pub theme_manager: ThemeManager,
    pub instance_service: InstancesService,
    pub java_service: JavaService,
    pub software_service: SoftwareService,
}

impl gpui::Global for AppState {}

impl AppState {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            theme_manager: ThemeManager::default(),
            instance_service: InstancesService::new().await?,
            java_service: JavaService::new(),
            software_service: SoftwareService::new(),
        })
    }
}
