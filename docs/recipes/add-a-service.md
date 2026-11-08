# Add a service view

A service view lists one kind of AWS resource, lets the operator describe a
row, and pivots to the AWS console or CLI for the selected row.

Reference pull request: #91, which added ECR. Open its "Files changed" tab
next to this recipe. Every step below names the file it touches in #91, so you
can copy the pattern instead of inventing one.

Keep the view's first pull request to listing, describe, and pivots. Finding
rules come in their own follow-up issues.

## 1. Add the SDK crate (if needed)

If the service needs a new crate, add it to `Cargo.toml` (for example
`aws-sdk-acm = "1"`), then add a client field to `AwsClients` in
`src/aws/clients.rs` and build it where the others are built. Some services
reuse an existing client; ECS autoscaling, for example, needs
`aws-sdk-applicationautoscaling`, but Aurora clusters reuse the RDS client.

## 2. Add the model

Create `src/models/<service>.rs` and register it in `src/models/mod.rs`.

- One struct per row, named `<Thing>Info` (see `EcrRepositoryInfo`).
- Always include `pub tags: Tags`. The application view (#90) groups every
  resource by its `Application` tag, so a model without tags cannot take part.
- Implement `DescribableResource` for the struct (see the ECR model). This is
  where the console URL and the `aws ...` CLI command for the selected row come
  from. Make the CLI command target the exact resource, not a whole list.
- Put display helpers and predicates on the struct and unit test them in the
  same file.

## 3. Add the fetcher

Create `src/aws/<service>.rs` and register it in `src/aws/mod.rs`.

- Return `(Vec<Info>, ServiceStatus)`. On an SDK error, return an empty list
  with `ServiceStatus::from_sdk_error(&err)`, never a silent empty list. An
  access-denied view must look different from an empty one.
- Use the SDK paginator (`into_paginator()`) so accounts with many resources
  are fully listed.
- When each row needs an extra call, run them with bounded concurrency using
  `DESCRIBE_CONCURRENCY` (see `src/aws/sqs.rs`).
- Read tags with the helpers in `src/aws/tags.rs`.

## 4. Wire it into the app

Follow the #91 diff for each of these:

- `src/app/mod.rs`: add the `ActiveView` variant, the `Vec<Info>` and
  `ServiceStatus` fields on `App`, their initial values, the lines that clear
  them on a region or profile switch, and the `FindingRoute` mapping.
- `src/app/refresh.rs`: add a `RefreshUpdate` variant and `InventoryKind`, and
  fetch the data when the view is entered or refreshed.
- `src/app/services.rs`: register the view so filter, describe, open, and CLI
  all work.
- `src/models/finding.rs`: add a `FindingRoute` variant so later rules can
  route to the view.
- `src/ui/views/<service>.rs` and `src/ui/views/mod.rs`: render the table with
  the shared helper in `src/ui/views/list_table.rs`. Copy `src/ui/views/ecr.rs`.
- `src/ui/mod.rs`: call the render function for the new view.
- `src/ui/views/command.rs`: add a palette entry with a name, description,
  group, and a couple of aliases.
- `src/ui/keys.rs`: add the view to the test list of views.

## 5. Check it

- `cargo fmt`, `cargo clippy`, and `cargo test` all pass.
- If you have the read-only profile, open the view through the command palette
  and check the rows, the describe overlay, and the console and CLI pivots.

## 6. Document it

Update, in the same pull request:

- `README.md`: add the view to "Current first-class views".
- `docs/architecture.md`: add the service to the coverage list.
- `docs/aws-service-roadmap.md`: mark it implemented.
- `AGENTS.md`: add it to the service list in "Product Snapshot".

Then add a `minor` changeset.
