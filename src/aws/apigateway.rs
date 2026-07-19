use crate::{
    app::App,
    models::{
        apigatway::{ApiGatewayInfo, ApiGatewaySummary},
        service_status::ServiceStatus,
    },
};

pub async fn fetch_apigateway_summary(app: &App) -> ApiGatewaySummary {
    let mut rest_count = 0;
    let mut http_count = 0;

    let mut saw_access_denied = false;
    let mut error_message: Option<String> = None;

    // --- REST APIs ---
    match app.aws.apigw.get_rest_apis().send().await {
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
    match app.aws.apigwv2.get_apis().send().await {
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

pub async fn fetch_apigateway_apis(app: &App) -> (Vec<ApiGatewayInfo>, ServiceStatus) {
    let mut apis = vec![];

    let mut saw_access_denied = false;
    let mut error_message: Option<String> = None;

    match app.aws.apigw.get_rest_apis().send().await {
        Ok(resp) => {
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
        Err(err) => match ServiceStatus::from_error_message(err.to_string()) {
            ServiceStatus::AccessDenied => saw_access_denied = true,
            ServiceStatus::Unavailable(msg) => error_message = Some(msg),
            ServiceStatus::Ok => {}
        },
    }

    match app.aws.apigwv2.get_apis().send().await {
        Ok(resp) => {
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
        Err(err) => match ServiceStatus::from_error_message(err.to_string()) {
            ServiceStatus::AccessDenied => saw_access_denied = true,
            ServiceStatus::Unavailable(msg) => error_message = Some(msg),
            ServiceStatus::Ok => {}
        },
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
