use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct GlanceConfig {
    pub region: Option<String>,
    pub profile: Option<String>,
}

impl Default for GlanceConfig {
    fn default() -> Self {
        Self {
            region: None,
            profile: None,
        }
    }
}

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".seamless-glance")
        .join("config.json")
}

pub fn load_config() -> GlanceConfig {
    let path = config_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &GlanceConfig) {
    let path = config_path();

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let _ = fs::write(path, serde_json::to_string_pretty(cfg).unwrap());
}
