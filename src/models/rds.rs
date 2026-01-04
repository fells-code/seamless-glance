use crate::models::service_status::ServiceStatus;

#[derive(Debug, Clone)]
pub struct RdsSummary {
    pub status: ServiceStatus,
    pub total: usize,
    pub available: usize,
}

#[derive(Debug, Clone)]
pub struct RdsInstanceInfo {
    pub identifier: String,
    pub engine: String,
    pub instance_class: String,
    pub status: String,
    pub az: String,
    pub multi_az: bool,
}
