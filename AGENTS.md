# AGENTS.md

This file is the operating guide for engineers and coding agents working in this repository.

## Working Standards (fells-code baseline)

These rules apply to every repository in the fells-code org. Repo-specific
guidance may extend them but must not contradict them.

### Attribution
- Commit and open PRs solely under the repository owner's identity. Never
  commit under an agent or assistant identity.
- Never attribute work to an AI assistant: no `Co-Authored-By: Claude` (or any
  assistant) trailers, no "Generated with" / "Created with Claude" notes, and no
  assistant branding or emoji anywhere in commit messages, PR or issue titles
  and descriptions, changesets, code comments, or docs.

### Comments
- Comment only when the code genuinely needs explaining: a non-obvious reason, a
  gotcha, or an invariant. Never narrate what the code plainly does.

### TODOs
- Every `TODO`/`FIXME` must reference a ticket, e.g. `// TODO(#123): ...`.
  Do not leave a bare TODO. If no ticket exists, create one first.

### Commits & branches
- Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `ci:`, `test:`).
- Descriptive branch names (`feat/...`, `fix/...`); never a `claude/` or other
  tool-generated prefix.

### Public-facing text
- No em dashes in commit messages, code comments, PR or issue text, changesets,
  or docs. Use a comma, parentheses, or a separate sentence.

### Privacy (public repository)
- This is a public repository. Everything committed or posted here is visible to
  anyone: commits, PRs, issues, comments, changesets, code, and docs.
- Never include the owner's or any individual's private or personal information.
  That includes personal email addresses, real names used as identifying detail,
  home or physical addresses, phone numbers, and any similar personal contact
  detail.
- Never include account-specific or credential-adjacent secrets: AWS account IDs,
  ARNs tied to a real account, access keys or tokens, profile names that reveal a
  client or person, internal hostnames, or private URLs and endpoints.
- When an example needs a value, use a placeholder (`123456789012`,
  `my-profile`, `example.com`) rather than a real one. When referencing repo
  files or code, use relative paths, not absolute paths that leak a local
  username or directory layout.

### Before declaring work done
- Run the checks that apply to the change (see Validation Expectations below:
  `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`) and report the real
  output. Do not open a PR while a relevant check is failing. If a step cannot
  be run, say so clearly.
- Match the surrounding code's style, naming, and comment density.

## Mission

Build and maintain Seamless Glance as a fast, terminal-native AWS operations dashboard with a polished TUI, clear operator workflows, and accurate service visibility.

Near-term product direction:

- evolve from a general AWS inventory browser into a triage accelerator
- emphasize wasted-resource discovery and operator attention queues
- make every important finding a pivot point into the AWS CLI or AWS console

## Product Snapshot

Today the app provides:

- a triage-oriented Findings view
- AWS account overview data
- cost overview data with cache-backed loading
- service inventory views for ECS, EC2, RDS, Lambda, API Gateway, SQS, VPC, CloudWatch, Secrets Manager, Load Balancers, Target Groups, and Security Groups
- resource actions such as describe overlays, AWS console deep links, region switching, in-app AWS profile switching, and EC2 SSH command generation

Current gap to keep in mind:

- the codebase now has an initial findings model, but the findings backlog is still much smaller than the eventual product direction

## Core Rules

1. Keep docs in sync with the product.
2. Do not overwrite or revert unrelated user changes.
3. Prefer small, targeted edits that match the existing architecture.
4. Preserve the terminal-first operator experience. New features should feel fast, keyboard-driven, and concise.
5. Treat correctness in AWS account context and region handling as high-sensitivity areas.
6. Bias toward operator actionability over raw exhaust. The best screen is the one that helps someone decide what to do next.
7. Prefer findings, pivots, and prioritization over adding another passive inventory table.

## Documentation Is Part Of Done

When features, goals, workflows, or architectural boundaries change, update the relevant docs in the same change:

- `AGENTS.md`: team rules, feature scope, maintenance expectations, or delivery standards
- `README.md`: user-visible capabilities, setup, controls, or current limitations
- `docs/architecture.md`: module responsibilities, runtime flow, service coverage, or extension patterns
- `docs/development.md`: contributor workflow, validation steps, or coding conventions
- `RELEASE.md`: release or distribution process

If none of these files changed, explicitly confirm the change truly had no documentation impact before finishing.

## Architecture Map

- `src/main.rs`: CLI flags, terminal lifecycle, and key event loop
- `src/app/mod.rs`: central app state and orchestration
- `src/aws/`: AWS SDK clients and service-specific fetch logic
- `src/models/`: UI-facing resource and summary models
- `src/ui/`: ratatui views, overlays, footer/header, and presentation helpers
- `src/resources/`: cross-cutting helpers such as multi-region aggregation and SSH context creation
- `src/cache/`: local cache support

## Change Workflow

1. Survey the touched code paths before editing.
2. Check `git status --short` and avoid trampling in-progress local work.
3. Make the smallest reasonable change that fits the current design.
4. Update docs alongside code when behavior or goals change.
5. Add a changeset (`npm run changeset`) for any user-facing change; the release is driven from it (see `RELEASE.md`).
6. Run the strongest relevant validation available.
7. Summarize any residual risk, especially around AWS permissions, global region handling, and UI state transitions.

## Validation Expectations

Use the most relevant commands available for the change:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Also use these when release packaging is affected:

```bash
make build
make release-local
```

Current reality:

- the repository has little to no meaningful automated test coverage yet
- manual reasoning and targeted validation still matter
- if you cannot run a validation step, say so clearly

## Product Direction

Use this hierarchy when making roadmap or implementation decisions:

1. Triage accelerator
2. Waste catalog
3. Fast pivot to action
4. Broad inventory coverage

Interpretation:

- a view that surfaces the most important issues is more valuable than a broader but passive service list
- a precise CLI or console pivot is more valuable than a generic detail dump
- findings should clearly separate incident risk, waste, and hygiene

## Roadmap Priorities

Current preferred delivery order:

1. Expand the findings backlog with high-signal incident, waste, and hygiene rules.
2. Add new AWS services that directly unlock valuable findings.
3. Improve navigation so Findings and the command palette scale better than fixed numeric service shortcuts.
4. Expand profile support and multi-account operator workflows.
5. Continue tightening action precision and region-aware pivots as service coverage expands.

When in doubt, choose work that makes the app better at answering:

- What needs attention now?
- What is costing money for little value?
- What should I open or run next?

## Extending The App

When adding a new AWS service or view:

1. Add or extend the model types in `src/models/`.
2. Add the service fetcher in `src/aws/`.
3. Update `AwsClients` if a new SDK client is required.
4. Extend `App` state plus `on_view_enter`, `refresh_active`, `trigger_describe`, and `trigger_open` as needed.
5. Add or update a ratatui view in `src/ui/views/`.
6. Wire navigation and command-palette support if the feature is operator-facing.
7. Document the new capability in `README.md` and `docs/architecture.md`.

If the change is triage-related, also decide explicitly:

1. Is this an incident, waste, hygiene, or inventory feature?
2. What is the operator's next action?
3. Should the app offer a console pivot, a CLI pivot, or both?

If the change affects navigation, also decide explicitly:

1. Is the behavior meant to be long-term or transitional?
2. Does it scale as more services and findings are added?
3. Does it reinforce Findings-first and command-palette-first workflows?

## Operator Experience Standards

- Favor keyboard-first interactions.
- Keep loading and refresh behavior explicit.
- Show partial availability honestly when AWS permissions or regional APIs fail.
- Avoid hiding access-denied or unavailable states behind empty success-looking UIs.
- Prefer concise labels and stable navigation over decorative UI changes.
- Separate urgent attention from optimization opportunities.
- Make pivots land on the exact resource whenever possible.
- Prefer targeted descriptions over account-wide or list-wide dumps.
- Treat fixed numeric service shortcuts as transitional unless there is a strong reason to extend them.

## Known Constraints

- Global aggregation is currently only implemented for selected services, not every view.
- Cost data is cached separately from the main refresh loop.
- The working tree may be dirty; assume other work can be in flight.
- AWS profile switching is wired into runtime: a `--profile` flag, a `p` key and `profile` command open a picker sourced from the shared config and credentials files, and the choice is persisted. Switching a profile does not re-enumerate enabled regions, and it is a credential switch only. It is not a tenant isolation boundary, which is governed by the profile's IAM permissions and account.
- Several current resource actions are better described as prototypes than finished operator pivots.

## Maintenance Rule

This file should be updated whenever:

- a new feature meaningfully changes what agents need to know
- a project goal or operating principle changes
- the architecture shifts enough that this map becomes stale
- documentation expectations change

Do not let `AGENTS.md` become a one-time setup artifact. It should evolve with the codebase.
