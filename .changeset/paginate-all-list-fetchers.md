---
"seamless-glance": patch
---

Return the full inventory instead of only the first page. Every service fetcher read a single page of results with no pagination, so accounts with many resources got truncated lists and undercounted overview numbers with no indication. All list fetchers (EC2, RDS, Lambda, SQS, VPC, subnets, security groups, load balancers, target groups, CloudWatch alarms, Secrets Manager, API Gateway REST and HTTP, ECS clusters) now page through the complete result set.
