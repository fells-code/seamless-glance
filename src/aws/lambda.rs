use crate::{
    app::App,
    aws::clients::clients_for_region,
    aws::tags,
    models::{
        lambda::{LambdaFunctionInfo, LambdaSummary},
        service_status::ServiceStatus,
        tags::Tags,
    },
    resources::region_aggregate::fetch_all_regions,
};
use aws_types::region::Region;

const UNAVAILABLE_EVERYWHERE: &str = "Lambda unavailable in all regions";

async fn fetch_lambda_for_region(
    region: Region,
    include_tags: bool,
    profile: Option<String>,
) -> Result<Vec<LambdaFunctionInfo>, ServiceStatus> {
    let aws = clients_for_region(&region, profile.as_deref()).await;

    let mut pages = aws.lambda.list_functions().into_paginator().items().send();

    let mut functions = Vec::new();
    let mut arns = Vec::new();

    while let Some(item) = pages.next().await {
        let f = match item {
            Ok(f) => f,
            Err(err) => return Err(ServiceStatus::from_sdk_error(&err)),
        };

        arns.push(f.function_arn().map(|arn| arn.to_string()));

        functions.push(LambdaFunctionInfo {
            name: f.function_name().unwrap_or("unknown").to_string(),
            region: region.as_ref().to_string(),
            runtime: f
                .runtime()
                .map(|r| format!("{:?}", r))
                .unwrap_or("unknown".into()),
            memory_mb: f.memory_size().unwrap_or(0),
            timeout_sec: f.timeout().unwrap_or(0),
            last_modified: f.last_modified().unwrap_or("-").to_string(),
            tags: Tags::Unavailable,
        });
    }

    if !include_tags {
        return Ok(functions);
    }

    // ListFunctions does not return tags, so they need one ListTags per function.
    let looked_up = tags::lookup_each(arns, |arn| {
        let aws = &aws;
        async move {
            let resp = aws.lambda.list_tags().resource(arn?).send().await.ok()?;
            Some(tags::from_map(resp.tags()))
        }
    })
    .await;

    for (function, tags) in functions.iter_mut().zip(looked_up) {
        function.tags = tags;
    }

    Ok(functions)
}

pub async fn fetch_lambda_functions(app: &App) -> (Vec<LambdaFunctionInfo>, ServiceStatus) {
    fetch_lambda(app, true).await
}

/// `include_tags` costs one `ListTags` call per function, so callers that only
/// need a count skip it.
async fn fetch_lambda(app: &App, include_tags: bool) -> (Vec<LambdaFunctionInfo>, ServiceStatus) {
    let profile = app.current_profile.clone();

    let (mut functions, status) = if app.is_global_region_selected() {
        fetch_all_regions(&app.regions, UNAVAILABLE_EVERYWHERE, move |region| {
            fetch_lambda_for_region(region, include_tags, profile.clone())
        })
        .await
    } else {
        match fetch_lambda_for_region(app.current_region().clone(), include_tags, profile).await {
            Ok(functions) => (functions, ServiceStatus::Ok),
            Err(status) => (vec![], status),
        }
    };

    functions.sort_by(|a, b| a.region.cmp(&b.region).then_with(|| a.name.cmp(&b.name)));

    (functions, status)
}

pub async fn fetch_lambda_summary(app: &App) -> LambdaSummary {
    let (functions, status) = fetch_lambda(app, false).await;

    crate::aws::account::lambda_summary(&functions, status)
}
