use crate::{
    app::App,
    models::{
        lambda::{LambdaFunctionInfo, LambdaSummary},
        service_status::ServiceStatus,
    },
};

pub async fn fetch_lambda_summary(app: &App) -> LambdaSummary {
    match app.aws.lambda.list_functions().send().await {
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

pub async fn fetch_lambda_functions(app: &App) -> Vec<LambdaFunctionInfo> {
    let resp = match app.aws.lambda.list_functions().send().await {
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
