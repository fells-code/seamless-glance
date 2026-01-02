use std::path::PathBuf;

use crate::license::License;

pub fn license_path() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join(".seamless-glance")
        .join("license.json")
}

pub fn load_license() -> Result<License, String> {
    let contents = std::fs::read_to_string(license_path()).map_err(|_| "License file not found")?;

    let contents = contents.trim_end();

    serde_json::from_str(contents).map_err(|e| format!("Invalid license format: {}", e))
}
