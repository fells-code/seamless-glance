---
"seamless-glance": patch
---

Report real ECS cluster capacity and status instead of placeholders. The CPU and Memory columns were both set to the registered container-instance count, so three columns showed the same unrelated number, and the Health column was hardcoded to OK regardless of the cluster. CPU and memory are now the share of registered capacity in use, read from the container instances backing the cluster, and Status is the lifecycle state ECS reports. Fargate clusters register no instances and have no cluster-level capacity pool, so they show a dash rather than a zero that would read as an idle cluster. Capacity is only looked up for clusters that actually have container instances, so a Fargate-only account makes no extra API calls.
