---
"seamless-glance": patch
---

Fetch target-group health and SQS queue attributes concurrently instead of one resource at a time. Both were serial loops issuing a describe per resource, so an account with many target groups or queues made hundreds of sequential round trips and amplified throttling. These per-resource calls now run concurrently with a bounded cap, so those views load much faster at scale.
