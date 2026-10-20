# Seamless Glance Release Process

Releases are automated with **Changesets** + **GitHub Actions**. The day-to-day
flow is just: *add a changeset, merge the version PR.* Everything after that
(building binaries for every platform, publishing the GitHub Release, updating
the Homebrew tap, and publishing to crates.io) happens in CI.

---

## Source of truth

`Cargo.toml` remains the canonical version. Changesets bumps `package.json` and
writes the changelog; `scripts/sync-cargo-version.sh` copies the resolved
version into `Cargo.toml` and `Cargo.lock` automatically. Never hand-edit the
version.

---

## Normal flow (automated)

### 1. Add a changeset with each user-facing change

```bash
npm run changeset
```

Pick the bump level (patch / minor / major) and write a one-line summary. Commit
the generated `.changeset/*.md` file with your PR. Changes without a changeset
are flagged by `npx changeset status`.

### 2. Merge to `main`

On push to `main`, `.github/workflows/version.yml` runs the Changesets action:

- **Pending changesets exist** → it opens/updates a **"chore: version packages"**
  PR that bumps the version, updates `Cargo.toml`/`Cargo.lock`, and regenerates
  `CHANGELOG.md`.
- Review and merge that PR when you're ready to release.

### 3. Merge the version PR → release ships itself

Merging the version PR leaves no pending changesets, so the action's publish
step runs `scripts/tag-release.sh`, which tags `v<version>` and dispatches
`.github/workflows/release.yml`. That workflow:

1. Builds binaries for all targets:
   - `aarch64-apple-darwin`, `x86_64-apple-darwin`
   - `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`
   - `x86_64-pc-windows-msvc`
2. Creates the GitHub Release with the binaries + `SHA256SUMS.txt`.
3. Regenerates and pushes the Homebrew formula in `fells-code/homebrew-seamless`.
4. Publishes to crates.io (`cargo install` / `cargo binstall`).

The public installers (`install.sh`, `install.ps1`) resolve the latest release
dynamically, so they need no per-release edits. They refuse to install a binary
that has no matching entry in the release's `SHA256SUMS.txt`, so a release
published without that file breaks both installers.

---

## Verify a release

The release is not done until every channel serves the new version. After the
version PR merges, check each one (replace `1.2.3` with the version):

```bash
# 1. The tag exists and the Release workflow passed
git fetch --tags && git tag -l v1.2.3
gh run list --workflow release.yml --limit 1

# 2. The GitHub Release has all six binaries plus SHA256SUMS.txt
gh release view v1.2.3 --json assets --jq '.assets[].name'

# 3. The Homebrew tap serves it
brew update && brew info fells-code/seamless/seamless-glance

# 4. crates.io serves it
cargo search seamless-glance --limit 1
```

Then install from at least one channel and confirm `glance --version` prints
the new version.

---

## If a release fails

- **A build or the GitHub Release job failed:** rerun only the failed jobs with
  `gh run rerun <run-id> --failed`. If the run is gone or the fix needed a code
  change on `main`, start the workflow again for the same tag with
  `gh workflow run release.yml -f version=v1.2.3`. The release job updates an
  existing GitHub Release rather than failing on it.
- **Only the crates.io job failed:** the binaries, GitHub Release and Homebrew
  formula still shipped, because that job is `continue-on-error`. Check
  `CARGO_REGISTRY_TOKEN`, then rerun that job.
- **The tag was never created:** the version PR merged but `tag-release.sh` did
  not run. Rerun the latest `version.yml` run on `main`, or use the manual
  fallback below.
- **The tag exists but no Release run started:** rerunning `version.yml` will
  not help, because `tag-release.sh` exits early when the tag already exists.
  Dispatch the build directly with `gh workflow run release.yml -f version=v1.2.3`.

Never delete and recreate a tag that has a published GitHub Release, and never
reuse a version number. Fix forward with a new patch release.

---

## One-time setup

- The **`seamless-glance` repo must be public** (release binaries must be
  publicly downloadable).
- Repo secrets:
  - `HOMEBREW_TAP_TOKEN`: a PAT with write access to `fells-code/homebrew-seamless`.
  - `CARGO_REGISTRY_TOKEN`: a crates.io token (the crates job is
    `continue-on-error`, so the binary release still succeeds without it).
- The `seamless-glance` name must be claimed on crates.io (or adjust
  `[package]` / docs if a different name is used).

---

## Manual fallback

If you need to cut a release locally (CI down, or a one-off):

```bash
# 1. Bump the version via changesets (preferred) or edit Cargo.toml directly.
# 2. Build local artifacts + regenerate the Homebrew formula:
make release-local                 # builds macOS + Linux-x86_64 into dist/
./scripts/release-helper.sh --skip-build   # writes SHA256SUMS-based formula + manifest
# 3. Tag and let the release workflow build the full matrix:
git tag v<version> && git push origin v<version>
```

`release-helper.sh` regenerates the formula from `dist/SHA256SUMS.txt`; it only
includes the platforms present locally (macOS + Linux-x86_64 via `make`), so
prefer the tag-driven CI build for the complete matrix.

---

## Rules

- One changeset per user-facing change; let the version PR own the bump.
- Do not reuse or retag versions.
- Do not hand-edit the version in `Cargo.toml`, CI, or the formula.
- Do not modify release binaries after upload.
