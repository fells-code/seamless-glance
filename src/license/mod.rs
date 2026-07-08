use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseType {
    Trial,
    Paid,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    pub key: String,
    pub r#type: LicenseType,
    pub email: String,
    pub issued_at: String,
    pub expires_at: String,
    pub signature: String,
}

impl License {
    pub fn is_paid(&self) -> bool {
        matches!(self.r#type, LicenseType::Paid)
    }

    pub fn trial_days_remaining(&self) -> Option<i64> {
        if self.is_paid() {
            return None;
        }

        let today = Utc::now().date_naive();
        let expires = NaiveDate::parse_from_str(&self.expires_at, "%Y-%m-%d").ok()?;

        Some((expires - today).num_days().max(0))
    }
}

pub mod ensure_license;
pub mod load;
pub mod public_key;
pub mod status;
pub mod verify;
