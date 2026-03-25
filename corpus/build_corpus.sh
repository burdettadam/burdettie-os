#!/bin/bash
# Build all corpus programs for arm64 macOS
# Each program is compiled as a standalone ObjC binary linked against Foundation

set -euo pipefail

CORPUS_DIR="$(cd "$(dirname "$0")" && pwd)"
ARCH="arm64"

COMMON_FLAGS=(
    -arch "$ARCH"
    -framework Foundation
    -fno-objc-arc
    -O0
    -g
)

PROGRAMS=(
    word_counter
    json_formatter
    file_lister
    string_processor
    config_reader
    date_calculator
    path_resolver
    collection_pipeline
    error_reporter
    runtime_info
    todo_manager
    text_analyzer
)

BUILT=0
FAILED=0

for prog in "${PROGRAMS[@]}"; do
    src="$CORPUS_DIR/$prog/main.m"
    out="$CORPUS_DIR/$prog/app"
    
    if [ ! -f "$src" ]; then
        echo "SKIP: $prog (no main.m)"
        continue
    fi
    
    echo -n "Building $prog... "
    if cc "${COMMON_FLAGS[@]}" "$src" -o "$out" 2>/dev/null; then
        echo "OK"
        BUILT=$((BUILT + 1))
    else
        echo "FAILED"
        # Retry with verbose output
        cc "${COMMON_FLAGS[@]}" "$src" -o "$out" 2>&1 | head -5
        FAILED=$((FAILED + 1))
    fi
done

echo ""
echo "Built: $BUILT / $((BUILT + FAILED))"
if [ $FAILED -gt 0 ]; then
    echo "Failed: $FAILED"
    exit 1
fi
echo "All corpus binaries compiled successfully."
