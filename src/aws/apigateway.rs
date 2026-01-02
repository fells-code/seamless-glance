use aws_sdk_apigateway::Client as RestClient;
use aws_sdk_apigatewayv2::Client as V2Client;

use crate::models::service_status::ServiceStatus;

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

pub async fn fetch_apigateway_summary() -> ApiGatewaySummary {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;

    let rest = RestClient::new(&config);
    let v2 = V2Client::new(&config);

    let mut rest_count = 0;
    let mut http_count = 0;

    let mut saw_access_denied = false;
    let mut error_message: Option<String> = None;

    // --- REST APIs ---
    match rest.get_rest_apis().send().await {
        Ok(resp) => {
            rest_count = resp.items().len() as u32;
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("AccessDenied") {
                saw_access_denied = true;
            } else {
                error_message = Some(msg);
            }
        }
    }

    // --- HTTP APIs ---
    match v2.get_apis().send().await {
        Ok(resp) => {
            http_count = resp.items().len() as u32;
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("AccessDenied") {
                saw_access_denied = true;
            } else {
                error_message = Some(msg);
            }
        }
    }

    let status = if saw_access_denied {
        ServiceStatus::AccessDenied
    } else if let Some(msg) = error_message {
        ServiceStatus::Unavailable(msg)
    } else {
        ServiceStatus::Ok
    };

    ApiGatewaySummary {
        rest_count,
        http_count,
        status,
    }
}

pub async fn fetch_apigateway_apis() -> Vec<ApiGatewayInfo> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;

    let rest = RestClient::new(&config);
    let v2 = V2Client::new(&config);

    let mut apis = vec![];

    if let Ok(resp) = rest.get_rest_apis().send().await {
        for api in resp.items() {
            apis.push(ApiGatewayInfo {
                id: api.id().unwrap_or("-").to_string(),
                name: api.name().unwrap_or("unnamed").to_string(),
                api_type: "REST".into(),
                created_at: api
                    .created_date()
                    .map(|d| d.to_string())
                    .unwrap_or("-".into()),
            });
        }
    }

    if let Ok(resp) = v2.get_apis().send().await {
        for api in resp.items() {
            apis.push(ApiGatewayInfo {
                id: api.api_id().unwrap_or("-").to_string(),
                name: api.name().unwrap_or("unnamed").to_string(),
                api_type: api
                    .protocol_type()
                    .map(|p| format!("{:?}", p))
                    .unwrap_or("HTTP".into()),
                created_at: api
                    .created_date()
                    .map(|d| d.to_string())
                    .unwrap_or("-".into()),
            });
        }
    }

    apis
}
