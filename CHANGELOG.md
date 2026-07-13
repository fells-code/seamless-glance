# seamless-glance

## 1.1.0

### Minor Changes

- 2c0d863: Add Changesets-driven versioning, automated GitHub releases, and cross-platform
  installation. Merging the version PR now builds binaries for macOS (arm64/x86_64),
  Linux (x86_64/aarch64, glibc + static musl), and Windows, publishes them to the
  GitHub Release with checksums, updates the Homebrew tap, and publishes to
  crates.io (`cargo install` / `cargo binstall`). New `install.sh` and `install.ps1`
  resolve the latest release automatically.
