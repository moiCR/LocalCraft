use std::{collections::HashMap, sync::RwLock};

use crate::java::java_installation::JavaInstallation;
pub mod java_installation;

#[derive(Default)]
pub struct JavaService {
    installations: RwLock<HashMap<String, JavaInstallation>>,
}

impl JavaService {
    pub fn new() -> Self {
        Self {
            installations: RwLock::new(HashMap::new()),
        }
    }

    pub fn installations(&self) -> &RwLock<HashMap<String, JavaInstallation>> {
        &self.installations
    }

    pub fn download_java(&self, _version: &str) -> Result<(), Box<dyn std::error::Error>> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Java downloads are not implemented yet",
        )
        .into())
    }
}
