use crate::{
    app::App,
    aws::tags,
    models::{
        apigatway::{ApiGatewayInfo, ApiGatewaySummary},
        service_status::ServiceStatus,
    },
};

pub async fn fetch_apigateway_summary(app: &App) -> ApiGatewaySummary {
    let mut rest_count = 0u32;
    let mut http_count = 0u32;

    let mut saw_access_denied = false;
    let mut error_message: Option<String> = None;

    // --- REST APIs ---
    let mut rest_pages = app
        .aws
        .apigw
        .get_rest_apis()
        .into_paginator()
        .items()
        .send();
    while let Some(item) = rest_pages.next().await {
        match item {
            Ok(_) => rest_count += 1,
            Err(err) => {
                match ServiceStatus::from_sdk_error(&err) {
                    ServiceStatus::AccessDenied => saw_access_denied = true,
                    ServiceStatus::Unavailable(msg) => error_message = Some(msg),
                    ServiceStatus::Ok => {}
                }
                break;
            }
        }
    }

    // --- HTTP APIs (apigatewayv2 GetApis has no generated paginator) ---
    let mut next_token: Option<String> = None;
    loop {
        let mut request = app.aws.apigwv2.get_apis();
        if let Some(token) = &next_token {
            request = request.next_token(token);
        }

        match request.send().await {
            Ok(resp) => {
                http_count += resp.items().len() as u32;
                match resp.next_token() {
                    Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                    _ => break,
                }
            }
            Err(err) => {
                match ServiceStatus::from_sdk_error(&err) {
                    ServiceStatus::AccessDenied => saw_access_denied = true,
                    ServiceStatus::Unavailable(msg) => error_message = Some(msg),
                    ServiceStatus::Ok => {}
                }
                break;
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

pub async fn fetch_apigateway_apis(app: &App) -> (Vec<ApiGatewayInfo>, ServiceStatus) {
    let mut apis = vec![];

    let mut saw_access_denied = false;
    let mut error_message: Option<String> = None;

    let mut rest_pages = app
        .aws
        .apigw
        .get_rest_apis()
        .into_paginator()
        .items()
        .send();
    while let Some(item) = rest_pages.next().await {
        match item {
            Ok(api) => apis.push(ApiGatewayInfo {
                id: api.id().unwrap_or("-").to_string(),
                name: api.name().unwrap_or("unnamed").to_string(),
                api_type: "REST".into(),
                created_at: api
                    .created_date()
                    .map(|d| d.to_string())
                    .unwrap_or("-".into()),
                tags: tags::from_map(api.tags()),
            }),
            Err(err) => {
                match ServiceStatus::from_sdk_error(&err) {
                    ServiceStatus::AccessDenied => saw_access_denied = true,
                    ServiceStatus::Unavailable(msg) => error_message = Some(msg),
                    ServiceStatus::Ok => {}
                }
                break;
            }
        }
    }

    // apigatewayv2 GetApis has no generated paginator; walk its next-token manually.
    let mut next_token: Option<String> = None;
    loop {
        let mut request = app.aws.apigwv2.get_apis();
        if let Some(token) = &next_token {
            request = request.next_token(token);
        }

        match request.send().await {
            Ok(resp) => {
                for api in resp.items() {
                    apis.push(ApiGatewayInfo {
                        id: api.api_id().unwrap_or("-").to_string(),
                        name: api.name().unwrap_or("unnamed").to_string(),
                        api_type: api
                            .protocol_type()
                            .map(|p| format!("{:?}", p))
                            .unwrap_or("HTTP".into()),
                        tags: tags::from_map(api.tags()),
                        created_at: api
                            .created_date()
                            .map(|d| d.to_string())
                            .unwrap_or("-".into()),
                    });
                }

                match resp.next_token() {
                    Some(token) if !token.is_empty() => next_token = Some(token.to_string()),
                    _ => break,
                }
            }
            Err(err) => {
                match ServiceStatus::from_sdk_error(&err) {
                    ServiceStatus::AccessDenied => saw_access_denied = true,
                    ServiceStatus::Unavailable(msg) => error_message = Some(msg),
                    ServiceStatus::Ok => {}
                }
                break;
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

    (apis, status)
}
