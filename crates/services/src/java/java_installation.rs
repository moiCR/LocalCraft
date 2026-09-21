use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct JavaInstallation {
    pub major_version: u8,
    pub vendor: String,
    binary_path: PathBuf,
}

impl JavaInstallation {
    pub fn new(major_version: u8, vendor: String, binary_path: PathBuf) -> Self {
        Self {
            major_version,
            vendor,
            binary_path,
        }
    }

    pub fn binary_path(&self) -> &PathBuf {
        &self.binary_path
    }
}
