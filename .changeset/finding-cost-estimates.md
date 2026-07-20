---
"seamless-glance": patch
---

Waste findings for idle EC2 instances and idle load balancers now show what the resource costs, read from the AWS Price List API and shown as "~$30/mo list". The figure is AWS public list price for the resource as configured, not billed spend: discounts, Savings Plans, and Reserved Instances are not reflected, so it reads as an upper bound on the standing charge and is labelled as list price wherever it appears. Prices are cached on disk for a month and only looked up for resources that actually produce a finding. Waste whose cost is usage-driven rather than a standing hourly charge, such as Lambda memory or stale API Gateway APIs, shows no estimate rather than a fabricated one.
