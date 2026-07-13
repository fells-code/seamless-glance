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
dynamically, so they need no per-release edits.

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
