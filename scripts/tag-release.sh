#!/usr/bin/env bash
# Publish step for the Changesets flow. Runs (via `npm run release`) only when
# there are no pending changesets left, i.e. right after the "Version packages"
# PR is merged. It tags the newly-bumped version and hands off to the release
# workflow, which builds binaries and creates the GitHub Release.
#
# A tag pushed with the default GITHUB_TOKEN does NOT trigger other workflows,
# so we also explicitly dispatch release.yml (requires `gh`, authenticated via
# GH_TOKEN/GITHUB_TOKEN in CI).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TOML="$ROOT_DIR/Cargo.toml"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$CARGO_TOML" | head -n1)"
if [[ -z "${VERSION:-}" ]]; then
  echo "Could not determine version from Cargo.toml" >&2
  exit 1
fi
TAG="v$VERSION"

# Identity for the tag commit when running in CI.
if [[ -z "$(git config user.email || true)" ]]; then
  git config user.email "github-actions[bot]@users.noreply.github.com"
  git config user.name "github-actions[bot]"
fi

if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null \
  || git ls-remote --exit-code --tags origin "refs/tags/$TAG" >/dev/null 2>&1; then
  echo "Tag $TAG already exists; nothing to publish."
  exit 0
fi

echo "Tagging release $TAG"
git tag -a "$TAG" -m "Release $TAG"
git push origin "$TAG"

if command -v gh >/dev/null 2>&1; then
  echo "Dispatching release workflow for $TAG"
  gh workflow run release.yml -f version="$TAG"
else
  echo "warning: gh not found; push a tag manually or dispatch release.yml to build $TAG" >&2
fi
