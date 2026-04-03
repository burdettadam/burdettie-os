#!/usr/bin/env bash
# launch.sh — One-command VM launcher for burdettie-os.
#
# Usage:
#   ./vm-image/launch.sh                # boot existing VM (or install if first run)
#   ./vm-image/launch.sh install        # force fresh install
#   ./vm-image/launch.sh status         # check VM status
#   ./vm-image/launch.sh clean          # remove VM bundle
#
# Prerequisites:
#   - macOS 13+ (Ventura or later) on Apple Silicon
#   - Xcode or Command Line Tools installed
#   - dist/ directory created via ./packaging/bundle-runtime.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

COMMAND="${1:-auto}"

# ── Ensure dist/ exists ────────────────────────────────────────────
if [ ! -d "$ROOT/dist" ] && [ "$COMMAND" != "clean" ] && [ "$COMMAND" != "status" ] && [ "$COMMAND" != "help" ]; then
    echo "▶ No dist/ directory found. Building runtime bundle..."
    if [ -x "$ROOT/packaging/prepare-vm-rootfs.sh" ]; then
        "$ROOT/packaging/prepare-vm-rootfs.sh"
    elif [ -x "$ROOT/packaging/bundle-runtime.sh" ]; then
        "$ROOT/packaging/bundle-runtime.sh"
    else
        echo "✗ Cannot find bundle script. Build manually first."
        exit 1
    fi
    echo ""
fi

# ── Build the Swift VM launcher ────────────────────────────────────
VM_BINARY="$SCRIPT_DIR/.build/release/darwin-vm"

build_launcher() {
    echo "▶ Building VM launcher..."
    cd "$SCRIPT_DIR"
    swift build -c release 2>&1
    cd "$ROOT"
    echo "  ✓ Built: $VM_BINARY"
    echo ""
}

if [ ! -f "$VM_BINARY" ]; then
    build_launcher
fi

# ── Auto-detect: install vs boot ───────────────────────────────────
if [ "$COMMAND" = "auto" ]; then
    VM_BUNDLE="${VM_BUNDLE:-$HOME/.burdettie-os/vm-bundle}"
    if [ -f "$VM_BUNDLE/disk.img" ]; then
        COMMAND="boot"
    else
        echo "  No existing VM found. Running first-time install..."
        echo "  This will download macOS (~13GB) and install it."
        echo ""
        COMMAND="install"
    fi
fi

# ── Run ────────────────────────────────────────────────────────────
export DARWIN_LOADER_ROOT="$ROOT"
exec "$VM_BINARY" "$COMMAND"
