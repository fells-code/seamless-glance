#[derive(Debug, Clone)]
pub enum ServiceStatus {
    Ok,
    AccessDenied,
    Unavailable(String), // future: throttling, region disabled, etc.
}

impl ServiceStatus {
    /// Classify an SDK error message into a status. Access denials become
    /// `AccessDenied`; everything else is surfaced as `Unavailable`.
    // TODO(#20): replace substring matching with typed SDK error inspection.
    pub fn from_error_message(message: String) -> Self {
        if message.contains("AccessDenied") {
            ServiceStatus::AccessDenied
        } else {
            ServiceStatus::Unavailable(message)
        }
    }
}
