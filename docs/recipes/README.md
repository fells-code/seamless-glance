# Recipes

Step-by-step guides for the two changes contributors make most often. Each
recipe points at a merged pull request you can read alongside it, so you can
see a real, reviewed version of every step.

- [Add a finding rule](add-a-finding.md): a new check over data Glance already
  fetches. Start here if you are new to the codebase.
- [Add a service view](add-a-service.md): a new AWS service with its own list
  view, describe overlay, and console and CLI pivots.

## Before you start any issue

1. Read the whole issue, including "Out of scope". If something in it is
   unclear, ask in a comment on the issue before writing code.
2. Pull `main` and branch from it: `git checkout main && git pull && git
   checkout -b feat/<short-name>`. Use `feat/`, `fix/`, `docs/`, or `test/` to
   match the change.
3. Check that `cargo test --all` passes before you change anything, so you know
   any later failure is yours.

## Before you open the pull request

Run all three, and fix everything they report:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Then:

- Add a changeset with `npm run changeset` if a user would notice the change
  (new view, new finding, changed behavior). Pick `minor` for a new view or
  finding and `patch` for a fix. Skip it for docs-only or test-only changes.
- Update the docs the issue lists. Documentation is part of done here (see
  `AGENTS.md`).
- Write the PR title as a Conventional Commit, for example
  `feat: flag dead-letter queues that hold messages`. In the PR description,
  write `Closes #<issue>` and paste the output of the three commands above.
- Do not use em dashes anywhere (code comments, docs, commits, PR text). Use a
  comma, parentheses, or a new sentence.
- Never paste real AWS account IDs, ARNs, hostnames, or secret values into
  code, tests, issues, or PRs. Use placeholders such as `123456789012` and
  `example.com`.

## Checking against real AWS

Unit tests are the main way to prove a change works. When the issue asks for a
live check, run Glance with the read-only profile you were given:

```bash
cargo run -- --profile <your-read-only-profile>
```

The profile can list and describe resources but cannot change anything or read
secret values. If a view shows an access-denied status, say so in the PR rather
than working around it. That is useful information, not a failure.
