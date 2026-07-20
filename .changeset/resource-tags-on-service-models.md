---
"seamless-glance": patch
---

Carry resource tags on EC2, RDS, VPC, security groups, Secrets Manager, API Gateway, and ECS, and add a finding that reports resources with no `Owner` tag so they can be attributed to a team. Tags previously existed only on EC2, which flattened `Name`, `Owner`, and `Environment` at fetch time and discarded everything else. All seven services read tags from responses they were already making, so this adds no extra API calls. A resource whose tags could not be read is tracked as unreadable rather than untagged, so a failed tag lookup is never reported as missing ownership.
