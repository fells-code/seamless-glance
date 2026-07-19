---
"seamless-glance": patch
---

Configure adaptive retry and timeouts for every AWS client. The SDK ran with default retries and no timeouts, so large accounts could hit throttling and partial failures, and an unreachable or disabled region could stall a refresh. All clients now use adaptive retry, which backs off when AWS starts throttling, plus connect, per-attempt, and overall operation timeouts.
