---
"seamless-glance": patch
---

Stop reporting dead-letter queues as SQS queues that are missing a DLQ. A queue's `RedrivePolicy` names the ARN of the queue it redrives to, so the set of queues acting as DLQs is derivable from data already fetched, with no extra API calls and no reliance on `*-dlq` naming conventions. Queues that another queue redrives to are now excluded from the missing-DLQ finding.
