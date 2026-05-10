# AWS Service Roadmap

## Purpose

This document ranks the next AWS services to add to Seamless Glance.

Ranking is based on:

1. triage value
2. waste-detection value
3. operator familiarity and broad usefulness
4. ability to produce meaningful CLI and console pivots

The product does not need every AWS service at once. It needs the services that most improve operator understanding of an account and most improve the findings backlog.

## Already Implemented

Current service coverage includes:

- CloudWatch
- EC2
- ECS
- Lambda
- API Gateway
- RDS
- SQS
- VPC
- Secrets Manager
- Load Balancers
- Target Groups
- Security Groups
- cost data

## Tier 1: Highest Value Next

These are the best next services because they strongly support the waste-catalog and triage goals.

### 1. EBS

- Why first: unattached volumes and old snapshots are classic AWS waste
- High-value findings:
  - unattached EBS volumes
  - oversized volumes
  - stale snapshots
- Good pivots:
  - `aws ec2 describe-volumes`
  - `aws ec2 describe-snapshots`

### 2. Elastic IPs

- Why first: unattached EIPs are easy to understand and easy to waste money on
- High-value findings:
  - unattached Elastic IPs
  - EIPs attached to stopped instances
- Good pivots:
  - `aws ec2 describe-addresses`

### 3. CloudWatch Logs

- Why first: retention and log sprawl are common blind spots
- High-value findings:
  - log groups with no retention
  - unexpectedly high-retention groups
  - stale log groups
- Good pivots:
  - `aws logs describe-log-groups`

### 4. S3

- Why first: nearly every AWS account uses S3, and storage lifecycle hygiene matters
- High-value findings:
  - buckets without lifecycle rules
  - buckets without obvious ownership tagging
  - buckets with public exposure concerns
- Good pivots:
  - `aws s3api list-buckets`
  - `aws s3api get-bucket-lifecycle-configuration`

### 5. IAM

- Why first: understanding the account means understanding identity risk
- High-value findings:
  - old access keys
  - unused users
  - excessive policy attachment
- Good pivots:
  - `aws iam list-users`
  - `aws iam list-access-keys`

### 6. ECR

- Why first: image sprawl and stale repositories are common cleanup targets
- High-value findings:
  - stale images
  - repositories with no pulls or old pushes
  - untagged image buildup
- Good pivots:
  - `aws ecr describe-repositories`
  - `aws ecr describe-images`

## Tier 2: Strong Operational Value

These services become especially useful once Tier 1 is in place.

### 7. Auto Scaling

- Helps explain EC2 fleet behavior
- Findings:
  - desired vs healthy mismatch
  - instances lingering outside expected scaling behavior

### 8. ElastiCache

- Common spend source
- Findings:
  - underutilized clusters
  - dev caches left running
  - single-node resilience issues

### 9. CloudWatch Log Insights / richer logs support

- Not a separate AWS service in the same sense, but operationally valuable
- Findings:
  - noisy or fast-growing log groups
  - services with suspicious recent error spikes

### 10. NAT Gateways

- High waste potential in many accounts
- Findings:
  - expensive NAT footprint for low-complexity environments
  - NATs in places that look dev or abandoned

### 11. Route 53

- Important for mapping application surfaces
- Findings:
  - stale records
  - risky public exposure patterns

### 12. SNS

- Helpful for notification topology and incident flow understanding
- Findings:
  - unused topics
  - broken subscription footprints

## Tier 3: Platform And Architecture Understanding

These deepen understanding of the cloud estate, but often after the highest-value waste and hygiene wins.

### 13. DynamoDB

- Findings:
  - underused tables
  - overprovisioned throughput if provisioned mode is used
  - old unused dev tables

### 14. Step Functions

- Findings:
  - stale workflows
  - repeatedly failing state machines

### 15. EventBridge

- Findings:
  - stale rules
  - targets pointing at old or missing resources

### 16. EFS

- Findings:
  - low-value or abandoned filesystems
  - mount targets without clear ownership

### 17. SSM

- Findings:
  - managed instances lacking patching posture
  - inconsistent session-manager readiness

### 18. CloudFront

- Findings:
  - stale distributions
  - low-value edge footprint

## Tier 4: Specialized Or Later-Phase Coverage

These are valuable, but usually after broader triage visibility is in place.

### 19. EKS

- High importance in some environments, but high complexity
- Better once the app’s service and findings model is more mature

### 20. WAF

- Good security context, but not always present

### 21. Backup

- Strong governance value
- Better once core compute/storage visibility is stronger

### 22. OpenSearch

- Useful in some stacks, but less universal

### 23. Redshift

- High spend potential, but less common than S3 or EBS

### 24. Kinesis / MSK

- Important in some event-heavy stacks
- Usually not the first place to expand for broad account visibility

## Recommended Service Delivery Order

If the team is implementing one service at a time, the preferred next sequence is:

1. EBS
2. Elastic IPs
3. CloudWatch Logs
4. S3
5. IAM
6. ECR
7. Auto Scaling
8. ElastiCache
9. NAT Gateways
10. Route 53

## Implementation Notes

When adding a new service:

1. add at least one high-value finding with it
2. add CLI and console pivots at the same time
3. avoid shipping a service as inventory-only unless it unlocks obvious later value
4. document why the service is ranked where it is

## Navigation Implications

As more services are added, fixed numeric shortcuts become less sustainable.

Service expansion should therefore reinforce:

- command-palette-first navigation
- findings-first workflows
- grouped service browsing
- fewer hardcoded one-key bindings for individual service destinations
