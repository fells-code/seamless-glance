#!/usr/bin/env bash
# Seamless Glance installer (macOS + Linux)
#
#   curl -fsSL https://raw.githubusercontent.com/fells-code/seamless-glance/main/install.sh | bash
#
# By default it installs the latest release. Pin a version with:
#   SEAMLESS_GLANCE_VERSION=1.2.3 bash install.sh
set -euo pipefail

BIN_NAME="seamless-glance"
ALIAS="glance"
REPO="fells-code/seamless-glance"
INSTALL_DIR="${SEAMLESS_GLANCE_INSTALL_DIR:-/usr/local/bin}"
VERSION="${SEAMLESS_GLANCE_VERSION:-latest}"

OS="$(uname -s)"
ARCH="$(uname -m)"

# ---- Resolve target triple -------------------------------------------------
is_musl() {
  [[ -f /etc/alpine-release ]] && return 0
  if command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then
    return 0
  fi
  return 1
}

case "$OS" in
  Darwin)
    if [[ "$ARCH" == "arm64" || "$ARCH" == "aarch64" ]]; then
      TARGET="aarch64-apple-darwin"
    else
      TARGET="x86_64-apple-darwin"
    fi
    ;;
  Linux)
    case "$ARCH" in
      x86_64|amd64)
        if is_musl; then
          TARGET="x86_64-unknown-linux-musl"
        else
          TARGET="x86_64-unknown-linux-gnu"
        fi
        ;;
      aarch64|arm64)
        TARGET="aarch64-unknown-linux-gnu"
        ;;
      *)
        echo "❌ Unsupported Linux architecture: $ARCH" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "❌ Unsupported platform: $OS / $ARCH" >&2
    echo "   On Windows, use install.ps1 (PowerShell)." >&2
    exit 1
    ;;
esac

# ---- Resolve version -------------------------------------------------------
if [[ "$VERSION" == "latest" ]]; then
  echo "🔎 Resolving latest release..."
  VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\{0,1\}\([^"]*\)".*/\1/p' | head -n1)"
  if [[ -z "$VERSION" ]]; then
    echo "❌ Could not resolve the latest release version." >&2
    exit 1
  fi
fi

FILE="$BIN_NAME-$VERSION-$TARGET"
URL="https://github.com/$REPO/releases/download/v$VERSION/$FILE"
CHECKSUM_URL="https://github.com/$REPO/releases/download/v$VERSION/SHA256SUMS.txt"

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT
cd "$WORKDIR"

echo "⬇️  Downloading Seamless Glance $VERSION ($TARGET)..."
curl -fsSL "$URL" -o "$FILE"

if curl -fsSL "$CHECKSUM_URL" -o SHA256SUMS.txt 2>/dev/null; then
  echo "🔐 Verifying checksum..."
  if command -v sha256sum >/dev/null 2>&1; then
    grep " $FILE\$" SHA256SUMS.txt | sha256sum -c -
  else
    grep " $FILE\$" SHA256SUMS.txt | shasum -a 256 -c -
  fi
else
  echo "⚠️  Checksum file not found, skipping verification"
fi

chmod +x "$FILE"
mv "$FILE" "$BIN_NAME"

# ---- Install ---------------------------------------------------------------
install_to() {
  local dir="$1"
  mv "$BIN_NAME" "$dir/$BIN_NAME"
  ln -sf "$dir/$BIN_NAME" "$dir/$ALIAS"
}

if [[ -w "$INSTALL_DIR" ]]; then
  install_to "$INSTALL_DIR"
elif command -v sudo >/dev/null 2>&1; then
  sudo mv "$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"
  sudo ln -sf "$INSTALL_DIR/$BIN_NAME" "$INSTALL_DIR/$ALIAS"
else
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
  install_to "$INSTALL_DIR"
  echo "⚠️  Installed to $INSTALL_DIR (add it to your PATH if needed)"
fi

echo ""
echo "✅ Seamless Glance $VERSION installed"
echo ""
echo "Next steps:"
echo "  1. Ensure AWS credentials are available (AWS_PROFILE, ~/.aws, SSO, or env vars)"
echo "  2. Run: seamless-glance   (or: glance)"
