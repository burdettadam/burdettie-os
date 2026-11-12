#!/usr/bin/env bash
# Rebuild every authored corpus program for ARM64 macOS.
set -euo pipefail
export LC_ALL=C

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "error: build_corpus.sh requires macOS and the Xcode command-line tools" >&2
    exit 1
fi

CORPUS_DIR="$(cd "$(dirname "$0")" && pwd)"
CC="$(xcrun --sdk macosx --find clang)"
SDK="$(xcrun --sdk macosx --show-sdk-path)"
COMMON_FLAGS=(-isysroot "$SDK" -arch arm64 -framework Foundation -fno-objc-arc -fblocks -O0 -g)
BUILT=0

# Source discovery keeps new programs in the build without maintaining a
# second, easily stale list. Shell glob ordering is stable under LC_ALL=C.
for source in "$CORPUS_DIR"/*/main.m; do
    [[ -f "$source" ]] || continue
    dir="$(dirname "$source")"
    echo "Building $(basename "$dir")"
    "$CC" "${COMMON_FLAGS[@]}" "$source" -o "$dir/app"
    BUILT=$((BUILT + 1))
done

if [[ "$BUILT" -eq 0 ]]; then
    echo "error: no corpus sources found" >&2
    exit 1
fi
printf 'Built all %s corpus programs.\n' "$BUILT"
