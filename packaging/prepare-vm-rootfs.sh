#!/usr/bin/env bash
# prepare-vm-rootfs.sh — Assemble runtime + fixtures into the shared directory
# layout expected by the VM guest.
#
# Usage:
#   ./packaging/prepare-vm-rootfs.sh                   # output → dist/
#   ./packaging/prepare-vm-rootfs.sh --output /tmp/vm   # custom output
#   ./packaging/prepare-vm-rootfs.sh --release          # use release builds
#
# Output layout (mounted inside guest at /Volumes/darwin-runtime/):
#   darwin-runtime/
#   ├── bin/
#   │   └── macho-inspect
#   ├── fixtures/
#   │   ├── simple_cli/app
#   │   └── ...
#   ├── config/
#   │   ├── auto-launch.plist
#   │   └── runtime-env.sh
#   └── docs/
#       └── ...
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

# ── Parse arguments ─────────────────────────────────────────────────
OUTPUT_DIR="$ROOT/dist"
RELEASE_FLAG=""

for arg in "$@"; do
    case "$arg" in
        --release)
            RELEASE_FLAG="--release"
            ;;
        --output)
            shift_next=true
            ;;
        --help|-h)
            head -19 "$0" | tail -18
            exit 0
            ;;
        *)
            if [ "${shift_next:-}" = true ]; then
                OUTPUT_DIR="$arg"
                shift_next=false
            fi
            ;;
    esac
done

echo "▶ Preparing VM rootfs → $OUTPUT_DIR"

# ── Step 1: Run the standard bundler ────────────────────────────────
echo ""
echo "  Step 1: Run bundle-runtime.sh..."
"$SCRIPT_DIR/bundle-runtime.sh" --output "$OUTPUT_DIR" $RELEASE_FLAG

# ── Step 2: Add config directory ────────────────────────────────────
echo ""
echo "  Step 2: Creating config/..."
mkdir -p "$OUTPUT_DIR/config"

# Environment setup script for the guest
cat > "$OUTPUT_DIR/config/runtime-env.sh" << 'ENVEOF'
#!/bin/bash
# Source this in the guest to set up the runtime environment.
# The shared directory is mounted via VirtioFS at /Volumes/darwin-runtime/
export DARWIN_RUNTIME_ROOT="/Volumes/darwin-runtime"
export DARWIN_LOADER_ROOT="$DARWIN_RUNTIME_ROOT"
export PATH="$DARWIN_RUNTIME_ROOT/bin:$PATH"

echo "burdetty-os runtime environment loaded"
echo "  Root:     $DARWIN_RUNTIME_ROOT"
echo "  Fixtures: $DARWIN_RUNTIME_ROOT/fixtures/"
echo "  Shell:    runtime-shell"
echo "  GUI:      gui hello-darwin | control-gallery | foundation-explorer"
echo ""
echo "Type 'runtime-shell' to start the interactive demo shell."
ENVEOF
chmod +x "$OUTPUT_DIR/config/runtime-env.sh"

# Auto-launch script that starts runtime-shell on boot
cat > "$OUTPUT_DIR/config/auto-start.sh" << 'STARTEOF'
#!/bin/bash
# Auto-start script for the VM guest — launched by auto-launch.plist.
source /Volumes/darwin-runtime/config/runtime-env.sh
exec runtime-shell
STARTEOF
chmod +x "$OUTPUT_DIR/config/auto-start.sh"

# Copy the auto-launch plist if it exists
if [ -f "$ROOT/vm-image/auto-launch.plist" ]; then
    cp "$ROOT/vm-image/auto-launch.plist" "$OUTPUT_DIR/config/"
fi

echo "  ✓ config/ ready"

# ── Summary ─────────────────────────────────────────────────────────
total_size=$(du -sh "$OUTPUT_DIR" 2>/dev/null | awk '{print $1}')
fixture_count=$(ls -d "$OUTPUT_DIR/fixtures"/*/app 2>/dev/null | wc -l | tr -d ' ')
echo ""
echo "VM rootfs ready: $OUTPUT_DIR ($total_size)"
echo "  bin:      $(ls "$OUTPUT_DIR/bin/" 2>/dev/null | wc -l | tr -d ' ') binaries"
echo "  fixtures: $fixture_count apps"
echo "  config:   $(ls "$OUTPUT_DIR/config/" 2>/dev/null | wc -l | tr -d ' ') files"
echo ""
echo "Mount inside guest at: /Volumes/darwin-runtime/"
echo "Then: source /Volumes/darwin-runtime/config/runtime-env.sh"
