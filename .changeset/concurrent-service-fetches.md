---
"seamless-glance": patch
---

Fetch the Findings and Cost Savings services concurrently instead of one after another. Those views awaited around eleven independent service fetches sequentially, so refresh latency was the sum of every call. The independent fetches now run concurrently with `tokio::join!`, so the wait is the slowest single fetch and the triage home paints much sooner on real accounts.
