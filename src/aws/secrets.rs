use crate::app::App;
use crate::models::secrets::{SecretInfo, SecretsSummary};
use crate::models::service_status::ServiceStatus;

use chrono::{DateTime, Utc};
use std::time::{Duration, UNIX_EPOCH};

fn aws_datetime_to_utc(dt: &aws_sdk_secretsmanager::primitives::DateTime) -> DateTime<Utc> {
    let system_time = UNIX_EPOCH
        + Duration::from_secs(dt.secs() as u64)
        + Duration::from_nanos(dt.subsec_nanos() as u64);

    DateTime::<Utc>::from(system_time)
}

pub async fn fetch_secrets(app: &App) -> (SecretsSummary, Vec<SecretInfo>) {
    let resp = match app.aws.sm.list_secrets().send().await {
        Ok(r) => r,
        Err(err) => {
            let msg = err.to_string();
            let status = if msg.contains("AccessDenied") {
                ServiceStatus::AccessDenied
            } else {
                ServiceStatus::Unavailable(msg)
            };

            return (
                SecretsSummary {
                    status,
                    total: 0,
                    rotation_disabled: 0,
                },
                vec![],
            );
        }
    };

    let mut secrets = Vec::new();
    let mut rotation_disabled = 0;

    for s in resp.secret_list() {
        let rotation_enabled = s.rotation_enabled().unwrap_or(false);
        if !rotation_enabled {
            rotation_disabled += 1;
        }

        let last_rotated = s
            .last_rotated_date()
            .map(|d| aws_datetime_to_utc(d).to_rfc3339());

        secrets.push(SecretInfo {
            name: s.name().unwrap_or("unknown").to_string(),
            rotation_enabled,
            last_rotated,
        });
    }

    (
        SecretsSummary {
            status: ServiceStatus::Ok,
            total: secrets.len(),
            rotation_disabled,
        },
        secrets,
    )
}
