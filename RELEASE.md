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
2. Commit the version bump
3. Build release binaries
4. Publish binaries to the distro repo
5. Update Homebrew formula
6. Update `install.sh`
7. Verify installation via Homebrew and curl
8. Publish release notes

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

### 2. Commit the Version Bump

Commit _only_ the version change:

```bash
git add Cargo.toml
git commit -m "chore: bump version to 0.1.0-beta.X"
git push origin main
```

---

### 3. Create a Git Tag (Recommended)

Create a tag that matches the Cargo version exactly:

```bash
git tag v0.1.0-beta.X
git push origin v0.1.0-beta.X
```

Do not reuse or retag existing versions.

---

### 4. Build Release Binaries

Build binaries for supported platforms:

```bash
make release-local
```

Ensure binaries report the correct version:

```bash
./target/release/seamless-glance --version
```

Output must match the version in `Cargo.toml`.

---

### 5. Publish Binaries to `seamless-glance-distro`

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

### 6. Update Homebrew Formula

In the Homebrew tap:

- Update:
  - `version`
  - `url`
  - `sha256` for each platform

Example:

```ruby
version "0.1.0-beta.X"
```

Ensure URLs point to the **new release assets**.

Commit and push the formula update.

---

### 7. Update `install.sh`

Update these fields:

```bash
VERSION="0.1.0-beta.X"
REPO="fells-code/seamless-glance-distro"
```

Ensure:

- URLs reference the correct release tag
- Filenames match the uploaded binaries
- Script still installs successfully

Commit and push the updated script.

---

### 8. Verify Installation (Required)

Before announcing the release, verify **both install paths**:

#### Homebrew

```bash
brew update
brew upgrade seamless-glance
seamless-glance --version
```

#### Curl Install

```bash
curl -fsSL https://seamlessglance.com/install.sh | bash
seamless-glance --version
```

Both must report the **new version**.

---

### 9. Publish Release Notes

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
