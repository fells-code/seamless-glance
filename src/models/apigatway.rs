use chrono::{DateTime, Duration, TimeZone, Utc};

use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::{
        describable::{shell_quote, DescribableResource},
        service_status::ServiceStatus,
    },
};

#[derive(Debug, Clone)]
pub struct ApiGatewaySummary {
    pub rest_count: u32,
    pub http_count: u32,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone)]
pub struct ApiGatewayInfo {
    pub id: String,
    pub name: String,
    pub api_type: String, // REST | HTTP
    pub created_at: String,
}

impl ApiGatewayInfo {
    pub const STALE_API_DAYS: i64 = 365;
    pub const GENERIC_NAME_HINTS: [&str; 8] = [
        "unnamed", "default", "test", "example", "sample", "temp", "tmp", "my-api",
    ];

    pub fn has_generic_name(&self) -> bool {
        let normalized = self.name.trim().to_ascii_lowercase();

        if normalized.is_empty() || normalized == "api" {
            return true;
        }

        Self::GENERIC_NAME_HINTS
            .iter()
            .any(|hint| normalized == *hint || normalized.starts_with(&format!("{hint}-")))
    }

    pub fn is_stale(&self) -> bool {
        let Some(created_at) = self.parsed_created_at() else {
            return false;
        };

        created_at <= Utc::now() - Duration::days(Self::STALE_API_DAYS)
    }

    pub fn needs_review(&self) -> bool {
        self.has_generic_name() || self.is_stale()
    }

    pub fn review_signals(&self) -> Vec<&'static str> {
        let mut signals = Vec::new();

        if self.has_generic_name() {
            signals.push("generic-name");
        }

        if self.is_stale() {
            signals.push("stale");
        }

        signals
    }

    fn parsed_created_at(&self) -> Option<DateTime<Utc>> {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&self.created_at) {
            return Some(dt.with_timezone(&Utc));
        }

        if let Ok(epoch_seconds) = self.created_at.parse::<i64>() {
            return Utc.timestamp_opt(epoch_seconds, 0).single();
        }

        None
    }
}

#[async_trait]
impl DescribableResource for ApiGatewayInfo {
    fn resource_name(&self) -> String {
        self.id.clone()
    }

    async fn describe(&self, clients: &AwsClients) -> anyhow::Result<String> {
        if self.api_type == "REST" {
            let resp = clients
                .apigw
                .get_rest_api()
                .rest_api_id(&self.id)
                .send()
                .await?;
            Ok(format!("{:#?}", resp))
        } else {
            let resp = clients.apigwv2.get_api().api_id(&self.id).send().await?;
            Ok(format!("{:#?}", resp))
        }
    }

    fn console_url(&self, region: &str) -> Option<String> {
        Some(format!(
            "https://{}.console.aws.amazon.com/apigateway/main/develop/routes?api={}&region={}",
            region, self.id, region
        ))
    }

    fn cli_command(&self, region: &str) -> Option<String> {
        if self.api_type == "REST" {
            Some(format!(
                "aws apigateway get-rest-api --rest-api-id {} --region {}",
                shell_quote(&self.id),
                shell_quote(region)
            ))
        } else {
            Some(format!(
                "aws apigatewayv2 get-api --api-id {} --region {}",
                shell_quote(&self.id),
                shell_quote(region)
            ))
        }
    }
}
