#!/bin/bash
set -e

echo "Installing mc-cli..."

# Determine platform target triple
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "${OS}-${ARCH}" in
    linux-x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
    linux-aarch64)  TARGET="aarch64-unknown-linux-gnu" ;;
    darwin-x86_64)  TARGET="x86_64-apple-darwin" ;;
    darwin-arm64)   TARGET="aarch64-apple-darwin" ;;
    *)
        echo "Error: Unsupported platform ${OS}-${ARCH}."
        echo "Please build from source: cargo install --git https://github.com/Mosuzzzz/mc-cli.git"
        exit 1
        ;;
esac

# Fetch latest release tag from GitHub API
echo "Fetching latest release..."
TAG=$(curl -sSf \
    -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/Mosuzzzz/mc-cli/releases/latest" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4)

if [ -z "$TAG" ]; then
    echo "Error: Could not fetch the latest release tag from GitHub."
    exit 1
fi
echo "Latest release: $TAG  (platform: $TARGET)"

BIN="mc-cli-${TARGET}"
BASE_URL="https://github.com/Mosuzzzz/mc-cli/releases/download/${TAG}"
TEMP_BIN="$(mktemp /tmp/mc-cli-XXXXXX)"

# Download binary
echo "Downloading $BIN..."
curl -sSfL "${BASE_URL}/${BIN}" -o "$TEMP_BIN"

# Download checksums and extract expected hash
echo "Verifying SHA-256 checksum..."
EXPECTED=$(curl -sSfL "${BASE_URL}/sha256sums.txt" \
    | grep "^[0-9a-f]\{64\}  ${BIN}$" \
    | awk '{print $1}')

if [ -z "$EXPECTED" ]; then
    echo "Error: Could not find checksum for '${BIN}' in sha256sums.txt."
    rm -f "$TEMP_BIN"
    exit 1
fi

if command -v sha256sum &>/dev/null; then
    ACTUAL=$(sha256sum "$TEMP_BIN" | awk '{print $1}')
elif command -v shasum &>/dev/null; then
    ACTUAL=$(shasum -a 256 "$TEMP_BIN" | awk '{print $1}')
else
    echo "Error: sha256sum / shasum not found — cannot verify download."
    rm -f "$TEMP_BIN"
    exit 1
fi

if [ "$EXPECTED" != "$ACTUAL" ]; then
    echo "Error: SHA-256 mismatch — refusing to install."
    echo "  Expected: $EXPECTED"
    echo "  Got:      $ACTUAL"
    rm -f "$TEMP_BIN"
    exit 1
fi
echo "Checksum OK."

chmod +x "$TEMP_BIN"
INSTALL_DIR="$HOME/.cargo/bin"
mkdir -p "$INSTALL_DIR"
mv "$TEMP_BIN" "$INSTALL_DIR/mc-cli"

echo ""
echo "mc-cli $TAG installed to $INSTALL_DIR/mc-cli"
echo "Make sure ~/.cargo/bin is in your PATH."
