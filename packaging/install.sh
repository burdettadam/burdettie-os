#!/usr/bin/env bash
# install.sh — Install darwin-runtime library, headers, binaries, and pkg-config
#
# Usage:
#   ./packaging/install.sh                     # install to /usr/local (release build)
#   ./packaging/install.sh --prefix ~/my-prefix
#   ./packaging/install.sh --debug             # install debug build
#   ./packaging/install.sh --uninstall         # remove installed files
#   ./packaging/install.sh --dry-run           # show what would be installed
#
# Installed layout under PREFIX:
#   lib/libdarwin_runtime_ffi.dylib
#   lib/pkgconfig/darwin-runtime.pc
#   include/darwin_runtime.h
#   bin/{runtime-shell,macho-inspect,hello-darwin,control-gallery,foundation-explorer}
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$SCRIPT_DIR/.."

PREFIX="/usr/local"
BUILD_PROFILE="release"
UNINSTALL=false
DRY_RUN=false
VERSION="0.1.0"

# ── Parse arguments ─────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix)
            PREFIX="$2"; shift 2 ;;
        --debug)
            BUILD_PROFILE="debug"; shift ;;
        --uninstall)
            UNINSTALL=true; shift ;;
        --dry-run)
            DRY_RUN=true; shift ;;
        --help|-h)
            head -16 "$0" | tail -14; exit 0 ;;
        *)
            echo "Unknown option: $1"; exit 1 ;;
    esac
done

BUILD_DIR="$ROOT/target/$BUILD_PROFILE"

# ── File manifest ───────────────────────────────────────────────────
DYLIB_SRC="$BUILD_DIR/libdarwin_runtime_ffi.dylib"
HEADER_SRC="$ROOT/crates/darwin-runtime-ffi/include/darwin_runtime.h"
PC_TEMPLATE="$ROOT/packaging/darwin-runtime.pc.in"

BINARIES=(runtime-shell macho-inspect hello-darwin control-gallery foundation-explorer)

# Installed paths
DYLIB_DST="$PREFIX/lib/libdarwin_runtime_ffi.dylib"
HEADER_DST="$PREFIX/include/darwin_runtime.h"
PC_DST="$PREFIX/lib/pkgconfig/darwin-runtime.pc"

# ── Uninstall ───────────────────────────────────────────────────────
if $UNINSTALL; then
    echo "▶ Uninstalling darwin-runtime from $PREFIX"
    removed=0
    for f in "$DYLIB_DST" "$HEADER_DST" "$PC_DST"; do
        if [ -f "$f" ]; then
            if $DRY_RUN; then echo "  would remove: $f"
            else rm -f "$f"; echo "  ✓ removed $f"; fi
            removed=$((removed + 1))
        fi
    done
    for bin_name in "${BINARIES[@]}"; do
        f="$PREFIX/bin/$bin_name"
        if [ -f "$f" ]; then
            if $DRY_RUN; then echo "  would remove: $f"
            else rm -f "$f"; echo "  ✓ removed $f"; fi
            removed=$((removed + 1))
        fi
    done
    echo "Uninstall complete ($removed files removed)"
    exit 0
fi

# ── Build if needed ─────────────────────────────────────────────────
if [ ! -f "$DYLIB_SRC" ]; then
    echo "▶ Building darwin-runtime-ffi ($BUILD_PROFILE)..."
    if [ "$BUILD_PROFILE" = "release" ]; then
        (cd "$ROOT" && cargo build -p darwin-runtime-ffi --release)
    else
        (cd "$ROOT" && cargo build -p darwin-runtime-ffi)
    fi
fi

# ── Verify artifacts exist ──────────────────────────────────────────
for f in "$DYLIB_SRC" "$HEADER_SRC" "$PC_TEMPLATE"; do
    if [ ! -f "$f" ]; then
        echo "ERROR: Required file not found: $f"
        exit 1
    fi
done

# ── Install ─────────────────────────────────────────────────────────
echo "▶ Installing darwin-runtime to $PREFIX"

install_file() {
    local src="$1" dst="$2"
    local dir; dir=$(dirname "$dst")
    if $DRY_RUN; then
        echo "  would install: $dst"
    else
        mkdir -p "$dir"
        cp "$src" "$dst"
        echo "  ✓ $dst"
    fi
}

# Library
install_file "$DYLIB_SRC" "$DYLIB_DST"

# Header
install_file "$HEADER_SRC" "$HEADER_DST"

# pkg-config — generate from template
PC_GENERATED=$(mktemp)
sed -e "s|@PREFIX@|$PREFIX|g" -e "s|@VERSION@|$VERSION|g" "$PC_TEMPLATE" > "$PC_GENERATED"
install_file "$PC_GENERATED" "$PC_DST"
rm -f "$PC_GENERATED"

# Binaries
for bin_name in "${BINARIES[@]}"; do
    bin_src="$BUILD_DIR/$bin_name"
    if [ -f "$bin_src" ]; then
        install_file "$bin_src" "$PREFIX/bin/$bin_name"
    fi
done

# ── Summary ─────────────────────────────────────────────────────────
echo ""
if $DRY_RUN; then
    echo "Dry run complete. No files were installed."
else
    echo "Install complete."
    echo "  Library: $DYLIB_DST"
    echo "  Header:  $HEADER_DST"
    echo "  PkgCfg:  $PC_DST"
    echo ""
    echo "Test with:"
    echo "  pkg-config --cflags --libs darwin-runtime"
    echo "  cc -o test test.c \$(pkg-config --cflags --libs darwin-runtime)"
fi
