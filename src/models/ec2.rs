use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::describable::{shell_quote, DescribableResource},
    models::tags::Tags,
};

#[derive(Debug, Clone)]
pub struct Ec2InstanceInfo {
    pub id: String,
    pub tags: Tags,
    pub avg_cpu_utilization: Option<f64>,
    pub instance_type: String,
    pub state: String,
    pub region: String,
    pub az: String,
    pub private_ip: Option<String>,
    pub public_ip: Option<String>,
    pub key_name: Option<String>,
}

impl Ec2InstanceInfo {
    pub const PRODUCTION_NAME_HINTS: [&str; 7] = [
        "prod",
        "production",
        "live",
        "critical",
        "primary",
        "main",
        "customer",
    ];
    pub const LOW_CPU_THRESHOLD_PERCENT: f64 = 5.0;
    pub const LOW_CPU_LOOKBACK_DAYS: i64 = 7;
    pub const LOW_CPU_PERIOD_SECONDS: i32 = 3600;

    pub fn is_stopped(&self) -> bool {
        self.state == "stopped"
    }

    pub fn is_running(&self) -> bool {
        self.state == "running"
    }

    pub fn has_public_ip(&self) -> bool {
        self.public_ip.is_some()
    }

    /// Tags this view treats as required for ownership attribution.
    pub const REQUIRED_TAGS: [&'static str; 3] = ["Name", "Owner", "Environment"];

    pub fn name(&self) -> Option<&str> {
        self.tags.value("Name")
    }

    pub fn owner(&self) -> Option<&str> {
        self.tags.value("Owner")
    }

    pub fn environment(&self) -> Option<&str> {
        self.tags.value("Environment")
    }

    /// How this instance is identified in lists and findings: its `Name` tag
    /// when it has one, otherwise its instance id.
    pub fn label(&self) -> String {
        self.name()
            .map(str::to_string)
            .unwrap_or_else(|| self.id.clone())
    }

    pub fn has_production_like_name(&self) -> bool {
        let Some(name) = self.name() else {
            return false;
        };

        let normalized = name.to_ascii_lowercase();
        Self::PRODUCTION_NAME_HINTS
            .iter()
            .any(|hint| normalized.contains(hint))
    }

    pub fn needs_stopped_review(&self) -> bool {
        self.is_stopped() && (self.has_public_ip() || self.has_production_like_name())
    }

    /// Required tags this instance lacks, or `None` when its tags could not be
    /// read and coverage cannot be judged.
    pub fn missing_required_tags(&self) -> Option<Vec<&'static str>> {
        self.tags.missing(&Self::REQUIRED_TAGS)
    }

    pub fn has_tag_coverage_gap(&self) -> bool {
        self.missing_required_tags()
            .is_some_and(|missing| !missing.is_empty())
    }

    pub fn has_sustained_low_cpu(&self) -> bool {
        self.is_running()
            && self
                .avg_cpu_utilization
                .is_some_and(|cpu| cpu < Self::LOW_CPU_THRESHOLD_PERCENT)
    }

    pub fn formatted_avg_cpu(&self) -> String {
        self.avg_cpu_utilization
            .map(|cpu| format!("{cpu:.1}%"))
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn review_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if self.has_public_ip() {
            signals.push("public-ip");
        }

        if self.has_production_like_name() {
            signals.push("prod-name");
        }

        if self.has_tag_coverage_gap() {
            signals.push("missing-tags");
        }

        if self.has_sustained_low_cpu() {
            signals.push("low-cpu");
        }

        signals
    }
}

#[async_trait]
impl DescribableResource for Ec2InstanceInfo {
    fn resource_name(&self) -> String {
        self.label()
    }

    fn action_region(&self) -> Option<&str> {
        Some(&self.region)
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients
            .ec2
            .describe_instances()
            .instance_ids(&self.id)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://console.aws.amazon.com/ec2/v2/home?region={region}#InstanceDetails:instanceId={}",
            self.id
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws ec2 describe-instances --instance-ids {} --region {}",
            shell_quote(&self.id),
            shell_quote(region)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(state: &str, tags: Tags) -> Ec2InstanceInfo {
        Ec2InstanceInfo {
            id: "i-0abc".into(),
            tags,
            avg_cpu_utilization: None,
            instance_type: "t3.medium".into(),
            state: state.into(),
            region: "us-east-1".into(),
            az: "us-east-1a".into(),
            private_ip: Some("10.0.0.5".into()),
            public_ip: None,
            key_name: None,
        }
    }

    fn tagged(name: &str) -> Tags {
        Tags::loaded([
            ("Name", name),
            ("Owner", "platform"),
            ("Environment", "dev"),
        ])
    }

    #[test]
    fn an_instance_falls_back_to_its_id_when_it_has_no_name() {
        assert_eq!(instance("running", Tags::empty()).label(), "i-0abc");
        assert_eq!(instance("running", tagged("web-1")).label(), "web-1");
    }

    #[test]
    fn a_production_like_name_is_matched_case_insensitively_anywhere_in_the_name() {
        for name in ["prod-api", "API-PROD", "customer-db", "Main-Gateway"] {
            assert!(
                instance("running", tagged(name)).has_production_like_name(),
                "{name} should read as production-like"
            );
        }

        for name in ["dev-api", "staging-web", "sandbox"] {
            assert!(
                !instance("running", tagged(name)).has_production_like_name(),
                "{name} should not read as production-like"
            );
        }
    }

    /// An untagged instance has no name to judge, which is not the same as
    /// having a name that looks non-production.
    #[test]
    fn an_unnamed_instance_is_not_production_like() {
        assert!(!instance("running", Tags::empty()).has_production_like_name());
        assert!(!instance("running", Tags::Unavailable).has_production_like_name());
    }

    #[test]
    fn a_stopped_instance_needs_review_only_with_a_reason() {
        let plain = instance("stopped", tagged("dev-box"));
        assert!(!plain.needs_stopped_review());

        let mut public = instance("stopped", tagged("dev-box"));
        public.public_ip = Some("54.1.2.3".into());
        assert!(public.needs_stopped_review());

        assert!(instance("stopped", tagged("prod-api")).needs_stopped_review());
    }

    /// The reasons only matter while the instance is stopped: a running
    /// instance with a public IP is normal.
    #[test]
    fn a_running_instance_never_needs_stopped_review() {
        let mut running = instance("running", tagged("prod-api"));
        running.public_ip = Some("54.1.2.3".into());

        assert!(!running.needs_stopped_review());
    }

    #[test]
    fn low_cpu_is_judged_only_on_running_instances_with_a_reading() {
        let below = Ec2InstanceInfo {
            avg_cpu_utilization: Some(Ec2InstanceInfo::LOW_CPU_THRESHOLD_PERCENT - 0.1),
            ..instance("running", tagged("web"))
        };
        assert!(below.has_sustained_low_cpu());

        let at_threshold = Ec2InstanceInfo {
            avg_cpu_utilization: Some(Ec2InstanceInfo::LOW_CPU_THRESHOLD_PERCENT),
            ..instance("running", tagged("web"))
        };
        assert!(
            !at_threshold.has_sustained_low_cpu(),
            "the threshold itself is not below it"
        );

        let stopped = Ec2InstanceInfo {
            avg_cpu_utilization: Some(0.0),
            ..instance("stopped", tagged("web"))
        };
        assert!(
            !stopped.has_sustained_low_cpu(),
            "a stopped instance idles by definition"
        );

        let no_reading = instance("running", tagged("web"));
        assert!(!no_reading.has_sustained_low_cpu(), "no metric is not zero");
    }

    #[test]
    fn a_missing_cpu_reading_renders_as_a_dash_not_zero() {
        assert_eq!(instance("running", tagged("web")).formatted_avg_cpu(), "-");

        let measured = Ec2InstanceInfo {
            avg_cpu_utilization: Some(2.345),
            ..instance("running", tagged("web"))
        };
        assert_eq!(measured.formatted_avg_cpu(), "2.3%");
    }

    #[test]
    fn tag_coverage_names_only_the_missing_tags() {
        let partial = instance("running", Tags::loaded([("Name", "web")]));

        assert_eq!(
            partial.missing_required_tags(),
            Some(vec!["Owner", "Environment"])
        );
        assert!(partial.has_tag_coverage_gap());

        let complete = instance("running", tagged("web"));
        assert_eq!(complete.missing_required_tags(), Some(Vec::new()));
        assert!(!complete.has_tag_coverage_gap());
    }

    /// Unreadable tags are not evidence of a gap. Reporting one would blame an
    /// instance for a failed lookup.
    #[test]
    fn unreadable_tags_are_not_a_coverage_gap() {
        let unknown = instance("running", Tags::Unavailable);

        assert_eq!(unknown.missing_required_tags(), None);
        assert!(!unknown.has_tag_coverage_gap());
        assert!(!unknown.review_signals().contains(&"missing-tags"));
    }

    #[test]
    fn review_signals_report_every_reason_at_once() {
        let mut bad = Ec2InstanceInfo {
            avg_cpu_utilization: Some(0.5),
            ..instance("running", Tags::loaded([("Name", "prod-api")]))
        };
        bad.public_ip = Some("54.1.2.3".into());

        assert_eq!(
            bad.review_signals(),
            vec!["public-ip", "prod-name", "missing-tags", "low-cpu"]
        );
    }

    #[test]
    fn a_clean_instance_has_no_signals() {
        assert!(instance("running", tagged("dev-box"))
            .review_signals()
            .is_empty());
    }
}
