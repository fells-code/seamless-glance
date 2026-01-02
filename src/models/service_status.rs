#[derive(Debug, Clone)]
pub enum ServiceStatus {
    Ok,
    AccessDenied,
    Unavailable(String), // future: throttling, region disabled, etc.
}
