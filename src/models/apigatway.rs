use async_trait::async_trait;

use crate::{
    aws::clients::AwsClients,
    models::{describable::DescribableResource, service_status::ServiceStatus},
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
}
