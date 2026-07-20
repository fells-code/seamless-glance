---
"seamless-glance": patch
---

Carry resource tags on Lambda, SQS, CloudWatch alarms, load balancers, and target groups, completing tag coverage across every service. These five do not return tags on their list or describe responses, so they need a separate lookup: Lambda, SQS, and CloudWatch fetch one per resource with bounded concurrency, while load balancers and target groups batch 20 ARNs per call. A lookup that fails marks that resource's tags unreadable rather than empty, so a throttled call is never reported as missing ownership. The `Owner` tag finding now covers all eleven tagged services.
