#!/bin/bash

# mc-cli uninstaller for Bash
# Removes the mc-cli binary from ~/.cargo/bin

TARGET_BIN="$HOME/.cargo/bin/mc-cli"

if [ -f "$TARGET_BIN" ]; then
    echo "Removing mc-cli from $TARGET_BIN..."
    rm "$TARGET_BIN"
    if [ $? -eq 0 ]; then
        echo "mc-cli uninstalled successfully."
    else
        echo "Failed to remove mc-cli. You might need to run this with sudo if it's in a protected directory."
    fi
else
    # Try finding it in path
    CMD_PATH=$(which mc-cli)
    if [ -n "$CMD_PATH" ]; then
        echo "Found mc-cli at $CMD_PATH. Removing..."
        rm "$CMD_PATH"
        if [ $? -eq 0 ]; then
            echo "mc-cli uninstalled successfully."
        else
            echo "Failed to remove $CMD_PATH. Try: sudo rm $CMD_PATH"
        fi
    else
        echo "mc-cli not found in ~/.cargo/bin or your PATH."
    fi
fi
