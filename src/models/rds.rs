use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::{
        describable::{shell_quote, DescribableResource},
        service_status::ServiceStatus,
        tags::Tags,
    },
};

#[derive(Debug, Clone)]
pub struct RdsSummary {
    pub status: ServiceStatus,
    pub total: usize,
    pub available: usize,
}

#[derive(Debug, Clone)]
pub struct RdsInstanceInfo {
    pub identifier: String,
    pub region: String,
    pub engine: String,
    pub instance_class: String,
    pub status: String,
    pub az: String,
    pub multi_az: bool,
    pub tags: Tags,
}

impl RdsInstanceInfo {
    pub const PRODUCTION_NAME_HINTS: [&str; 7] = [
        "prod",
        "production",
        "live",
        "critical",
        "primary",
        "main",
        "customer",
    ];

    pub fn is_available(&self) -> bool {
        self.status == "available"
    }

    pub fn has_production_like_identifier(&self) -> bool {
        let normalized = self.identifier.to_ascii_lowercase();
        Self::PRODUCTION_NAME_HINTS
            .iter()
            .any(|hint| normalized.contains(hint))
    }

    pub fn needs_single_az_review(&self) -> bool {
        self.is_available() && !self.multi_az && self.has_production_like_identifier()
    }

    pub fn review_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if !self.multi_az {
            signals.push("single-az");
        }

        if self.has_production_like_identifier() {
            signals.push("prod-name");
        }

        signals
    }
}

#[async_trait]
impl DescribableResource for RdsInstanceInfo {
    fn resource_name(&self) -> String {
        self.identifier.clone()
    }

    fn action_region(&self) -> Option<&str> {
        Some(&self.region)
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        let resp = clients
            .rds
            .describe_db_instances()
            .db_instance_identifier(&self.identifier)
            .send()
            .await?;

        Ok(format!("{:#?}", resp))
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://{}.console.aws.amazon.com/rds/home?region={}#database:id={}",
            region, region, self.identifier
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        Some(format!(
            "aws rds describe-db-instances --db-instance-identifier {} --region {}",
            shell_quote(&self.identifier),
            shell_quote(region)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(identifier: &str, status: &str, multi_az: bool) -> RdsInstanceInfo {
        RdsInstanceInfo {
            identifier: identifier.into(),
            region: "us-east-1".into(),
            engine: "postgres".into(),
            instance_class: "db.t3.medium".into(),
            status: status.into(),
            az: "us-east-1a".into(),
            multi_az,
            tags: Tags::empty(),
        }
    }

    #[test]
    fn single_az_review_needs_all_three_conditions() {
        assert!(db("prod-orders", "available", false).needs_single_az_review());

        assert!(
            !db("prod-orders", "available", true).needs_single_az_review(),
            "multi-AZ is already covered"
        );
        assert!(
            !db("dev-orders", "available", false).needs_single_az_review(),
            "a non-production name is not worth flagging"
        );
        assert!(
            !db("prod-orders", "creating", false).needs_single_az_review(),
            "an instance that is not available yet is not a coverage gap"
        );
    }

    #[test]
    fn a_production_like_identifier_matches_a_hint_anywhere() {
        for identifier in ["prod-db", "DB-PRODUCTION", "customer-data", "main-writer"] {
            assert!(
                db(identifier, "available", false).has_production_like_identifier(),
                "{identifier} should read as production-like"
            );
        }

        for identifier in ["dev-db", "test-writer", "sandbox"] {
            assert!(
                !db(identifier, "available", false).has_production_like_identifier(),
                "{identifier} should not read as production-like"
            );
        }
    }

    #[test]
    fn review_signals_report_each_reason() {
        assert_eq!(
            db("prod-orders", "available", false).review_signals(),
            vec!["single-az", "prod-name"]
        );
        assert_eq!(
            db("dev-orders", "available", false).review_signals(),
            vec!["single-az"]
        );
        assert!(db("dev-orders", "available", true)
            .review_signals()
            .is_empty());
    }
}
