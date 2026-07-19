---
"seamless-glance": patch
---

Stop crashing on denied or throttled STS and ECS calls. A denied `sts:GetCallerIdentity` or a failed ECS `list_clusters`/`describe_clusters` previously panicked and took the whole dashboard down, which is easy to hit on scoped least-privilege profiles. These now degrade to an unknown identity and an empty cluster list instead of crashing.
