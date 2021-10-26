use anyhow::Error;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // in ms
    pub packet_duration: u64,
    pub max_queued_packets: usize,
    // GDrive folder ID
    pub folder_id: String,
    // How many files to use for caching
    pub file_count: u16,
    pub port: u16,
}

impl Settings {
    pub async fn load(path: &str) -> Result<Settings, Error> {
        let settings: Settings = serde_json::from_str(&tokio::fs::read_to_string(path).await?)?;
        Ok(settings)
    }
}