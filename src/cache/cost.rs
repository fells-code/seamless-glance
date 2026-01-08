use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::BudgetInfo;

const CACHE_TTL: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Debug, Serialize, Deserialize)]
pub struct CostCache {
    pub fetched_at: DateTime<Utc>,
    pub budget: BudgetInfo,
    pub monthly_costs: Vec<f64>,
    pub service_costs: Vec<(String, f64)>,
}

fn cache_path() -> PathBuf {
    dirs::home_dir()
        .expect("home dir")
        .join(".seamless-glance")
        .join("cache")
        .join("cost.json")
}

pub fn load_if_fresh() -> Option<CostCache> {
    let path = cache_path();
    let data = fs::read_to_string(path).ok()?;
    let cache: CostCache = serde_json::from_str(&data).ok()?;

    let age = SystemTime::now()
        .duration_since(cache.fetched_at.into())
        .ok()?;

    if age <= CACHE_TTL {
        Some(cache)
    } else {
        None
    }
}

pub fn save(cache: &CostCache) {
    let path = cache_path();

    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }

    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(path, json);
    }
}
