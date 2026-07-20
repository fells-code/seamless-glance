//! Persisted user preferences.
//!
//! A config that cannot be read is never quietly replaced. Preferences are the
//! only thing in this file and there is no way to recover them once
//! overwritten, so an unreadable file is preserved and reported rather than
//! treated as absent.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::theme::ThemeName;

/// Schema version of the config this build writes.
///
/// Bump when a change cannot be expressed by adding a `#[serde(default)]`
/// field, and give [`migrate`] an arm for the older shape.
pub const SCHEMA_VERSION: u32 = 1;

/// Version assumed for a config written before the field existed.
const PRE_VERSIONED: u32 = 0;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GlanceConfig {
    #[serde(default)]
    pub schema_version: u32,
    pub region: Option<String>,
    pub profile: Option<String>,
    pub theme: Option<String>,
}

impl Default for GlanceConfig {
    fn default() -> Self {
        GlanceConfig {
            schema_version: SCHEMA_VERSION,
            region: None,
            profile: None,
            theme: None,
        }
    }
}

/// What happened when the config was read.
#[derive(Debug)]
pub enum ConfigLoad {
    /// Read and usable. `warning` is set when a value was rejected but the rest
    /// of the config was kept.
    Loaded {
        config: GlanceConfig,
        warning: Option<String>,
    },
    /// No config file yet. Normal on first run, and not worth reporting.
    Missing,
    /// The file exists but could not be used. The original has been preserved
    /// at `backup` and must not be overwritten with defaults.
    Unreadable {
        reason: String,
        backup: Option<PathBuf>,
    },
}

impl ConfigLoad {
    /// The config to run with, defaulting when there is nothing usable.
    pub fn config(self) -> GlanceConfig {
        match self {
            ConfigLoad::Loaded { config, .. } => config,
            ConfigLoad::Missing | ConfigLoad::Unreadable { .. } => GlanceConfig::default(),
        }
    }

    /// Message to show the operator, if anything needs saying.
    pub fn warning(&self) -> Option<String> {
        match self {
            ConfigLoad::Loaded { warning, .. } => warning.clone(),
            ConfigLoad::Missing => None,
            ConfigLoad::Unreadable { reason, backup } => Some(match backup {
                Some(path) => format!(
                    "Could not read config ({reason}). The previous file was kept at {} and defaults are in use.",
                    path.display()
                ),
                None => format!(
                    "Could not read config ({reason}) and it could not be moved aside. Defaults are in use and preferences will not be saved."
                ),
            }),
        }
    }

    /// Whether saving would destroy something that could not be read.
    pub fn blocks_saving(&self) -> bool {
        matches!(self, ConfigLoad::Unreadable { backup: None, .. })
    }
}

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".seamless-glance")
        .join("config.json")
}

/// Move an unreadable config aside under a unique name.
///
/// Timestamped rather than a fixed `.bak` so a second failure cannot overwrite
/// the backup taken for the first.
fn preserve(path: &Path) -> Option<PathBuf> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let backup = path.with_extension(format!("invalid-{stamp}.json"));

    fs::rename(path, &backup).ok()?;
    Some(backup)
}

/// Bring an older config forward. A version newer than this build's is left
/// alone: it may carry fields this build does not know about, and rewriting it
/// would drop them.
fn migrate(mut config: GlanceConfig) -> GlanceConfig {
    if config.schema_version == PRE_VERSIONED {
        config.schema_version = SCHEMA_VERSION;
    }

    config
}

/// Drop values that are not usable, keeping the rest of the config.
///
/// A theme that no longer exists (renamed, or written by a newer build) should
/// cost the operator their theme, not their region and profile.
fn validate(mut config: GlanceConfig) -> (GlanceConfig, Option<String>) {
    let mut warning = None;

    if let Some(theme) = config.theme.clone() {
        if ThemeName::from_str(&theme).is_none() {
            warning = Some(format!(
                "Ignoring unknown theme {theme:?} in config; using the default theme."
            ));
            config.theme = None;
        }
    }

    (config, warning)
}

pub fn load_config() -> ConfigLoad {
    let path = config_path();

    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return ConfigLoad::Missing,
        Err(err) => {
            return ConfigLoad::Unreadable {
                reason: err.to_string(),
                // Unreadable rather than malformed, so leave the file in place:
                // renaming it would likely fail for the same reason.
                backup: None,
            };
        }
    };

    match serde_json::from_str::<GlanceConfig>(&raw) {
        Ok(config) => {
            let (config, warning) = validate(migrate(config));
            ConfigLoad::Loaded { config, warning }
        }
        Err(err) => ConfigLoad::Unreadable {
            reason: err.to_string(),
            backup: preserve(&path),
        },
    }
}

pub fn save_config(config: &GlanceConfig) -> Result<(), String> {
    let path = config_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let json = serde_json::to_string_pretty(config).map_err(|err| err.to_string())?;

    fs::write(&path, json).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_theme(theme: Option<&str>) -> GlanceConfig {
        GlanceConfig {
            schema_version: SCHEMA_VERSION,
            region: Some("us-east-1".into()),
            profile: Some("prod".into()),
            theme: theme.map(str::to_string),
        }
    }

    #[test]
    fn a_config_written_before_versioning_migrates_forward() {
        let old: GlanceConfig =
            serde_json::from_str(r#"{"region":"us-east-1","profile":null,"theme":"autumn"}"#)
                .expect("the older shape still parses");
        assert_eq!(old.schema_version, PRE_VERSIONED);

        let migrated = migrate(old);
        assert_eq!(migrated.schema_version, SCHEMA_VERSION);
        assert_eq!(migrated.region.as_deref(), Some("us-east-1"));
        assert_eq!(migrated.theme.as_deref(), Some("autumn"));
    }

    /// An unusable theme must not cost the operator their region and profile.
    #[test]
    fn an_unknown_theme_is_dropped_without_losing_other_settings() {
        let (config, warning) = validate(config_with_theme(Some("chartreuse")));

        assert_eq!(config.theme, None);
        assert_eq!(config.region.as_deref(), Some("us-east-1"));
        assert_eq!(config.profile.as_deref(), Some("prod"));
        assert!(warning
            .expect("warns about the theme")
            .contains("chartreuse"));
    }

    #[test]
    fn a_known_theme_is_kept_without_warning() {
        let (config, warning) = validate(config_with_theme(Some("winter")));

        assert_eq!(config.theme.as_deref(), Some("winter"));
        assert!(warning.is_none());
    }

    #[test]
    fn an_absent_theme_is_not_a_warning() {
        let (config, warning) = validate(config_with_theme(None));

        assert_eq!(config.theme, None);
        assert!(warning.is_none());
    }

    /// The whole point: a malformed file must never read as "no config", which
    /// is what let the next save overwrite it with defaults.
    #[test]
    fn an_unreadable_config_is_distinct_from_a_missing_one() {
        let missing = ConfigLoad::Missing;
        assert!(missing.warning().is_none());
        assert!(!missing.blocks_saving());

        let unreadable = ConfigLoad::Unreadable {
            reason: "expected value at line 1".into(),
            backup: Some(PathBuf::from("/tmp/config.invalid-1.json")),
        };
        let warning = unreadable.warning().expect("reports the failure");
        assert!(warning.contains("config.invalid-1.json"));
        // Backed up, so overwriting the live file destroys nothing.
        assert!(!unreadable.blocks_saving());
    }

    /// If the original could not be moved aside, saving over it would destroy
    /// preferences that might still be recoverable by hand.
    #[test]
    fn saving_is_blocked_when_the_original_could_not_be_preserved() {
        let unpreserved = ConfigLoad::Unreadable {
            reason: "permission denied".into(),
            backup: None,
        };

        assert!(unpreserved.blocks_saving());
        assert!(unpreserved.warning().unwrap().contains("will not be saved"));
    }

    #[test]
    fn an_unusable_config_still_yields_defaults_to_run_with() {
        let config = ConfigLoad::Unreadable {
            reason: "bad".into(),
            backup: None,
        }
        .config();

        assert_eq!(config, GlanceConfig::default());
        assert_eq!(config.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn a_saved_config_round_trips() {
        let original = config_with_theme(Some("winter"));
        let json = serde_json::to_string_pretty(&original).expect("serializes");
        let restored: GlanceConfig = serde_json::from_str(&json).expect("deserializes");

        assert_eq!(restored, original);
        assert_eq!(restored.schema_version, SCHEMA_VERSION);
    }
}
