#!/usr/bin/env bash
set -euo pipefail

# Build (optionally) the release artifacts for the current Cargo version and
# regenerate the Homebrew formula in the neighboring tap repo. This does NOT
# change the version, tag, commit, or push. See RELEASE.md.
#
# The public curl installers (install.sh / install.ps1) resolve the latest
# release dynamically, so they do not need per-release edits and are not touched
# here.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOMEBREW_REPO_PATH="$ROOT_DIR/../homebrew-seamless"
RELEASE_REPO="fells-code/seamless-glance"

ALLOW_DIRTY=0
DRY_RUN=0
SKIP_BUILD=0
SKIP_HOMEBREW=0

usage() {
  cat <<'EOF'
Usage: ./scripts/release-helper.sh [options]

Build release artifacts for the current Cargo version and regenerate the
Homebrew formula in the neighboring tap repo. Does not change the version.

Options:
  --allow-dirty             Allow uncommitted changes in the tap repo
  --dry-run                 Print planned actions without changing files
  --skip-build              Reuse existing dist artifacts instead of rebuilding
  --skip-homebrew           Do not update the Homebrew formula
  --homebrew-repo-path PATH Override local homebrew-seamless checkout path
  --release-repo REPO       Override GitHub release repo slug used in URLs
  --help                    Show this help text
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --allow-dirty) ALLOW_DIRTY=1 ;;
    --dry-run) DRY_RUN=1 ;;
    --skip-build) SKIP_BUILD=1 ;;
    --skip-homebrew) SKIP_HOMEBREW=1 ;;
    --homebrew-repo-path) HOMEBREW_REPO_PATH="$2"; shift ;;
    --release-repo) RELEASE_REPO="$2"; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
  shift
done

require_dir() { [[ -d "$1" ]] || { echo "Missing $2 directory: $1" >&2; exit 1; }; }
require_file() { [[ -f "$1" ]] || { echo "Missing $2 file: $1" >&2; exit 1; }; }

require_clean_repo() {
  local repo_path="$1" label="$2"
  [[ "$ALLOW_DIRTY" -eq 1 ]] && return 0
  if [[ -n "$(git -C "$repo_path" status --short)" ]]; then
    echo "$label has uncommitted changes: $repo_path" >&2
    echo "Re-run with --allow-dirty if you want to proceed anyway." >&2
    exit 1
  fi
}

version_from_cargo() {
  sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1
}

# Look up a checksum by filename in dist/SHA256SUMS.txt; empty if not present.
checksum_for() {
  awk -v file="$2" '$2 == file { print $1 }' "$1"
}

asset_url() {
  echo "https://github.com/$RELEASE_REPO/releases/download/v$VERSION/$1"
}

VERSION="$(version_from_cargo)"
[[ -n "$VERSION" ]] || { echo "Could not determine version from Cargo.toml" >&2; exit 1; }

MAC_ARM_FILE="seamless-glance-$VERSION-aarch64-apple-darwin"
MAC_X86_FILE="seamless-glance-$VERSION-x86_64-apple-darwin"
LINUX_GNU_X86_FILE="seamless-glance-$VERSION-x86_64-unknown-linux-gnu"
LINUX_GNU_ARM_FILE="seamless-glance-$VERSION-aarch64-unknown-linux-gnu"
LINUX_MUSL_X86_FILE="seamless-glance-$VERSION-x86_64-unknown-linux-musl"
WINDOWS_X86_FILE="seamless-glance-$VERSION-x86_64-pc-windows-msvc.exe"
CHECKSUM_FILE="$ROOT_DIR/dist/SHA256SUMS.txt"

if [[ "$SKIP_HOMEBREW" -eq 0 ]]; then
  require_dir "$HOMEBREW_REPO_PATH" "Homebrew repo"
  require_clean_repo "$HOMEBREW_REPO_PATH" "Homebrew repo"
fi

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  echo "Building release artifacts for version $VERSION..."
  make -C "$ROOT_DIR" release-local
else
  echo "Skipping build; reusing existing dist artifacts for version $VERSION..."
fi

require_file "$CHECKSUM_FILE" "checksum manifest"

MAC_ARM_SHA="$(checksum_for "$CHECKSUM_FILE" "$MAC_ARM_FILE")"
MAC_X86_SHA="$(checksum_for "$CHECKSUM_FILE" "$MAC_X86_FILE")"
LINUX_GNU_X86_SHA="$(checksum_for "$CHECKSUM_FILE" "$LINUX_GNU_X86_FILE")"
LINUX_GNU_ARM_SHA="$(checksum_for "$CHECKSUM_FILE" "$LINUX_GNU_ARM_FILE")"
LINUX_MUSL_X86_SHA="$(checksum_for "$CHECKSUM_FILE" "$LINUX_MUSL_X86_FILE")"
WINDOWS_X86_SHA="$(checksum_for "$CHECKSUM_FILE" "$WINDOWS_X86_FILE")"

# macOS checksums are required (Homebrew is macOS-first).
if [[ -z "$MAC_ARM_SHA" || -z "$MAC_X86_SHA" ]]; then
  echo "Failed to resolve macOS checksums from $CHECKSUM_FILE" >&2
  exit 1
fi

# ---- Homebrew formula ------------------------------------------------------
write_homebrew_formula() {
  local formula="$HOMEBREW_REPO_PATH/Formula/seamless-glance.rb"
  require_dir "$(dirname "$formula")" "Homebrew Formula"

  local linux_block=""
  if [[ -n "$LINUX_GNU_X86_SHA" ]]; then
    local arm_case=""
    if [[ -n "$LINUX_GNU_ARM_SHA" ]]; then
      arm_case=$(cat <<RUBY
    if Hardware::CPU.arm?
      url "$(asset_url "$LINUX_GNU_ARM_FILE")"
      sha256 "$LINUX_GNU_ARM_SHA"
    else
      url "$(asset_url "$LINUX_GNU_X86_FILE")"
      sha256 "$LINUX_GNU_X86_SHA"
    end
RUBY
)
    else
      arm_case=$(cat <<RUBY
    url "$(asset_url "$LINUX_GNU_X86_FILE")"
    sha256 "$LINUX_GNU_X86_SHA"
RUBY
)
    fi
    linux_block=$(cat <<RUBY

  on_linux do
$arm_case
  end
RUBY
)
  fi

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Would write Homebrew formula: $formula"
    return 0
  fi

  cat > "$formula" <<RUBY
class SeamlessGlance < Formula
  desc "Fast, read-only AWS TUI for cloud infrastructure visibility"
  homepage "https://github.com/$RELEASE_REPO"
  license "GPL-3.0-only"
  version "$VERSION"

  on_macos do
    if Hardware::CPU.arm?
      url "$(asset_url "$MAC_ARM_FILE")"
      sha256 "$MAC_ARM_SHA"
    else
      url "$(asset_url "$MAC_X86_FILE")"
      sha256 "$MAC_X86_SHA"
    end
  end
$linux_block
  def install
    bin.install Dir["seamless-glance-*"].first => "seamless-glance"
    bin.install_symlink "seamless-glance" => "glance"
  end

  test do
    system "#{bin}/seamless-glance", "--version"
  end
end
RUBY
  echo "Wrote Homebrew formula: $formula"
}

# ---- Release manifest ------------------------------------------------------
manifest_asset() {
  local file="$1" sha="$2"
  [[ -z "$sha" ]] && return 0
  cat <<JSON
    "$file": {
      "filename": "$file",
      "sha256": "$sha",
      "url": "$(asset_url "$file")"
    },
JSON
}

write_release_manifest() {
  local manifest_file="$ROOT_DIR/dist/release-manifest.json"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Would write release manifest: $manifest_file"
    return 0
  fi
  {
    echo "{"
    echo "  \"version\": \"$VERSION\","
    echo "  \"tag\": \"v$VERSION\","
    echo "  \"release_repo\": \"$RELEASE_REPO\","
    echo "  \"assets\": {"
    # Emit each present asset, then strip the trailing comma from the last one.
    {
      manifest_asset "$MAC_ARM_FILE" "$MAC_ARM_SHA"
      manifest_asset "$MAC_X86_FILE" "$MAC_X86_SHA"
      manifest_asset "$LINUX_GNU_X86_FILE" "$LINUX_GNU_X86_SHA"
      manifest_asset "$LINUX_GNU_ARM_FILE" "$LINUX_GNU_ARM_SHA"
      manifest_asset "$LINUX_MUSL_X86_FILE" "$LINUX_MUSL_X86_SHA"
      manifest_asset "$WINDOWS_X86_FILE" "$WINDOWS_X86_SHA"
    } | perl -0pe 's/,(\s*)$/$1/'
    echo "  }"
    echo "}"
  } > "$manifest_file"
  echo "Wrote release manifest: $manifest_file"
}

write_release_manifest
[[ "$SKIP_HOMEBREW" -eq 0 ]] && write_homebrew_formula

echo
echo "Release helper complete for version $VERSION"
echo "  Release repo: $RELEASE_REPO"
if [[ "$SKIP_HOMEBREW" -eq 0 ]]; then
  echo "  Formula:      $HOMEBREW_REPO_PATH/Formula/seamless-glance.rb"
fi
