#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DISTRO_REPO_PATH="$ROOT_DIR/../seamless-glance-distro"
HOMEBREW_REPO_PATH="$ROOT_DIR/../homebrew-seamless"
WEBSITE_REPO_PATH="$ROOT_DIR/../seamless-glance-website"
RELEASE_REPO="fells-code/seamless-glance-distro"

ALLOW_DIRTY=0
DRY_RUN=0
SKIP_BUILD=0
SKIP_DISTRO_INSTALL=0
SKIP_HOMEBREW=0
SKIP_WEBSITE=0

usage() {
  cat <<'EOF'
Usage: ./scripts/release-helper.sh [options]

Build release artifacts for the current Cargo version and sync release-facing files
in the neighboring support repos. This script does not change the version for you.

Options:
  --allow-dirty             Allow uncommitted changes in target repos
  --dry-run                 Print planned actions without changing files
  --skip-build              Reuse existing dist artifacts instead of rebuilding
  --skip-distro-install     Do not update seamless-glance-distro/install.sh
  --skip-homebrew           Do not update the Homebrew formula
  --skip-website            Do not update seamless-glance-website/public/install.sh
  --distro-repo-path PATH   Override local seamless-glance-distro checkout path
  --homebrew-repo-path PATH Override local homebrew-seamless checkout path
  --website-repo-path PATH  Override local seamless-glance-website checkout path
  --release-repo REPO       Override GitHub release repo slug used in URLs
  --help                    Show this help text
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --allow-dirty)
      ALLOW_DIRTY=1
      ;;
    --dry-run)
      DRY_RUN=1
      ;;
    --skip-build)
      SKIP_BUILD=1
      ;;
    --skip-distro-install)
      SKIP_DISTRO_INSTALL=1
      ;;
    --skip-homebrew)
      SKIP_HOMEBREW=1
      ;;
    --skip-website)
      SKIP_WEBSITE=1
      ;;
    --distro-repo-path)
      DISTRO_REPO_PATH="$2"
      shift
      ;;
    --homebrew-repo-path)
      HOMEBREW_REPO_PATH="$2"
      shift
      ;;
    --website-repo-path)
      WEBSITE_REPO_PATH="$2"
      shift
      ;;
    --release-repo)
      RELEASE_REPO="$2"
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
  shift
done

run() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '+'
    printf ' %q' "$@"
    printf '\n'
    return 0
  fi

  "$@"
}

require_dir() {
  local dir="$1"
  local label="$2"
  if [[ ! -d "$dir" ]]; then
    echo "Missing $label directory: $dir" >&2
    exit 1
  fi
}

require_file() {
  local file="$1"
  local label="$2"
  if [[ ! -f "$file" ]]; then
    echo "Missing $label file: $file" >&2
    exit 1
  fi
}

require_clean_repo() {
  local repo_path="$1"
  local label="$2"

  if [[ "$ALLOW_DIRTY" -eq 1 ]]; then
    return 0
  fi

  if [[ -n "$(git -C "$repo_path" status --short)" ]]; then
    echo "$label has uncommitted changes: $repo_path" >&2
    echo "Re-run with --allow-dirty if you want to proceed anyway." >&2
    exit 1
  fi
}

version_from_cargo() {
  sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1
}

checksum_for() {
  local checksum_file="$1"
  local filename="$2"
  awk -v file="$filename" '$2 == file { print $1 }' "$checksum_file"
}

update_install_script() {
  local file="$1"
  require_file "$file" "install script"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Would update install script: $file"
    return 0
  fi

  VERSION="$VERSION" RELEASE_REPO="$RELEASE_REPO" perl -0pi -e '
    s/^VERSION=".*?"$/qq{VERSION="$ENV{VERSION}"}/me;
    s/^REPO=".*?"$/qq{REPO="$ENV{RELEASE_REPO}"}/me;
  ' "$file"
}

update_homebrew_formula() {
  local formula="$HOMEBREW_REPO_PATH/Formula/seamless-glance.rb"
  require_file "$formula" "Homebrew formula"

  local arm_url="https://github.com/$RELEASE_REPO/releases/download/v$VERSION/$MAC_ARM_FILE"
  local x86_url="https://github.com/$RELEASE_REPO/releases/download/v$VERSION/$MAC_X86_FILE"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Would update Homebrew formula: $formula"
    return 0
  fi

  VERSION="$VERSION" \
  ARM_URL="$arm_url" \
  ARM_SHA="$MAC_ARM_SHA" \
  X86_URL="$x86_url" \
  X86_SHA="$MAC_X86_SHA" \
  perl -0pi -e '
    s/version ".*?"/qq{version "$ENV{VERSION}"}/e;
    s#(if OS\.mac\? && Hardware::CPU\.arm\?\s+url ")[^"]+(")#$1 . $ENV{ARM_URL} . $2#se;
    s#(if OS\.mac\? && Hardware::CPU\.arm\?\s+url "[^"]+"\s+sha256 ")[0-9a-f]+(")#$1 . $ENV{ARM_SHA} . $2#se;
    s#(elsif OS\.mac\?\s+url ")[^"]+(")#$1 . $ENV{X86_URL} . $2#se;
    s#(elsif OS\.mac\?\s+url "[^"]+"\s+sha256 ")[0-9a-f]+(")#$1 . $ENV{X86_SHA} . $2#se;
  ' "$formula"
}

write_release_manifest() {
  local manifest_file="$ROOT_DIR/dist/release-manifest.json"

  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Would write release manifest: $manifest_file"
    return 0
  fi

  cat > "$manifest_file" <<EOF
{
  "version": "$VERSION",
  "tag": "v$VERSION",
  "release_repo": "$RELEASE_REPO",
  "assets": {
    "macos_arm64": {
      "filename": "$MAC_ARM_FILE",
      "sha256": "$MAC_ARM_SHA",
      "url": "https://github.com/$RELEASE_REPO/releases/download/v$VERSION/$MAC_ARM_FILE"
    },
    "macos_x86_64": {
      "filename": "$MAC_X86_FILE",
      "sha256": "$MAC_X86_SHA",
      "url": "https://github.com/$RELEASE_REPO/releases/download/v$VERSION/$MAC_X86_FILE"
    },
    "linux_x86_64": {
      "filename": "$LINUX_X86_FILE",
      "sha256": "$LINUX_X86_SHA",
      "url": "https://github.com/$RELEASE_REPO/releases/download/v$VERSION/$LINUX_X86_FILE"
    }
  }
}
EOF
}

VERSION="$(version_from_cargo)"
if [[ -z "$VERSION" ]]; then
  echo "Could not determine version from Cargo.toml" >&2
  exit 1
fi

MAC_ARM_FILE="seamless-glance-$VERSION-aarch64-apple-darwin"
MAC_X86_FILE="seamless-glance-$VERSION-x86_64-apple-darwin"
LINUX_X86_FILE="seamless-glance-$VERSION-x86_64-unknown-linux-gnu"
CHECKSUM_FILE="$ROOT_DIR/dist/SHA256SUMS.txt"

require_dir "$ROOT_DIR" "repo root"

if [[ "$SKIP_DISTRO_INSTALL" -eq 0 ]]; then
  require_dir "$DISTRO_REPO_PATH" "distro repo"
  require_clean_repo "$DISTRO_REPO_PATH" "Distro repo"
fi

if [[ "$SKIP_HOMEBREW" -eq 0 ]]; then
  require_dir "$HOMEBREW_REPO_PATH" "Homebrew repo"
  require_clean_repo "$HOMEBREW_REPO_PATH" "Homebrew repo"
fi

if [[ "$SKIP_WEBSITE" -eq 0 ]]; then
  require_dir "$WEBSITE_REPO_PATH" "website repo"
  require_clean_repo "$WEBSITE_REPO_PATH" "Website repo"
fi

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  echo "Building release artifacts for version $VERSION..."
  run make -C "$ROOT_DIR" release-local
else
  echo "Skipping build; reusing existing dist artifacts for version $VERSION..."
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  MAC_ARM_SHA="<dry-run>"
  MAC_X86_SHA="<dry-run>"
  LINUX_X86_SHA="<dry-run>"
else
  require_file "$CHECKSUM_FILE" "checksum manifest"
  require_file "$ROOT_DIR/dist/$MAC_ARM_FILE" "macOS arm64 artifact"
  require_file "$ROOT_DIR/dist/$MAC_X86_FILE" "macOS x86_64 artifact"
  require_file "$ROOT_DIR/dist/$LINUX_X86_FILE" "Linux x86_64 artifact"

  MAC_ARM_SHA="$(checksum_for "$CHECKSUM_FILE" "$MAC_ARM_FILE")"
  MAC_X86_SHA="$(checksum_for "$CHECKSUM_FILE" "$MAC_X86_FILE")"
  LINUX_X86_SHA="$(checksum_for "$CHECKSUM_FILE" "$LINUX_X86_FILE")"

  if [[ -z "$MAC_ARM_SHA" || -z "$MAC_X86_SHA" || -z "$LINUX_X86_SHA" ]]; then
    echo "Failed to resolve one or more artifact checksums from $CHECKSUM_FILE" >&2
    exit 1
  fi
fi

write_release_manifest

if [[ "$SKIP_DISTRO_INSTALL" -eq 0 ]]; then
  update_install_script "$DISTRO_REPO_PATH/install.sh"
fi

if [[ "$SKIP_HOMEBREW" -eq 0 ]]; then
  update_homebrew_formula
fi

if [[ "$SKIP_WEBSITE" -eq 0 ]]; then
  update_install_script "$WEBSITE_REPO_PATH/public/install.sh"
fi

echo
echo "Release helper complete for version $VERSION"
echo
echo "Artifacts:"
echo "  $ROOT_DIR/dist/$MAC_ARM_FILE"
echo "  $ROOT_DIR/dist/$MAC_X86_FILE"
echo "  $ROOT_DIR/dist/$LINUX_X86_FILE"
echo "  $ROOT_DIR/dist/SHA256SUMS.txt"
echo "  $ROOT_DIR/dist/release-manifest.json"
echo
echo "Updated repos:"
if [[ "$SKIP_DISTRO_INSTALL" -eq 0 ]]; then
  echo "  $DISTRO_REPO_PATH/install.sh"
fi
if [[ "$SKIP_HOMEBREW" -eq 0 ]]; then
  echo "  $HOMEBREW_REPO_PATH/Formula/seamless-glance.rb"
fi
if [[ "$SKIP_WEBSITE" -eq 0 ]]; then
  echo "  $WEBSITE_REPO_PATH/public/install.sh"
fi
