use chrono::{Duration, NaiveDate, Utc};
use std::fs;

use crate::license::{
    load::{license_dir, license_path},
    License, LicenseType,
};

fn today_ymd() -> NaiveDate {
    Utc::now().date_naive()
}

fn fmt(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Create a 30-day trial license and write it to ~/.seamless-glance/license.json
fn create_trial_license() -> Result<License, String> {
    let start = today_ymd();
    let expires = start + Duration::days(365);

    let lic = License {
        key: "SG-TRIAL-LOCAL".to_string(),
        r#type: LicenseType::Trial,
        email: "trail@seamlessglance.com".to_string(),
        issued_at: fmt(start),
        expires_at: fmt(expires),
        signature: "".to_string(),
    };

    fs::create_dir_all(license_dir()).map_err(|e| format!("Failed to create license dir: {e}"))?;
    let json = serde_json::to_string_pretty(&lic)
        .map_err(|e| format!("Failed to serialize trial license: {e}"))?;
    fs::write(license_path(), json).map_err(|e| format!("Failed to write trial license: {e}"))?;

    Ok(lic)
}

/// Load license if present; otherwise create trial license.
/// Returns the license object for validation/use.
pub fn ensure_license_present() -> Result<License, String> {
    let path = license_path();

    if !path.exists() {
        return create_trial_license();
    }

    let contents = fs::read_to_string(&path).map_err(|_| "License file not found".to_string())?;

    serde_json::from_str::<License>(contents.trim_end())
        .map_err(|e| format!("Invalid license format: {e}"))
}
