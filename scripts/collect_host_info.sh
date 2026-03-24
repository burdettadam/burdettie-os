#!/usr/bin/env bash
# collect_host_info.sh — Capture host observation data for a fixture binary.
# Usage: ./collect_host_info.sh <binary_path> [output_dir]
set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <binary_path> [output_dir]"
    exit 1
fi

BINARY="$1"
OUTPUT_DIR="${2:-./observed}"
BASENAME="$(basename "$BINARY")"

mkdir -p "$OUTPUT_DIR"

echo "=== Collecting host info for: $BINARY ==="

# Static: linked dylibs
echo "--- otool -L ---"
otool -L "$BINARY" > "$OUTPUT_DIR/${BASENAME}_otool_L.txt"

# Static: load commands
echo "--- otool -l ---"
otool -l "$BINARY" > "$OUTPUT_DIR/${BASENAME}_otool_l.txt"

# Static: header
echo "--- otool -h ---"
otool -h "$BINARY" > "$OUTPUT_DIR/${BASENAME}_otool_h.txt"

# Runtime: dyld library loading (run the binary with DYLD_PRINT_LIBRARIES)
echo "--- DYLD_PRINT_LIBRARIES ---"
DYLD_PRINT_LIBRARIES=1 "$BINARY" > "$OUTPUT_DIR/${BASENAME}_dyld_libs.txt" 2>&1 || true

# Runtime: initializer order (run with DYLD_PRINT_INITIALIZERS)
echo "--- DYLD_PRINT_INITIALIZERS ---"
DYLD_PRINT_INITIALIZERS=1 "$BINARY" > "$OUTPUT_DIR/${BASENAME}_dyld_inits.txt" 2>&1 || true

echo "=== Host info saved to $OUTPUT_DIR ==="
