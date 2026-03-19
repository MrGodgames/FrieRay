use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

/// In-memory log buffer for the UI
#[derive(Clone)]
pub struct LogBuffer {
    entries: Arc<Mutex<Vec<LogEntry>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub message: String,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add(&self, level: &str, message: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let hours = (secs / 3600) % 24;
        let mins = (secs / 60) % 60;
        let s = secs % 60;
        let time = format!("{:02}:{:02}:{:02}", hours, mins, s);

        let entry = LogEntry {
            time,
            level: level.to_string(),
            message: message.to_string(),
        };

        // Also output to env_logger
        match level {
            "error" => log::error!("[UI] {}", message),
            "warn" => log::warn!("[UI] {}", message),
            "success" => log::info!("[UI] ✓ {}", message),
            _ => log::info!("[UI] {}", message),
        }

        let mut entries = self.entries.lock().await;
        entries.push(entry);

        // Keep last 500 entries
        if entries.len() > 500 {
            let drain_end = entries.len() - 500;
            entries.drain(0..drain_end);
        }
    }

    pub async fn get_all(&self) -> Vec<LogEntry> {
        self.entries.lock().await.clone()
    }

    pub async fn clear(&self) {
        self.entries.lock().await.clear();
    }
}
