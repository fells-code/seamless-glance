# Development Guide

## Working Agreement

This repository prefers targeted, maintainable changes over sweeping rewrites. Before editing:

1. inspect the relevant module path
2. check `git status --short`
3. avoid reverting unrelated local edits
4. decide whether the change affects user docs, architecture docs, or release docs

Current product direction should shape implementation choices:

- prioritize triage workflows over passive browsing
- prioritize wasted-resource detection over broader but shallower coverage
- prioritize exact CLI/console pivots over generic debug output

## Local Commands

Core development commands:

```bash
cargo run
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Packaging and release-oriented commands:

```bash
make build
make dist
make checksums
make release-local
make release-helper
```

## Credentials And Local State

The app relies on:

- AWS credentials resolved by the AWS Rust SDK
- `~/.seamless-glance/config.json` for persisted region selection
- `~/.seamless-glance/config.json` for persisted theme selection
- `~/.seamless-glance/license.json` for trial or paid license state

Be careful when changing startup behavior because licensing, config, and AWS region selection all happen very early in process startup.

## Coding Patterns

### App-first orchestration

Most interactive behavior is coordinated from [`src/app/mod.rs`](/Users/brandoncorbett/git/seamless-glance/src/app/mod.rs). If a change affects refreshes, selected rows, overlays, or region behavior, inspect that file first.
This is also where cost-savings opportunities are derived from the combination of cached billing insight and live AWS resource data.

### Service fetchers return UI-ready data

The `src/aws/` modules generally map AWS responses into model structs that are ready for display. Keep UI formatting concerns out of fetchers where possible, but include enough structure that the UI layer stays simple.
Cost Explorer is a slight exception: richer aggregation in `src/aws/cost.rs` is intentional so usage-type summaries can be cached and reused by both Cost Overview and Cost Savings.

### Terminal UX matters

The app is intentionally keyboard-driven. New behavior should:

- keep shortcuts predictable
- avoid unnecessary modal friction
- surface access errors clearly
- support fast scanability in narrow terminals
- keep navigation metadata centralized so help, command palette, and footer cues do not drift

For roadmap work, also ask:

- does this reduce time-to-triage?
- does this help identify waste or low-value spend?
- does this make the next operator action obvious?

When adding or changing resource actions, prefer this ladder:

- in-app describe for quick context
- AWS CLI command for precise operator handoff
- AWS console pivot for visual follow-up

When adding or changing cost-oriented features, prefer this ladder:

- improve cached factual billing data first
- derive explainable savings heuristics from cost + usage + findings
- surface recommendations in a way that routes back into actionable service screens

## Validation Expectations

Run the strongest checks that fit the change. Minimum expectations:

- docs-only changes: read for consistency and accuracy
- Rust changes: `cargo fmt`, `cargo clippy`, and `cargo test` when feasible
- packaging changes: relevant `make` targets
- multi-repo release sync changes: `./scripts/release-helper.sh --dry-run` plus a careful diff review

If you skip a validation step, note why.

## Documentation Maintenance

Documentation updates are required when behavior changes.

Use this mapping:

- update `README.md` for user-visible features, controls, setup, or limitations
- update `docs/architecture.md` for module or runtime flow changes
- update `AGENTS.md` for team rules, goals, or agent workflow expectations
- update `RELEASE.md` for shipping, packaging, or distribution process changes

This rule is especially important for:

- adding a new AWS service
- adding or reprioritizing findings
- changing keyboard shortcuts
- changing region or global aggregation behavior
- changing license handling
- changing release packaging

Recommended roadmap docs for product planning:

- `docs/findings-roadmap.md`
- `docs/aws-service-roadmap.md`
- `docs/navigation-strategy.md`

## Known Realities

- the repo currently has minimal automated test coverage
- the `tests/` directory is present but effectively empty
- some existing docs can drift if they are not updated alongside code

Contributors should leave the repository a little clearer than they found it, even for small changes.
