use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{BudgetInfo, ServiceCostInsight};

const CACHE_TTL: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Debug, Serialize, Deserialize)]
pub struct CostCache {
    pub fetched_at: DateTime<Utc>,
    // Scope the cached financial data to the account it was fetched under so
    // profile A's spend can never be shown while profile B is active.
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub region: String,
    pub budget: BudgetInfo,
    pub monthly_costs: Vec<f64>,
    pub service_costs: Vec<(String, f64)>,
    #[serde(default)]
    pub service_cost_insights: Vec<ServiceCostInsight>,
}

fn sanitize(component: &str) -> String {
    component
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn cache_file_name(profile: Option<&str>, region: &str) -> String {
    let profile = sanitize(profile.unwrap_or("default"));
    let region = sanitize(region);
    format!("cost-{profile}-{region}.json")
}

fn cache_path(profile: Option<&str>, region: &str) -> PathBuf {
    dirs::home_dir()
        .expect("home dir")
        .join(".seamless-glance")
        .join("cache")
        .join(cache_file_name(profile, region))
}

pub fn load_if_fresh(profile: Option<&str>, region: &str) -> Option<CostCache> {
    let path = cache_path(profile, region);
    let data = fs::read_to_string(path).ok()?;
    let cache: CostCache = serde_json::from_str(&data).ok()?;

    // Defense in depth against a sanitized-filename collision: only accept a
    // cache whose stored scope matches the requested profile and region.
    if cache.profile.as_deref() != profile || cache.region != region {
        return None;
    }

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
    let path = cache_path(cache.profile.as_deref(), &cache.region);

    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }

    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_file_name_is_scoped_and_sanitized() {
        assert_eq!(
            cache_file_name(Some("prod"), "us-east-1"),
            "cost-prod-us-east-1.json"
        );
        assert_eq!(cache_file_name(None, "global"), "cost-default-global.json");
        assert_eq!(
            cache_file_name(Some("team/prod"), "us-east-1"),
            "cost-team_prod-us-east-1.json"
        );
    }

    #[test]
    fn different_profiles_map_to_different_files() {
        assert_ne!(
            cache_file_name(Some("a"), "us-east-1"),
            cache_file_name(Some("b"), "us-east-1"),
            "distinct profiles must not share a cost cache file"
        );
    }
}
