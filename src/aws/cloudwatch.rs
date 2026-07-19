use crate::app::App;
use crate::models::cloudwatch::{CloudWatchAlarm, CloudWatchSummary};
use crate::models::service_status::ServiceStatus;

pub async fn fetch_cloudwatch(app: &App) -> (CloudWatchSummary, Vec<CloudWatchAlarm>) {
    let resp = match app.aws.cw.describe_alarms().send().await {
        Ok(r) => r,
        Err(err) => {
            return (
                CloudWatchSummary {
                    status: ServiceStatus::from_sdk_error(&err),
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
    let mut alarms = alarms;
    alarms.sort_by(|a, b| {
        let a_rank = if a.state == "ALARM" { 0 } else { 1 };
        let b_rank = if b.state == "ALARM" { 0 } else { 1 };

        a_rank
            .cmp(&b_rank)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.namespace.cmp(&b.namespace))
    });

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
