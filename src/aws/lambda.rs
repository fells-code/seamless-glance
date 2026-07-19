use crate::{
    app::App,
    aws::clients::{build_sdk_config, AwsClients},
    models::{
        lambda::{LambdaFunctionInfo, LambdaSummary},
        service_status::ServiceStatus,
    },
};
use aws_types::region::Region;
use futures::future::join_all;

async fn clients_for_region(region: &Region, profile: Option<&str>) -> AwsClients {
    let sdk_config = build_sdk_config(region.clone(), profile).await;
    AwsClients::new(&sdk_config)
}

async fn fetch_lambda_for_region(
    region: Region,
    profile: Option<String>,
) -> Result<Vec<LambdaFunctionInfo>, ServiceStatus> {
    let aws = clients_for_region(&region, profile.as_deref()).await;

    let resp = match aws.lambda.list_functions().send().await {
        Ok(r) => r,
        Err(err) => {
            let msg = err.to_string();
            let status = if msg.contains("AccessDenied") {
                ServiceStatus::AccessDenied
            } else {
                ServiceStatus::Unavailable(msg)
            };

            return Err(status);
        }
    };

    let functions = resp
        .functions()
        .iter()
        .map(|f| LambdaFunctionInfo {
            name: f.function_name().unwrap_or("unknown").to_string(),
            region: region.as_ref().to_string(),
            runtime: f
                .runtime()
                .map(|r| format!("{:?}", r))
                .unwrap_or("unknown".into()),
            memory_mb: f.memory_size().unwrap_or(0),
            timeout_sec: f.timeout().unwrap_or(0),
            last_modified: f.last_modified().unwrap_or("-").to_string(),
        })
        .collect();

    Ok(functions)
}

pub async fn fetch_lambda_functions(app: &App) -> Vec<LambdaFunctionInfo> {
    let profile = app.current_profile.clone();
    let mut functions = if app.is_global_region_selected() {
        let region_profile = profile.clone();
        let futures = app
            .regions
            .iter()
            .cloned()
            .map(move |region| fetch_lambda_for_region(region, region_profile.clone()));

        let results = join_all(futures).await;

        let mut all = Vec::new();

        for result in results {
            match result {
                Ok(mut funcs) => all.append(&mut funcs),
                Err(ServiceStatus::AccessDenied) => {
                    eprintln!("Access denied to Lambda in one region");
                }
                Err(ServiceStatus::Unavailable(msg)) => {
                    eprintln!("Lambda unavailable in one region: {}", msg);
                }
                Err(_) => {}
            }
        }

        all
    } else {
        fetch_lambda_for_region(app.current_region().clone(), profile)
            .await
            .unwrap_or_default()
    };

    functions.sort_by(|a, b| a.region.cmp(&b.region).then_with(|| a.name.cmp(&b.name)));

    functions
}

pub async fn fetch_lambda_summary(app: &App) -> LambdaSummary {
    if !app.is_global_region_selected() {
        return match fetch_lambda_for_region(
            app.current_region().clone(),
            app.current_profile.clone(),
        )
        .await
        {
            Ok(functions) => LambdaSummary {
                function_count: functions.len() as u32,
                status: ServiceStatus::Ok,
            },
            Err(status) => LambdaSummary {
                function_count: 0,
                status,
            },
        };
    }

    let profile = app.current_profile.clone();
    let futures = app
        .regions
        .iter()
        .cloned()
        .map(move |region| fetch_lambda_for_region(region, profile.clone()));

    let results = join_all(futures).await;

    let mut function_count = 0u32;
    let mut any_success = false;
    let mut saw_access_denied = false;
    let mut saw_unavailable_msg: Option<String> = None;

    for result in results {
        match result {
            Ok(functions) => {
                any_success = true;
                function_count += functions.len() as u32;
            }
            Err(ServiceStatus::AccessDenied) => {
                saw_access_denied = true;
            }
            Err(ServiceStatus::Unavailable(msg)) => {
                if saw_unavailable_msg.is_none() {
                    saw_unavailable_msg = Some(msg);
                }
            }
            Err(_) => {}
        }
    }

    let status = if any_success {
        ServiceStatus::Ok
    } else if saw_access_denied {
        ServiceStatus::AccessDenied
    } else {
        ServiceStatus::Unavailable(
            saw_unavailable_msg.unwrap_or_else(|| "Lambda unavailable in all regions".to_string()),
        )
    };

    LambdaSummary {
        function_count,
        status,
    }
}
