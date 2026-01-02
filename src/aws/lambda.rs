use crate::models::service_status::ServiceStatus;
use aws_sdk_lambda::Client;

#[derive(Debug, Clone)]
pub struct LambdaSummary {
    pub function_count: u32,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone)]
pub struct LambdaFunctionInfo {
    pub name: String,
    pub runtime: String,
    pub memory_mb: i32,
    pub timeout_sec: i32,
    pub last_modified: String,
}

pub async fn fetch_lambda_summary() -> LambdaSummary {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;
    let client = Client::new(&config);

    match client.list_functions().send().await {
        Ok(resp) => LambdaSummary {
            function_count: resp.functions().len() as u32,
            status: ServiceStatus::Ok,
        },
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("AccessDenied") {
                LambdaSummary {
                    function_count: 0,
                    status: ServiceStatus::AccessDenied,
                }
            } else {
                LambdaSummary {
                    function_count: 0,
                    status: ServiceStatus::Unavailable(msg),
                }
            }
        }
    }
}

pub async fn fetch_lambda_functions() -> Vec<LambdaFunctionInfo> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::v2025_08_07()).await;
    let client = Client::new(&config);

    let resp = match client.list_functions().send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    resp.functions()
        .iter()
        .map(|f| LambdaFunctionInfo {
            name: f.function_name().unwrap_or("unknown").to_string(),
            runtime: f
                .runtime()
                .map(|r| format!("{:?}", r))
                .unwrap_or("unknown".into()),
            memory_mb: f.memory_size().unwrap_or(0),
            timeout_sec: f.timeout().unwrap_or(0),
            last_modified: f.last_modified().unwrap_or("-").to_string(),
        })
        .collect()
}
