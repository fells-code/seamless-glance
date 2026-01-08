use crate::app::App;
use crate::models::cloudwatch::{CloudWatchAlarm, CloudWatchSummary};
use crate::models::service_status::ServiceStatus;

pub async fn fetch_cloudwatch(app: &App) -> (CloudWatchSummary, Vec<CloudWatchAlarm>) {
    let resp = match app.aws.cw.describe_alarms().send().await {
        Ok(r) => r,
        Err(err) => {
            let msg = err.to_string();
            let status = if msg.contains("AccessDenied") {
                ServiceStatus::AccessDenied
            } else {
                ServiceStatus::Unavailable(msg)
            };

            return (
                CloudWatchSummary {
                    status,
                    total_alarms: 0,
                    alarms_in_alarm: 0,
                },
                vec![],
            );
        }
    };

    let alarms: Vec<CloudWatchAlarm> = resp
        .metric_alarms()
        .iter()
        .map(|a| CloudWatchAlarm {
            name: a.alarm_name().unwrap_or("").to_string(),
            state: a
                .state_value()
                .map(|s| s.as_str())
                .unwrap_or("UNKNOWN")
                .to_string(),
            namespace: a.namespace().unwrap_or("").to_string(),
            metric: a.metric_name().unwrap_or("").to_string(),
        })
        .collect();

    let total = alarms.len();
    let alarming = alarms.iter().filter(|a| a.state == "ALARM").count();

    (
        CloudWatchSummary {
            status: ServiceStatus::Ok,
            total_alarms: total,
            alarms_in_alarm: alarming,
        },
        alarms,
    )
}
