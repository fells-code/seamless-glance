---
"seamless-glance": patch
---

Surface EC2 fetch failures instead of showing an empty table. EC2 was the one service whose errors were written to an invisible log and rendered as an empty instance list, because it aggregated regions through a helper that discarded status. EC2, RDS, and Lambda now share one status-preserving multi-region fetch path, so a denied or failed EC2 fetch reads as denied or unavailable like every other service, and a global fetch where only some regions answer still shows the data it did get.
