#!/usr/bin/env bash
# Sync the version resolved by `changeset version` (in package.json) into
# Cargo.toml, which is the build source of truth for Seamless Glance.
#
# Run automatically by `npm run version-packages` (see package.json). Safe to run
# manually if the two files ever drift.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TOML="$ROOT_DIR/Cargo.toml"
PKG_JSON="$ROOT_DIR/package.json"

require() {
  [[ -f "$1" ]] || { echo "Missing $2: $1" >&2; exit 1; }
}

require "$CARGO_TOML" "Cargo.toml"
require "$PKG_JSON" "package.json"

# Prefer node (present in the release toolchain); fall back to a grep parse.
if command -v node >/dev/null 2>&1; then
  VERSION="$(node -p "require('$PKG_JSON').version")"
else
  VERSION="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$PKG_JSON" | head -n1)"
fi

if [[ -z "${VERSION:-}" ]]; then
  echo "Could not read version from package.json" >&2
  exit 1
fi

# The only line-anchored `version = "..."` in Cargo.toml is the [package] one;
# dependency versions are inline (e.g. `tokio = { version = "1" }`).
VERSION="$VERSION" perl -0pi -e 's/^version = ".*?"$/qq{version = "$ENV{VERSION}"}/me;' "$CARGO_TOML"

echo "Cargo.toml version set to $VERSION"

# Keep Cargo.lock in step with the manifest so the release build is reproducible.
if command -v cargo >/dev/null 2>&1; then
  cargo update -p seamless-glance --precise "$VERSION" >/dev/null 2>&1 \
    || cargo update -p seamless-glance >/dev/null 2>&1 \
    || echo "warning: could not refresh Cargo.lock automatically; run 'cargo update -p seamless-glance'" >&2
  echo "Cargo.lock refreshed for seamless-glance@$VERSION"
else
  echo "warning: cargo not found; Cargo.lock not refreshed" >&2
fi
