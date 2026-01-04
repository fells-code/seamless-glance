use crate::models::service_status::ServiceStatus;

#[derive(Debug, Clone)]
pub struct SecretsSummary {
    pub status: ServiceStatus,
    pub total: usize,
    pub rotation_disabled: usize,
}

#[derive(Debug, Clone)]
pub struct SecretInfo {
    pub name: String,
    pub rotation_enabled: bool,
    pub last_rotated: Option<String>,
}
