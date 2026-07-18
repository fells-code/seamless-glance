use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Discover the AWS profile names available on this machine by reading the
/// shared config and credentials files, honoring the standard `AWS_CONFIG_FILE`
/// and `AWS_SHARED_CREDENTIALS_FILE` overrides. Returns a sorted, de-duplicated
/// list. The AWS SDK resolves credentials itself; this is only used to populate
/// the in-app profile picker.
pub fn discover_profiles() -> Vec<String> {
    let mut names = BTreeSet::new();

    if let Some(path) = config_file_path() {
        collect_sections(&path, true, &mut names);
    }

    if let Some(path) = credentials_file_path() {
        collect_sections(&path, false, &mut names);
    }

    names.into_iter().collect()
}

fn config_file_path() -> Option<PathBuf> {
    env_path("AWS_CONFIG_FILE").or_else(|| aws_dir_file("config"))
}

fn credentials_file_path() -> Option<PathBuf> {
    env_path("AWS_SHARED_CREDENTIALS_FILE").or_else(|| aws_dir_file("credentials"))
}

fn env_path(var: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(value) if !value.is_empty() => Some(PathBuf::from(value)),
        _ => None,
    }
}

fn aws_dir_file(name: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".aws").join(name))
}

/// Parse INI-style section headers. In the config file profiles are written as
/// `[profile name]` (with a bare `[default]`), while the credentials file uses
/// `[name]`. Non-profile config sections (`sso-session`, `services`) are ignored.
fn collect_sections(path: &Path, is_config: bool, names: &mut BTreeSet<String>) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };

    for line in content.lines() {
        let trimmed = line.trim();

        let Some(section) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        else {
            continue;
        };

        let section = section.trim();

        let name = if is_config {
            if section.starts_with("sso-session") || section.starts_with("services") {
                continue;
            }
            section.strip_prefix("profile ").unwrap_or(section).trim()
        } else {
            section
        };

        if !name.is_empty() {
            names.insert(name.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(contents: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("seamless-glance-profiles-{}", contents.len()));
        let mut file = std::fs::File::create(&path).expect("create temp file");
        file.write_all(contents.as_bytes())
            .expect("write temp file");
        path
    }

    #[test]
    fn config_file_strips_profile_prefix_and_skips_non_profiles() {
        let path = temp_file(
            "[default]\nregion = us-east-1\n\n[profile client-a]\n[profile client-b]\n\n[sso-session corp]\n[services shared]\n",
        );

        let mut names = BTreeSet::new();
        collect_sections(&path, true, &mut names);

        let collected: Vec<_> = names.into_iter().collect();
        assert_eq!(collected, vec!["client-a", "client-b", "default"]);
    }

    #[test]
    fn credentials_file_uses_bare_section_names() {
        let path = temp_file("[default]\naws_access_key_id = AKIA\n\n[client-a]\n");

        let mut names = BTreeSet::new();
        collect_sections(&path, false, &mut names);

        let collected: Vec<_> = names.into_iter().collect();
        assert_eq!(collected, vec!["client-a", "default"]);
    }
}
