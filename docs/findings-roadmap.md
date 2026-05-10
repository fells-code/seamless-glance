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

- CloudWatch alarms in `ALARM`
- target groups with zero healthy targets
- unhealthy target groups
- SQS queues without DLQs
- secrets without rotation
- stopped EC2 instances
- security groups open to the world
- security groups exposing sensitive ports publicly

These are a foundation, not the target end state.

## Near-Term Findings

These are the best next additions because they fit the current architecture and the data is mostly already present or easy to add.

### CloudWatch

#### Alarms in ALARM by resource and age

- Category: `Incident`
- Why it matters: a count is useful, but the operator needs the actual failing alarms first
- Needed data: alarm name, state, last state update if available, related metric context
- Best pivots: CloudWatch view, CLI describe-alarms, console link

### EC2

#### Stopped instances with likely waste signal

- Category: `Waste`
- Why it matters: stopped compute often indicates abandoned dev or old experiments
- Needed data: existing instance list, plus optional age or tags later
- Best pivots: EC2 view, CLI describe-instances, console link

#### Stopped instances with public IP or production-like naming

- Category: `Hygiene` or `Waste`
- Why it matters: a stopped resource that still looks externally relevant or production-scoped should be reviewed more carefully
- Needed data: instance tags, public IP, name pattern
- Best pivots: EC2 view, CLI, console

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

#### Rotation disabled on likely production secrets

- Category: `Hygiene`
- Suggested severity: `High`
- Why it matters: the same problem matters more when the secret appears production-scoped
- Needed data: naming or tag heuristics
- Best pivots: Secrets view, CLI, console

### SQS

#### High visible or in-flight messages

- Category: `Incident`
- Why it matters: backlog and stuck work are operationally meaningful
- Needed data: existing message counters plus thresholds
- Best pivots: SQS view, CLI, console

### RDS

#### Instance not `available`

- Category: `Incident`
- Why it matters: database availability issues deserve first-class visibility
- Needed data: existing status field
- Best pivots: RDS view, CLI describe-db-instances, console

#### Single-AZ database that appears production-like

- Category: `Hygiene`
- Why it matters: resilience posture may be weaker than the workload suggests
- Needed data: existing `multi_az` plus tag or naming heuristics
- Best pivots: RDS view, CLI, console

### Lambda

#### Suspiciously high memory allocation

- Category: `Waste`
- Why it matters: high configured memory often signals avoidable cost
- Needed data: existing memory setting plus thresholding
- Best pivots: Lambda view, CLI get-function, console

#### Very old last-modified functions

- Category: `Waste` or `Hygiene`
- Why it matters: stale functions may be abandoned or under-owned
- Needed data: existing last modified timestamp plus aging heuristic
- Best pivots: Lambda view, CLI, console

### VPC

#### Default VPC still present

- Category: `Hygiene`
- Why it matters: default-network usage often correlates with low-governance deployments
- Needed data: existing `is_default`
- Best pivots: VPC view, CLI describe-vpcs, console

### API Gateway

#### Generic or stale APIs

- Category: `Waste` or `Hygiene`
- Why it matters: unnamed or untouched APIs are often drift or leftovers
- Needed data: existing name and created date, plus simple heuristics
- Best pivots: API Gateway view, CLI, console

## Findings Requiring Moderate Data Expansion

These are strong candidates once the team adds a little more fetch depth.

### EC2

- low CPU for sustained periods
- missing `Name`, `Owner`, or `Environment` tags
- long-running dev or staging instances

### ELB / Target Groups

- load balancer with no active or healthy targets
- orphan target groups with no meaningful attachment

### RDS

- dev or staging databases that run continuously
- oversized classes relative to purpose or usage

### Lambda

- functions with no invocations recently
- functions with consistently short executions but large memory allocation

### SQS

- DLQ growth spikes
- queues with growing backlog over time rather than point-in-time volume only

### CloudWatch

- important services with no alarms at all
- alarm coverage gaps for deployed workloads

### Secrets Manager

- secrets not rotated for an unacceptably long time even when rotation is enabled

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

1. RDS instances not `available`
2. Lambda functions with suspiciously high memory or very old deploy dates
3. default VPC present
4. CloudWatch findings expanded from counts to named failing alarms

## Implementation Guidance

When adding a new finding:

1. prefer using already-fetched data before adding deeper AWS calls
2. keep heuristics explainable to the operator
3. avoid scoring everything as urgent
4. always pair the finding with a next-step pivot
5. document thresholds and heuristics when they are non-obvious

## Future Direction

Over time, the findings layer should likely support:

- filtering by category
- filtering by severity
- grouping by service
- grouping by region
- suppression or snoozing
- ownership and tag-aware prioritization
- trend-aware findings instead of point-in-time-only findings
