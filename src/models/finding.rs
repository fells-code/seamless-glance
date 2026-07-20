#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    High,
    Medium,
    Low,
}

impl FindingSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingSeverity::High => "HIGH",
            FindingSeverity::Medium => "MED",
            FindingSeverity::Low => "LOW",
        }
    }

    pub fn rank(&self) -> u8 {
        match self {
            FindingSeverity::High => 0,
            FindingSeverity::Medium => 1,
            FindingSeverity::Low => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingCategory {
    Incident,
    Waste,
    Hygiene,
}

impl FindingCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingCategory::Incident => "Incident",
            FindingCategory::Waste => "Waste",
            FindingCategory::Hygiene => "Hygiene",
        }
    }

    /// Triage order. Sorting on this rather than the label keeps incidents
    /// ahead of waste, which alphabetical ordering would not.
    pub fn rank(&self) -> u8 {
        match self {
            FindingCategory::Incident => 0,
            FindingCategory::Waste => 1,
            FindingCategory::Hygiene => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingRoute {
    Ec2,
    CloudWatch,
    Lambda,
    Rds,
    Apigateway,
    Secrets,
    Sqs,
    TargetGroups,
    LoadBalancers,
    SecurityGroups,
    Vpc,
}

#[derive(Debug, Clone)]
pub struct Finding {
    /// Which rule produced this finding. Part of the stable key, so that two
    /// rules reporting the same resource stay distinct entries in the queue.
    pub rule: &'static str,
    /// The AWS-side identity of the resource this finding is about.
    ///
    /// `None` for findings with no single underlying resource: account-level
    /// rollups, and the overview fallbacks that fire when a detail list could
    /// not be fetched and only summary counters are available.
    pub resource_id: Option<String>,
    pub severity: FindingSeverity,
    pub category: FindingCategory,
    pub service: String,
    pub region: String,
    pub summary: String,
    pub next_step: String,
    pub route: FindingRoute,
}

impl Finding {
    /// Identity that survives a refresh, for deduplication and acknowledgement.
    ///
    /// `None` when the finding is not about one identifiable resource, which is
    /// also why acknowledging such a finding cannot be made to stick.
    pub fn key(&self) -> Option<String> {
        self.resource_id
            .as_ref()
            .map(|id| format!("{}|{}|{}", self.rule, self.region, id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(rule: &'static str, region: &str, resource_id: Option<&str>) -> Finding {
        Finding {
            rule,
            resource_id: resource_id.map(str::to_string),
            severity: FindingSeverity::Medium,
            category: FindingCategory::Hygiene,
            service: "EC2".into(),
            region: region.into(),
            summary: "summary".into(),
            next_step: "next".into(),
            route: FindingRoute::Ec2,
        }
    }

    #[test]
    fn the_same_resource_under_two_rules_gets_distinct_keys() {
        let a = finding("rule_a", "us-east-1", Some("i-1"));
        let b = finding("rule_b", "us-east-1", Some("i-1"));

        assert_ne!(a.key(), b.key());
    }

    #[test]
    fn the_same_resource_id_in_two_regions_gets_distinct_keys() {
        let a = finding("rule_a", "us-east-1", Some("i-1"));
        let b = finding("rule_a", "eu-west-1", Some("i-1"));

        assert_ne!(a.key(), b.key());
    }

    #[test]
    fn a_key_is_stable_across_rebuilds() {
        assert_eq!(
            finding("rule_a", "us-east-1", Some("i-1")).key(),
            finding("rule_a", "us-east-1", Some("i-1")).key()
        );
    }

    #[test]
    fn a_finding_with_no_resource_has_no_key() {
        assert_eq!(finding("rule_a", "us-east-1", None).key(), None);
    }

    #[test]
    fn incidents_sort_ahead_of_waste_and_hygiene() {
        assert!(FindingCategory::Incident.rank() < FindingCategory::Waste.rank());
        assert!(FindingCategory::Waste.rank() < FindingCategory::Hygiene.rank());
    }

    #[test]
    fn severity_ranks_high_first_and_low_last() {
        assert!(FindingSeverity::High.rank() < FindingSeverity::Medium.rank());
        assert!(FindingSeverity::Medium.rank() < FindingSeverity::Low.rank());
    }
}
