use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

use crate::{
    aws::clients::AwsClients,
    models::{
        describable::{shell_quote, DescribableResource},
        tags::Tags,
    },
};

/// Rollup of a repository's images, from one `DescribeImages` listing.
#[derive(Debug, Clone, Default)]
pub struct EcrImageStats {
    pub count: usize,
    pub untagged: usize,
    pub total_bytes: i64,
    pub last_pushed: Option<DateTime<Utc>>,
    /// ECR only records pulls since it started tracking them and refreshes the
    /// timestamp at most daily, so `None` means "no recorded pull", not
    /// "never pulled".
    pub last_pulled: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct EcrRepositoryInfo {
    pub name: String,
    pub registry_id: String,
    /// `None` when the policy lookup failed, so an unreadable policy is never
    /// reported as a missing one.
    pub has_lifecycle_policy: Option<bool>,
    /// `None` when the image listing failed.
    pub images: Option<EcrImageStats>,
    pub tags: Tags,
}

impl EcrRepositoryInfo {
    pub const UNTAGGED_BUILDUP_THRESHOLD: usize = 20;
    pub const STALE_PUSH_DAYS: i64 = 180;
    pub const STALE_PULL_DAYS: i64 = 90;

    fn has_images(&self) -> bool {
        self.images.as_ref().is_some_and(|images| images.count > 0)
    }

    pub fn has_untagged_buildup(&self) -> bool {
        self.images
            .as_ref()
            .is_some_and(|images| images.untagged >= Self::UNTAGGED_BUILDUP_THRESHOLD)
    }

    /// Images are still stored, nothing has been pushed in a long while, and
    /// nothing has been pulled recently either, so the storage likely serves no
    /// running workload.
    pub fn is_stale(&self) -> bool {
        let Some(images) = self.images.as_ref().filter(|images| images.count > 0) else {
            return false;
        };
        let Some(last_pushed) = images.last_pushed else {
            return false;
        };

        let now = Utc::now();
        let push_is_old = last_pushed <= now - Duration::days(Self::STALE_PUSH_DAYS);
        let pull_is_old = images
            .last_pulled
            .is_none_or(|pulled| pulled <= now - Duration::days(Self::STALE_PULL_DAYS));

        push_is_old && pull_is_old
    }

    /// An empty repository accrues nothing, so it has nothing to expire.
    pub fn lacks_lifecycle_policy(&self) -> bool {
        self.has_lifecycle_policy == Some(false) && self.has_images()
    }

    pub fn review_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if self.has_untagged_buildup() {
            signals.push("untagged");
        }

        if self.is_stale() {
            signals.push("stale");
        }

        if self.lacks_lifecycle_policy() {
            signals.push("no-lifecycle");
        }

        signals
    }

    pub fn size_label(&self) -> String {
        match &self.images {
            Some(images) => format_bytes(images.total_bytes),
            None => "-".into(),
        }
    }
}

fn format_bytes(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

    let mut value = bytes.max(0) as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[async_trait]
impl DescribableResource for EcrRepositoryInfo {
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients
            .ecr
            .describe_repositories()
            .repository_names(&self.name)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://{region}.console.aws.amazon.com/ecr/repositories/private/{}/{}?region={region}",
            self.registry_id, self.name
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws ecr describe-images --repository-name {} --region {}",
            shell_quote(&self.name),
            shell_quote(region)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(
        images: Option<EcrImageStats>,
        has_lifecycle_policy: Option<bool>,
    ) -> EcrRepositoryInfo {
        EcrRepositoryInfo {
            name: "web".into(),
            registry_id: "123456789012".into(),
            has_lifecycle_policy,
            images,
            tags: Tags::empty(),
        }
    }

    fn days_ago(days: i64) -> Option<DateTime<Utc>> {
        Some(Utc::now() - Duration::days(days))
    }

    fn stats(count: usize, pushed: i64, pulled: Option<i64>) -> EcrImageStats {
        EcrImageStats {
            count,
            last_pushed: days_ago(pushed),
            last_pulled: pulled.and_then(days_ago),
            ..EcrImageStats::default()
        }
    }

    #[test]
    fn untagged_buildup_threshold_is_inclusive() {
        let at = EcrImageStats {
            untagged: EcrRepositoryInfo::UNTAGGED_BUILDUP_THRESHOLD,
            ..EcrImageStats::default()
        };
        let below = EcrImageStats {
            untagged: EcrRepositoryInfo::UNTAGGED_BUILDUP_THRESHOLD - 1,
            ..EcrImageStats::default()
        };

        assert!(repo(Some(at), Some(true)).has_untagged_buildup());
        assert!(!repo(Some(below), Some(true)).has_untagged_buildup());
    }

    #[test]
    fn an_old_push_with_no_recent_pull_is_stale() {
        let old = EcrRepositoryInfo::STALE_PUSH_DAYS + 1;

        assert!(repo(Some(stats(3, old, None)), Some(true)).is_stale());
        assert!(repo(Some(stats(3, old, Some(365))), Some(true)).is_stale());
    }

    /// A repository nobody pushes to can still back a running service, and a
    /// recent pull is the evidence that it does.
    #[test]
    fn a_recent_pull_keeps_an_old_repository_from_being_stale() {
        let old = EcrRepositoryInfo::STALE_PUSH_DAYS + 1;

        assert!(!repo(Some(stats(3, old, Some(1))), Some(true)).is_stale());
    }

    #[test]
    fn a_recent_push_is_not_stale() {
        assert!(!repo(Some(stats(3, 1, None)), Some(true)).is_stale());
    }

    #[test]
    fn an_empty_repository_is_not_stale_or_missing_a_lifecycle_policy() {
        let empty = repo(Some(EcrImageStats::default()), Some(false));

        assert!(!empty.is_stale());
        assert!(!empty.lacks_lifecycle_policy());
    }

    /// A failed lookup is not evidence of anything, so it must not produce a
    /// signal that reads as a real gap.
    #[test]
    fn unreadable_data_raises_no_signals() {
        let unreadable = repo(None, None);

        assert!(unreadable.review_signals().is_empty());
        assert_eq!(unreadable.size_label(), "-");

        let policy_unknown = repo(Some(stats(3, 1, Some(1))), None);
        assert!(!policy_unknown.lacks_lifecycle_policy());
    }

    #[test]
    fn a_repository_with_images_and_no_policy_lacks_a_lifecycle_policy() {
        assert!(repo(Some(stats(3, 1, Some(1))), Some(false)).lacks_lifecycle_policy());
        assert!(!repo(Some(stats(3, 1, Some(1))), Some(true)).lacks_lifecycle_policy());
    }

    #[test]
    fn sizes_are_shown_in_the_largest_whole_unit() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
