#!/usr/bin/env bash
# make-vm-image.sh — Create a disk image from the bundled rootfs.
#
# Usage:
#   ./packaging/make-vm-image.sh                   # uses dist/ as input
#   ./packaging/make-vm-image.sh --input /tmp/pkg   # custom input dir
#
# Output: burdetty-os.dmg (macOS disk image)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

INPUT_DIR="$ROOT/dist"
OUTPUT="$ROOT/burdetty-os.dmg"

for arg in "$@"; do
    case "$arg" in
        --input)
            shift_next=true
            ;;
        --help|-h)
            head -9 "$0" | tail -8
            exit 0
            ;;
        *)
            if [ "${shift_next:-}" = true ]; then
                INPUT_DIR="$arg"
                shift_next=false
            fi
            ;;
    esac
done

if [ ! -d "$INPUT_DIR" ]; then
    echo "Error: Input directory '$INPUT_DIR' not found."
    echo "Run ./packaging/bundle-runtime.sh first."
    exit 1
fi

echo "▶ Creating disk image from: $INPUT_DIR"

# Use hdiutil on macOS to create a DMG
if command -v hdiutil &>/dev/null; then
    # Remove old image if present
    [ -f "$OUTPUT" ] && rm "$OUTPUT"

    hdiutil create \
        -volname "burdetty-os" \
        -srcfolder "$INPUT_DIR" \
        -ov \
        -format UDZO \
        "$OUTPUT"

    echo "✓ Created: $OUTPUT"
    ls -lh "$OUTPUT"
else
    # Fallback: create a tar.gz archive
    OUTPUT="$ROOT/burdetty-os.tar.gz"
    tar -czf "$OUTPUT" -C "$INPUT_DIR" .
    echo "✓ Created: $OUTPUT (hdiutil not available, used tar.gz)"
    ls -lh "$OUTPUT"
fi
