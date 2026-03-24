#!/usr/bin/env bash
# bundle-runtime.sh — Collect runtime binaries + fixtures into a distributable layout.
#
# Usage:
#   ./packaging/bundle-runtime.sh                  # output → dist/
#   ./packaging/bundle-runtime.sh --output /tmp/pkg # custom output dir
#   ./packaging/bundle-runtime.sh --release         # use release build
#
# Output layout:
#   dist/
#   ├── bin/
#   │   └── macho-inspect
#   ├── fixtures/
#   │   ├── simple_cli/app
#   │   ├── return42/app
#   │   └── ...
#   └── docs/
#       ├── README.md
#       ├── architecture.md
#       └── BUILD.md
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

# ── Parse arguments ─────────────────────────────────────────────────
OUTPUT_DIR="$ROOT/dist"
BUILD_DIR="target/debug"

for arg in "$@"; do
    case "$arg" in
        --release)
            BUILD_DIR="target/release"
            ;;
        --output)
            shift_next=true
            ;;
        --help|-h)
            head -16 "$0" | tail -15
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

echo "▶ Bundling runtime into: $OUTPUT_DIR"

# ── Clean output directory ─────────────────────────────────────────
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/bin" "$OUTPUT_DIR/lib" "$OUTPUT_DIR/include" \
         "$OUTPUT_DIR/lib/pkgconfig" "$OUTPUT_DIR/fixtures" "$OUTPUT_DIR/docs"

# ── Copy binaries ──────────────────────────────────────────────────
bin_count=0
for bin_name in macho-inspect runtime-shell hello-darwin control-gallery foundation-explorer; do
    if [ -f "$ROOT/$BUILD_DIR/$bin_name" ]; then
        cp "$ROOT/$BUILD_DIR/$bin_name" "$OUTPUT_DIR/bin/"
        echo "  ✓ bin/$bin_name"
        bin_count=$((bin_count + 1))
    else
        echo "  ⚠ $bin_name not found in $BUILD_DIR"
    fi
done

# ── Copy FFI library + header + pkg-config ─────────────────────────
DYLIB_NAME="libdarwin_runtime_ffi.dylib"
if [ -f "$ROOT/$BUILD_DIR/$DYLIB_NAME" ]; then
    cp "$ROOT/$BUILD_DIR/$DYLIB_NAME" "$OUTPUT_DIR/lib/"
    echo "  ✓ lib/$DYLIB_NAME"
fi

HEADER="$ROOT/crates/darwin-runtime-ffi/include/darwin_runtime.h"
if [ -f "$HEADER" ]; then
    cp "$HEADER" "$OUTPUT_DIR/include/"
    echo "  ✓ include/darwin_runtime.h"
fi

PC_TEMPLATE="$ROOT/packaging/darwin-runtime.pc.in"
if [ -f "$PC_TEMPLATE" ]; then
    sed -e "s|@PREFIX@|$OUTPUT_DIR|g" -e "s|@VERSION@|0.1.0|g" \
        "$PC_TEMPLATE" > "$OUTPUT_DIR/lib/pkgconfig/darwin-runtime.pc"
    echo "  ✓ lib/pkgconfig/darwin-runtime.pc"
fi

# ── Copy fixtures ──────────────────────────────────────────────────
fixture_count=0
for fixture_dir in "$ROOT/fixtures"/*/; do
    fixture_name=$(basename "$fixture_dir")
    mkdir -p "$OUTPUT_DIR/fixtures/$fixture_name"

    # Copy the app binary if present
    if [ -f "$fixture_dir/app" ]; then
        cp "$fixture_dir/app" "$OUTPUT_DIR/fixtures/$fixture_name/"
        fixture_count=$((fixture_count + 1))
    fi

    # Copy any dylibs or frameworks
    for dylib in "$fixture_dir"/*.dylib; do
        [ -f "$dylib" ] && cp "$dylib" "$OUTPUT_DIR/fixtures/$fixture_name/"
    done

    # Copy framework bundles
    for fw in "$fixture_dir"/*.framework; do
        [ -d "$fw" ] && cp -R "$fw" "$OUTPUT_DIR/fixtures/$fixture_name/"
    done

    # Copy expected.json if present (used by comparison tests)
    if [ -f "$fixture_dir/expected.json" ]; then
        cp "$fixture_dir/expected.json" "$OUTPUT_DIR/fixtures/$fixture_name/"
    fi
done
echo "  ✓ fixtures/ ($fixture_count app binaries)"

# ── Copy docs ──────────────────────────────────────────────────────
doc_count=0
for doc in "$ROOT/docs"/*.md "$ROOT/README.md"; do
    if [ -f "$doc" ]; then
        cp "$doc" "$OUTPUT_DIR/docs/"
        doc_count=$((doc_count + 1))
    fi
done
echo "  ✓ docs/ ($doc_count files)"

# ── Copy demos.json and banner.txt ─────────────────────────────────
mkdir -p "$OUTPUT_DIR/config"
for cfg_file in demos.json banner.txt; do
    if [ -f "$ROOT/packaging/$cfg_file" ]; then
        cp "$ROOT/packaging/$cfg_file" "$OUTPUT_DIR/config/"
        echo "  ✓ config/$cfg_file"
    fi
done

# ── Summary ────────────────────────────────────────────────────────
total_size=$(du -sh "$OUTPUT_DIR" 2>/dev/null | awk '{print $1}')
echo ""
echo "Bundle complete: $OUTPUT_DIR ($total_size)"
echo "  bin:      $(ls "$OUTPUT_DIR/bin/" 2>/dev/null | wc -l | tr -d ' ') binaries"
echo "  fixtures: $fixture_count apps"
echo "  docs:     $doc_count files"
