# Seamless Glance

Terminal-native AWS visibility for operators who want fast account and service context without living in the AWS console.

The product is currently evolving from a general service inventory viewer toward a triage accelerator that highlights wasted resources and gives fast pivots into the AWS CLI and AWS console.

## What It Does

Seamless Glance is a Rust TUI that connects to AWS using the standard SDK credential chain and presents:

- account overview data
- monthly cost and service cost breakdowns
- region-scoped and selected global service inventories
- drill-down views for common AWS services
- console deep links, resource describe overlays, and EC2 SSH helpers

The long-term emphasis is:

- surface what needs attention first
- catalog waste and low-value resources
- make next-step operator actions fast

The navigation model is also expected to evolve over time:

- Findings is the operator-centric home
- the command palette is the scalable way to reach services
- fixed numeric shortcuts are a transitional compatibility layer, not the long-term destination model

Current first-class views:

- Findings
- Account Overview
- Cost Overview
- Cost Savings
- VPC
- EC2
- CloudWatch
- Lambda
- Secrets Manager
- ECS
- API Gateway
- RDS
- SQS
- Load Balancers
- Target Groups
- Security Groups

## Requirements

- Rust toolchain for local development
- valid AWS credentials available to the AWS SDK
- a Seamless Glance license file at `~/.seamless-glance/license.json`, or a first run that can create a trial license there

On first run, the app can create a local trial license automatically. Paid licenses are validated at startup.

## Run Locally

```bash
cargo run
```

Useful CLI options:

```bash
cargo run -- --help
cargo run -- --version
cargo run -- --license-status
```

## Controls

Primary navigation:

- `f` Findings
- `Tab` / `Shift+Tab` cycle through major views
- `/` open the command palette with grouped view suggestions and aliases
- `t` cycle through the Seamless theme set
- `1` Account Overview
- `2` Cost Overview
- `0` Cost Savings
- `3` VPC
- `4` EC2
- `5` CloudWatch
- `6` Lambda
- `7` Secrets Manager
- `8` ECS
- `9` API Gateway

General controls:

- `←` / `→` change region
- `↑` / `↓` move selection or scroll overlays
- `PgUp` / `PgDn` jump-scroll lists, overlays, and help
- `Home` / `End` jump to the top or bottom of long views
- `?` open help
- `r` refresh active view
- `d` describe selected resource
- `v` toggle Describe between structured and JSON views
- `c` show the AWS CLI command for the selected resource
- `o` open selected resource in the AWS console
- `g` jump to the global region slot
- `s` prepare an SSH command for the selected EC2 instance
- `q` quit

View transitions now show a loading overlay while the next screen refreshes, instead of blocking silently during fetches.
The app now opens on Findings by default so triage is the first thing you see.
Account Overview now serves as an inventory snapshot for the current AWS profile, with account context, footprint summaries, and a service inventory table rather than findings-style callouts.
Cost Overview now includes budget, forecast, and usage-aware service cost context from the local billing cache.
Cost Savings is a dedicated recommendation screen that combines spend, usage types, and waste-oriented findings into savings opportunities.

Resource actions are expected to target the selected resource directly. In global-capable views, actions should use the resource's own region rather than the UI's fallback region.

Current resource-action model:

- `d` describe the selected resource in-app
- `v` toggle a describe overlay between a structured readable view and a JSON-oriented view
- `c` show and optionally run the AWS CLI command for the selected resource
- `o` open the selected resource in the AWS console

Findings view behavior:

- `Enter` opens the related service view for the selected finding
- the initial finding set includes named CloudWatch alarms in `ALARM`, CloudWatch coverage gaps for deployed services without matching alarm namespaces, running EC2 instances averaging below 5 percent CPU over the last 7 days, stopped EC2 instances, stopped EC2 instances with public IPs or production-like names, EC2 instances missing `Name`, `Owner`, or `Environment` tags, API Gateway APIs with generic names or age over one year, SQS queues with high visible or in-flight message counts, RDS instances that are not available, production-like single-AZ RDS instances, production-like secrets without rotation, secrets with stale rotation despite rotation being enabled, Lambda functions with high memory or stale deploy dates, default VPCs still present, secrets without rotation, target groups with zero healthy targets, target groups with unhealthy targets, target groups with no load balancer attachment and no registered targets, load balancers with no active target path, load balancers with zero healthy targets, SQS queues without DLQs, security groups open to the world, and security groups exposing sensitive ports publicly

Command palette shortcuts currently include:

- `findings`
- `account`
- `savings`
- `ecs`
- `ec2`
- `rds`
- `cost`
- `lambda`
- `apigw`
- `sqs`
- `vpc`
- `cw`
- `sm`
- `lb`
- `tg`
- `sg`
- aliases such as `overview`, `billing`, `api`, `queues`, `secrets`, and `alarms`
- `theme <name>` with `autumn`, `winter`, `summer`, `spring`, or `developer`
- `region <name>`
- `rg <name>`

## Global View Notes

The special `global` region slot is currently implemented for:

- EC2
- Lambda
- RDS

Other views remain region-scoped today.

## Project Structure

- [`src/main.rs`](/Users/brandoncorbett/git/seamless-glance/src/main.rs) owns process startup, license gating, terminal setup, and key handling.
- [`src/app/mod.rs`](/Users/brandoncorbett/git/seamless-glance/src/app/mod.rs) owns application state, refresh orchestration, view transitions, overlays, and resource actions.
- [`src/aws/`](/Users/brandoncorbett/git/seamless-glance/src/aws) contains AWS service fetchers and SDK client wiring.
- [`src/models/`](/Users/brandoncorbett/git/seamless-glance/src/models) defines data models rendered by the UI.
- [`src/ui/`](/Users/brandoncorbett/git/seamless-glance/src/ui) contains rendering, overlays, command palette, and terminal helpers.
- [`src/license/`](/Users/brandoncorbett/git/seamless-glance/src/license) handles trial creation, paid license loading, and signature validation.
- [`src/cache/`](/Users/brandoncorbett/git/seamless-glance/src/cache) currently caches cost data.

Longer-form references:

- [Architecture](docs/architecture.md)
- [Development Guide](docs/development.md)
- [Findings Roadmap](docs/findings-roadmap.md)
- [AWS Service Roadmap](docs/aws-service-roadmap.md)
- [Navigation Strategy](docs/navigation-strategy.md)
- [Release Process](RELEASE.md)
- [Agent Guidance](AGENTS.md)

## Build And Quality Commands

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
make build
make release-local
```

There is currently no meaningful committed test suite under `tests/`, so `cargo test` is mostly a safety net for future additions.

## Documentation Policy

This repository treats documentation as part of the product:

- update `README.md` when user-facing behavior changes
- update `docs/architecture.md` when modules, flows, or ownership change
- update `docs/development.md` when the workflow or expectations change
- update `AGENTS.md` whenever feature scope, team goals, or maintenance rules change

If a feature ships without its docs, the change is incomplete.
