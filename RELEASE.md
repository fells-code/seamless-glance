# Seamless Glance — Release Process

This document describes the **exact, required steps** to create a Seamless Glance release.
Follow this process **to the letter** to avoid version mismatches or broken installs.

---

## Source of Truth

**The version in `Cargo.toml` is the canonical version.**

Everything else must match it:

- Git tag
- Binary filenames
- GitHub release
- Homebrew formula
- `install.sh`

Do not infer versions from commits or CI variables.

---

## Release Checklist (High Level)

1. Bump version in `Cargo.toml`
2. Run the local release helper
3. Review the changed support repos
4. Commit the version bump and synced release files
5. Publish binaries to the distro repo
6. Verify installation via Homebrew and curl
7. Publish release notes

---

## Step-by-Step Instructions

### 1. Bump Version in `Cargo.toml`

Edit `Cargo.toml`:

```toml
[package]
name = "seamless-glance"
version = "0.1.0-beta.X"
```

Rules:

- Use semver
- Use `-beta.X` while in beta
- This is the **only place** the version is defined

---

### 2. Run The Local Release Helper

After you manually bump the version, run:

```bash
./scripts/release-helper.sh
```

Or, if you prefer:

```bash
make release-helper
```

What the helper does:

- reads the current version from `Cargo.toml`
- builds release artifacts into `dist/`
- generates `SHA256SUMS.txt`
- writes `dist/release-manifest.json`
- updates:
  - `../seamless-glance-distro/install.sh`
  - `../homebrew-seamless/Formula/seamless-glance.rb`

What it intentionally does **not** do:

- bump the version for you
- create Git tags
- make Git commits
- push to GitHub
- publish GitHub releases

Safety behavior:

- refuses to edit dirty support repos unless you pass `--allow-dirty`
- supports `--dry-run`
- supports path overrides if your sibling checkouts live elsewhere

---

### 3. Review And Commit The Synced Files

Review the source repo and support repo diffs, then commit them manually in the repos you want:

```bash
git status --short
git -C ../seamless-glance-distro status --short
git -C ../homebrew-seamless status --short
```

---

### 4. Create A Git Tag (Recommended)

Create a tag that matches the Cargo version exactly:

```bash
git tag v0.1.0-beta.X
git push origin v0.1.0-beta.X
```

Do not reuse or retag existing versions.

---

### 5. Publish Binaries To `seamless-glance-distro`

In the **distro repository**:

1. Create a new GitHub release:

   - Tag: `v0.1.0-beta.X`
   - Title: `Seamless Glance v0.1.0-beta.X`

2. Upload binaries with **exact filenames**:

```text
seamless-glance-0.1.0-beta.X-aarch64-apple-darwin
seamless-glance-0.1.0-beta.X-x86_64-apple-darwin
seamless-glance-0.1.0-beta.X-x86_64-unknown-linux-gnu
```

3. Upload `SHA256SUMS.txt` (if available)

Do not overwrite existing assets.

---

### 6. Verify Installation (Required)

Before announcing the release, verify the install path:

#### Homebrew

```bash
brew update
brew upgrade seamless-glance
seamless-glance --version
```

This must report the **new version**.

---

### 7. Publish Release Notes

Use the following sections in GitHub Releases:

- Features
- Fixes
- Chores

Ensure notes reflect actual changes since the last release.

---

## Important Rules (Do Not Skip)

- Do not reuse version numbers
- Do not modify binaries after upload
- Do not change Cargo version in CI
- Do not release without verifying installs
- Do not assume Homebrew auto-upgrades existing installs

---

## Troubleshooting

### Binary reports old version

- Verify `Cargo.toml`
- Verify build artifact
- Verify Homebrew symlink
- Check for local `make install` overrides
- Re-run `./scripts/release-helper.sh --skip-build` if the support repos drifted after the artifact build

### Homebrew installs wrong version

- Run:
  ```bash
  brew uninstall --force seamless-glance
  brew update
  brew install fells-code/seamless/seamless-glance
  brew link --overwrite seamless-glance
  ```

---

## Philosophy

This process prioritizes:

- correctness
- repeatability
- clarity over automation

Automation can be added later once release cadence stabilizes.

---

End of document.
