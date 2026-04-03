#!/bin/bash
# extract_xcode_tools.sh — Extract Mach-O tool binaries from macOS host
# for testing inside Docker on Linux ARM64.
#
# Usage (from repo root, on macOS):
#   ./scripts/extract_xcode_tools.sh [output_dir]
#
# Default output: ./docker/xcode-tools/

set -euo pipefail

OUT="${1:-docker/xcode-tools}"
XCODE_TC="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin"
XCODE_DEV="/Applications/Xcode.app/Contents/Developer/usr/bin"

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

# Ensure we're on macOS
if [ "$(uname -s)" != "Darwin" ]; then
    echo "error: this script must run on macOS to extract Xcode tools"
    exit 1
fi

# Ensure Xcode is installed
if [ ! -d "$XCODE_TC" ]; then
    echo "error: Xcode toolchain not found at $XCODE_TC"
    exit 1
fi

echo -e "${CYAN}Extracting Xcode CLI tool binaries → ${OUT}${NC}"
mkdir -p "$OUT/xcode-tc" "$OUT/xcode-dev" "$OUT/system"

copy_tool() {
    local src="$1"
    local dest_dir="$2"
    local name
    name=$(basename "$src")

    if [ ! -f "$src" ]; then
        return
    fi

    # Only copy actual Mach-O binaries (skip shell scripts / wrappers)
    if ! file "$src" 2>/dev/null | grep -q 'Mach-O'; then
        echo -e "  ${RED}skip${NC} $name (not Mach-O)"
        return
    fi

    cp "$src" "$dest_dir/$name"
    echo -e "  ${GREEN}copy${NC} $name"
}

echo -e "\n${CYAN}═══ Xcode Toolchain ═══${NC}"
XCODE_TC_TOOLS=(
    lipo size strings nm strip vtool
    libtool ranlib ar ld
    install_name_tool bitcode_strip nmedit segedit
    codesign_allocate
    llvm-objdump llvm-nm llvm-size llvm-strings
    llvm-readelf llvm-readobj llvm-otool
    llvm-libtool-darwin llvm-dwarfdump llvm-profdata
    llvm-cov dsymutil llvm-cxxfilt
    swift-demangle swift-api-digester swift-symbolgraph-extract
    clang clang++ clangd
    tapi unifdef indent
)
for tool in "${XCODE_TC_TOOLS[@]}"; do
    copy_tool "$XCODE_TC/$tool" "$OUT/xcode-tc"
done

echo -e "\n${CYAN}═══ Xcode Developer ═══${NC}"
XCODE_DEV_TOOLS=(
    xcodebuild xcrun genstrings
    ibtool actool stapler notarytool altool agvtool
    plutil
    xcstringstool
)
for tool in "${XCODE_DEV_TOOLS[@]}"; do
    copy_tool "$XCODE_DEV/$tool" "$OUT/xcode-dev"
done

echo -e "\n${CYAN}═══ System Tools ═══${NC}"
SYSTEM_TOOLS=(
    /usr/bin/grep /usr/bin/sed /usr/bin/awk
    /usr/bin/sort /usr/bin/uniq /usr/bin/wc /usr/bin/cut
    /usr/bin/find /usr/bin/stat /usr/bin/file
    /usr/bin/xattr /usr/bin/xargs
    /usr/bin/basename /usr/bin/dirname
    /usr/bin/tar /usr/bin/gzip /usr/bin/zip /usr/bin/unzip
    /usr/bin/uname /usr/bin/sw_vers /usr/bin/arch
    /usr/bin/id /usr/bin/whoami /usr/bin/hostname /usr/bin/env
    /usr/bin/defaults /usr/bin/plutil
    /usr/bin/nslookup
    /usr/bin/openssl /usr/bin/shasum
    /usr/bin/diff /usr/bin/head /usr/bin/tail
)
for tool in "${SYSTEM_TOOLS[@]}"; do
    copy_tool "$tool" "$OUT/system"
done

# Summary
TC_COUNT=$(ls "$OUT/xcode-tc" 2>/dev/null | wc -l | tr -d ' ')
DEV_COUNT=$(ls "$OUT/xcode-dev" 2>/dev/null | wc -l | tr -d ' ')
SYS_COUNT=$(ls "$OUT/system" 2>/dev/null | wc -l | tr -d ' ')
TOTAL=$((TC_COUNT + DEV_COUNT + SYS_COUNT))

echo ""
echo -e "${CYAN}Summary:${NC}"
echo "  Xcode Toolchain: $TC_COUNT tools"
echo "  Xcode Developer: $DEV_COUNT tools"
echo "  System:          $SYS_COUNT tools"
echo "  Total:           $TOTAL tools"
echo ""
echo "  Output directory: $OUT"
echo "  Total size: $(du -sh "$OUT" | cut -f1)"
echo ""
echo "Next: build the Docker image:"
echo "  docker build --platform linux/arm64 -f docker/Dockerfile.xcode-test -t burdettie-xcode-test ."
