#!/bin/bash

# mc-cli installer
# Usage: curl -sSL https://raw.githubusercontent.com/Mosuzzzz/mc-cli/main/install.sh | bash

set -e

echo "Installing mc-cli..."

if ! command -v cargo &> /dev/null; then
    echo "Error: cargo (Rust) is not installed."
    echo "Please install Rust first: https://rustup.rs/"
    exit 1
fi

cargo install --git https://github.com/Mosuzzzz/mc-cli.git

echo "mc-cli has been installed to ~/.cargo/bin"
echo "Make sure ~/.cargo/bin is in your PATH."
