use crate::models::service_status::ServiceStatus;

#[derive(Debug, Clone)]
pub struct CloudWatchSummary {
    pub status: ServiceStatus,
    pub total_alarms: usize,
    pub alarms_in_alarm: usize,
}

#[derive(Debug, Clone)]
pub struct CloudWatchAlarm {
    pub name: String,
    pub state: String,
    pub namespace: String,
    pub metric: String,
}
