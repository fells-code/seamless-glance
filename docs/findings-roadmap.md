# Findings Roadmap

## Purpose

This document captures the planned findings backlog for Seamless Glance.

The goal is not to list every possible AWS smell. The goal is to prioritize findings that best support the product direction:

1. triage accelerator
2. waste catalog
3. fast pivot to action

Each finding should answer:

- what needs attention now
- what may be costing money for little value
- what should the operator inspect next

## Finding Taxonomy

Every finding should be classified as one of:

- `Incident`: something likely broken, degraded, or urgent
- `Waste`: something likely costing money for little value
- `Hygiene`: something risky, sloppy, or likely to become a problem later

Each finding should also carry:

- severity
- service
- region
- summary
- why it matters
- next step
- related resource count or affected resource
- pivot targets: service view, AWS CLI, AWS console

## Current Findings

The app currently implements an initial set of high-signal findings:

- named CloudWatch alarms in `ALARM`
- CloudWatch coverage gaps for deployed services without matching alarm namespaces
- target groups with zero healthy targets
- unhealthy target groups
- target groups with no load balancer attachment and no registered targets
- load balancers with no active target path
- load balancers with zero healthy targets
- running EC2 instances averaging below 5 percent CPU over the last 7 days
- stopped instances with public IP or production-like naming
- EC2 instances missing `Name`, `Owner`, or `Environment` tags
- API Gateway APIs with generic names or age over one year
- SQS queues with high visible or in-flight message counts
- SQS queues without DLQs
- RDS instances not `available`
- production-like single-AZ RDS instances
- production-like secrets without rotation
- secrets with stale rotation despite rotation being enabled
- Lambda functions with suspiciously high memory
- Lambda functions with very old last-modified dates
- default VPC still present
- secrets without rotation
- stopped EC2 instances
- security groups open to the world
- security groups exposing sensitive ports publicly

These are a foundation, not the target end state.

## Near-Term Findings

These are the best next additions because they fit the current architecture and the data is mostly already present or easy to add.

### EC2

#### Stopped instances with likely waste signal

- Category: `Waste`
- Why it matters: stopped compute often indicates abandoned dev or old experiments
- Needed data: existing instance list, plus optional age or tags later
- Best pivots: EC2 view, CLI describe-instances, console link

### Security Groups

#### Open to world on any ingress

- Category: `Hygiene`
- Why it matters: broad inbound access is a common root cause for accidental exposure
- Needed data: existing ingress inspection
- Best pivots: Security Groups view, CLI describe-security-groups, console

### Target Groups

#### Unhealthy targets present

- Category: `Incident`
- Why it matters: partial outages and degraded deployments should be obvious from the findings layer
- Needed data: existing target health data
- Best pivots: Target Groups view, CLI, console

### Secrets Manager

#### Rotation disabled

- Category: `Hygiene`
- Why it matters: static secrets drift toward long-lived risk
- Needed data: existing secret summary and secret list
- Best pivots: Secrets view, CLI describe-secret, console

## Findings Requiring Moderate Data Expansion

These are strong candidates once the team adds a little more fetch depth.

### EC2

- long-running dev or staging instances

### ELB / Target Groups

### RDS

- dev or staging databases that run continuously
- oversized classes relative to purpose or usage

### Lambda

- functions with no invocations recently
- functions with consistently short executions but large memory allocation

### SQS

- DLQ growth spikes
- queues with growing backlog over time rather than point-in-time volume only

### Security Groups

- duplicate rules
- overly permissive egress
- public ingress without obvious justification

## Findings Requiring New Service Coverage

These are especially valuable for the waste-catalog direction and probably deserve their own service implementation work.

### Storage And Network Waste

- unattached EBS volumes
- unattached Elastic IPs
- old EBS snapshots
- unused AMIs
- orphan ENIs

### Load And Edge Waste

- idle load balancers
- NAT gateways with suspiciously high spend or low value
- CloudFront distributions with questionable ownership or age

### Data And Log Hygiene

- S3 buckets without lifecycle rules
- CloudWatch log groups with no retention or overly long retention
- EFS filesystems with unclear ownership or likely drift

### Compute And Platform Drift

- ECS services with desired count `0` but lingering infrastructure
- ECR repositories with stale images
- Auto Scaling groups with drift between desired and healthy capacity
- EventBridge rules targeting outdated or missing resources

## Recommended Next Batch

If the team wants the highest signal with the least new plumbing, implement these next:

1. EC2 long-running dev or staging instances

## Implementation Guidance

When adding a new finding:

1. prefer using already-fetched data before adding deeper AWS calls
2. keep heuristics explainable to the operator
3. avoid scoring everything as urgent
4. always pair the finding with a next-step pivot
5. document thresholds and heuristics when they are non-obvious

Current implemented thresholds that should stay explainable:

- EC2 low-CPU waste review when a running instance averages below `5%` CPU utilization over the last `7` days
- SQS backlog incident when a queue has `>= 100` visible messages
- SQS backlog incident when a queue has `>= 50` in-flight messages
- Secrets stale-rotation review when a secret has rotation enabled but `last_rotated` is at least `180` days old

Current implemented heuristics that should stay explainable:

- RDS resilience review when an instance is available, single-AZ, and its identifier contains a production-like hint such as `prod`, `production`, `live`, `critical`, `primary`, `main`, or `customer`
- API Gateway review when an API name is generic like `unnamed`, `default`, `test`, `example`, `sample`, `temp`, `tmp`, `my-api`, or `api`, or when its creation date is at least `365` days old
- Secrets review when rotation is disabled and the secret name contains a production-like hint such as `prod`, `production`, `live`, `critical`, `primary`, `main`, or `customer`
- CloudWatch coverage-gap review when deployed inventories exist for `EC2`, `Lambda`, `RDS`, `ECS`, `API Gateway`, or `SQS` but no alarms are present in the corresponding AWS namespace
- EC2 tag-coverage review when an instance is missing any of the `Name`, `Owner`, or `Environment` tags
- Target group orphan review when a target group has no attached load balancer and zero registered targets
- Load balancer no-active-target-path review when a load balancer has no attached target groups or zero registered targets behind them
- Load balancer incident review when registered targets exist but healthy targets total zero

## Future Direction

Over time, the findings layer should likely support:

- filtering by category
- filtering by severity
- grouping by service
- grouping by region
- suppression or snoozing
- ownership and tag-aware prioritization
- trend-aware findings instead of point-in-time-only findings
