---
"seamless-glance": patch
---

Classify access-denied errors by their AWS error code instead of substring-matching the display string. Detection previously looked for `AccessDenied` in the error text, so authorization failures that surface as `AccessDeniedException`, `UnauthorizedOperation`, or `AuthFailure` were mislabeled as generic "unavailable". All services now classify through a shared `ServiceStatus::from_sdk_error` helper that inspects the SDK error code, so a denied resource reads as access-denied across the board.
