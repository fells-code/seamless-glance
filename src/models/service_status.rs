use aws_smithy_types::error::metadata::ProvideErrorMetadata;

#[derive(Debug, Clone)]
pub enum ServiceStatus {
    Ok,
    AccessDenied,
    Unavailable(String), // future: throttling, region disabled, etc.
}

impl ServiceStatus {
    /// Classify an AWS SDK error into a status by inspecting its error code
    /// rather than substring-matching the display string, so authorization
    /// denials are recognized across services regardless of how they render.
    pub fn from_sdk_error<E: ProvideErrorMetadata>(err: &E) -> Self {
        if err.code().is_some_and(is_access_denied_code) {
            ServiceStatus::AccessDenied
        } else {
            ServiceStatus::Unavailable(error_summary(err))
        }
    }
}

/// Authorization-denial error codes seen across the AWS services this app
/// queries. EC2 uses `UnauthorizedOperation`/`AuthFailure`; most other services
/// use `AccessDenied` or `AccessDeniedException`.
fn is_access_denied_code(code: &str) -> bool {
    matches!(
        code,
        "AccessDenied"
            | "AccessDeniedException"
            | "UnauthorizedOperation"
            | "AuthorizationError"
            | "AuthFailure"
            | "Forbidden"
    )
}

fn error_summary<E: ProvideErrorMetadata>(err: &E) -> String {
    match (err.code(), err.message()) {
        (Some(code), Some(message)) => format!("{code}: {message}"),
        (Some(code), None) => code.to_string(),
        (None, Some(message)) => message.to_string(),
        (None, None) => "unknown error".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_smithy_types::error::metadata::ErrorMetadata;

    fn error_with_code(code: &str) -> ErrorMetadata {
        ErrorMetadata::builder()
            .code(code)
            .message("denied")
            .build()
    }

    #[test]
    fn recognizes_access_denied_variants_by_code() {
        for code in [
            "AccessDenied",
            "AccessDeniedException",
            "UnauthorizedOperation",
            "AuthFailure",
        ] {
            assert!(
                matches!(
                    ServiceStatus::from_sdk_error(&error_with_code(code)),
                    ServiceStatus::AccessDenied
                ),
                "expected {code} to classify as AccessDenied"
            );
        }
    }

    #[test]
    fn non_auth_errors_are_unavailable_with_a_summary() {
        let status = ServiceStatus::from_sdk_error(&error_with_code("ThrottlingException"));
        match status {
            ServiceStatus::Unavailable(msg) => {
                assert!(msg.contains("ThrottlingException"));
                assert!(msg.contains("denied"));
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn missing_metadata_falls_back_to_unavailable() {
        let empty = ErrorMetadata::builder().build();
        assert!(matches!(
            ServiceStatus::from_sdk_error(&empty),
            ServiceStatus::Unavailable(_)
        ));
    }
}
