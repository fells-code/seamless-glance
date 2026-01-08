use crate::models::service_status::ServiceStatus;

pub struct ElbResult {
    pub count: u32,
    pub status: ServiceStatus,
}
