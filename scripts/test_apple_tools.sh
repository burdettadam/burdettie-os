#!/bin/bash
# test_apple_tools.sh — Aggressive compatibility testing of Apple CLI tools
# through the darwin-loader to expose missing features and gaps.
#
# Usage: ./scripts/test_apple_tools.sh [category]
# Categories: all, xcode-toolchain, xcode-dev, system, apple-frameworks

set -euo pipefail

LOADER="/tmp/burdettieOS-target/release/macho-inspect"
XCODE_TC="/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin"
XCODE_DEV="/Applications/Xcode.app/Contents/Developer/usr/bin"
TMPFILE=$(mktemp /tmp/loader_test.XXXXXX)
RESULTS_FILE="/tmp/apple_tools_test_results.txt"

# Counters
PASS=0
FAIL=0
CRASH=0
SKIP=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

cleanup() { rm -f "$TMPFILE" "$TMPFILE.err"; }
trap cleanup EXIT

# Run a tool through the loader and classify the result
# Args: $1=binary_path $2=category $3=args... (space-separated)
test_tool() {
    local bin="$1"
    local category="$2"
    shift 2
    local args="$*"
    local name=$(basename "$bin")

    # Skip if not a Mach-O file
    if ! file "$bin" 2>/dev/null | grep -q 'Mach-O'; then
        return
    fi

    # Skip shell scripts and universal wrappers that just exec
    if file "$bin" 2>/dev/null | grep -q 'shell script\|text'; then
        return
    fi

    printf "  %-35s " "$name"

    # Run with 10-second timeout (perl-based for macOS compatibility)
    local exit_code=0
    perl -e 'alarm 10; exec @ARGV' "$LOADER" --exec "$bin" --exec-args $args >"$TMPFILE" 2>"$TMPFILE.err" || exit_code=$?

    local stdout_size=$(wc -c < "$TMPFILE" | tr -d ' ')
    local stderr_content=$(cat "$TMPFILE.err")
    local stdout_head=$(head -c 200 "$TMPFILE")

    # Classify result
    if [ $exit_code -eq 124 ] || [ $exit_code -eq 142 ]; then
        # Timeout (124 = timeout cmd, 142 = SIGALRM from perl)
        printf "${YELLOW}TIMEOUT${NC}  (hung after 10s)\n"
        echo "$category|$name|TIMEOUT|$exit_code|hung after 10s" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    elif [ $exit_code -eq 139 ]; then
        # SIGSEGV
        local crash_detail=$(echo "$stderr_content" | grep -oE 'SIGSEGV|signal 11|segfault|unresolved.*stub|unknown selector' | head -1)
        [ -z "$crash_detail" ] && crash_detail="SIGSEGV"
        printf "${RED}CRASH${NC}    (SIGSEGV) %s\n" "$crash_detail"
        echo "$category|$name|CRASH|139|$crash_detail" >> "$RESULTS_FILE"
        ((CRASH++)) || true
    elif [ $exit_code -eq 134 ]; then
        # SIGABRT
        local abort_detail=$(echo "$stderr_content" | grep -oE 'unrecognized selector|uncaught exception|abort|Assertion' | head -1)
        [ -z "$abort_detail" ] && abort_detail="SIGABRT"
        printf "${RED}ABORT${NC}    (SIGABRT) %s\n" "$abort_detail"
        echo "$category|$name|ABORT|134|$abort_detail" >> "$RESULTS_FILE"
        ((CRASH++)) || true
    elif [ $exit_code -eq 136 ]; then
        # SIGFPE
        printf "${RED}CRASH${NC}    (SIGFPE)\n"
        echo "$category|$name|CRASH|136|SIGFPE" >> "$RESULTS_FILE"
        ((CRASH++)) || true
    elif [ $exit_code -eq 138 ]; then
        # SIGBUS
        printf "${RED}CRASH${NC}    (SIGBUS)\n"
        echo "$category|$name|CRASH|138|SIGBUS" >> "$RESULTS_FILE"
        ((CRASH++)) || true
    elif echo "$stderr_content" | grep -qi 'dependency.*not found\|failed to load\|missing.*framework\|bad magic'; then
        local dep_detail=$(echo "$stderr_content" | grep -oiE 'dependency.*not found.*|failed to load.*|missing.*framework.*|bad magic.*' | head -1 | head -c 100)
        printf "${YELLOW}DEP${NC}      %s\n" "$dep_detail"
        echo "$category|$name|DEP_FAIL|$exit_code|$dep_detail" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    elif [ $exit_code -le 2 ] || [ $exit_code -eq 64 ]; then
        # 0, 1, 2 = normal exits; 64 = EX_USAGE (usage error, but tool ran)
        local output_preview=$(echo "$stdout_head" | tr '\n' ' ' | head -c 80)
        printf "${GREEN}PASS${NC}     (exit %d, %d bytes) %s\n" "$exit_code" "$stdout_size" "$output_preview"
        echo "$category|$name|PASS|$exit_code|${stdout_size}B output" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        local err_preview=$(echo "$stderr_content" | head -c 120 | tr '\n' ' ')
        printf "${YELLOW}FAIL${NC}     (exit %d) %s\n" "$exit_code" "$err_preview"
        echo "$category|$name|FAIL|$exit_code|$err_preview" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi
}

# ── Test Categories ─────────────────────────────────────────────

test_xcode_toolchain() {
    echo -e "\n${CYAN}═══ Xcode Toolchain Tools ($XCODE_TC) ═══${NC}"
    echo "Testing compilation, linking, and developer toolchain utilities..."

    # Already tested (should still pass)
    test_tool "$XCODE_TC/lipo" "xcode-tc" "-info" "/usr/bin/true"
    test_tool "$XCODE_TC/size" "xcode-tc" "/usr/bin/true"
    test_tool "$XCODE_TC/strings" "xcode-tc" "/usr/bin/true"
    test_tool "$XCODE_TC/nm" "xcode-tc" "/usr/bin/true"
    test_tool "$XCODE_TC/strip" "xcode-tc"
    test_tool "$XCODE_TC/vtool" "xcode-tc" "-help"

    # New: More cctools
    test_tool "$XCODE_TC/libtool" "xcode-tc" "-V"
    test_tool "$XCODE_TC/ranlib" "xcode-tc"
    test_tool "$XCODE_TC/ar" "xcode-tc" "--version"
    test_tool "$XCODE_TC/as" "xcode-tc" "--version"
    test_tool "$XCODE_TC/ld" "xcode-tc" "-v"
    test_tool "$XCODE_TC/install_name_tool" "xcode-tc"
    test_tool "$XCODE_TC/bitcode_strip" "xcode-tc"
    test_tool "$XCODE_TC/nmedit" "xcode-tc"
    test_tool "$XCODE_TC/segedit" "xcode-tc"
    test_tool "$XCODE_TC/pagestuff" "xcode-tc"
    test_tool "$XCODE_TC/redo_prebinding" "xcode-tc"
    test_tool "$XCODE_TC/checksyms" "xcode-tc"
    test_tool "$XCODE_TC/cmpdylib" "xcode-tc"
    test_tool "$XCODE_TC/codesign_allocate" "xcode-tc"

    # New: LLVM tools
    test_tool "$XCODE_TC/llvm-objdump" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-nm" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-size" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-strings" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-readelf" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-readobj" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-otool" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-libtool-darwin" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-dwarfdump" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-profdata" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-cov" "xcode-tc" "--version"
    test_tool "$XCODE_TC/dsymutil" "xcode-tc" "--version"
    test_tool "$XCODE_TC/llvm-cxxfilt" "xcode-tc" "--version"

    # New: Swift tools
    test_tool "$XCODE_TC/swift-demangle" "xcode-tc"
    test_tool "$XCODE_TC/swift-api-digester" "xcode-tc" "--help"
    test_tool "$XCODE_TC/swift-symbolgraph-extract" "xcode-tc" "--help"

    # New: Clang/compiler tools
    test_tool "$XCODE_TC/clang" "xcode-tc" "--version"
    test_tool "$XCODE_TC/clang++" "xcode-tc" "--version"
    test_tool "$XCODE_TC/clangd" "xcode-tc" "--version"
    test_tool "$XCODE_TC/c-index-test" "xcode-tc" "--version"
    test_tool "$XCODE_TC/sourcekit-lsp" "xcode-tc" "--help"

    # New: Misc developer tools
    test_tool "$XCODE_TC/tapi" "xcode-tc" "--version"
    test_tool "$XCODE_TC/tapi-analyze" "xcode-tc" "--help"
    test_tool "$XCODE_TC/unifdef" "xcode-tc" "--version"
    test_tool "$XCODE_TC/indent" "xcode-tc" "--version"
}

test_xcode_dev() {
    echo -e "\n${CYAN}═══ Xcode Developer Tools ($XCODE_DEV) ═══${NC}"
    echo "Testing build system, project management, and developer utilities..."

    # Build tools
    test_tool "$XCODE_DEV/xcodebuild" "xcode-dev" "-version"
    test_tool "$XCODE_DEV/xcrun" "xcode-dev" "--version"
    test_tool "$XCODE_DEV/xcode-select" "xcode-dev" "--version"
    test_tool "$XCODE_DEV/genstrings" "xcode-dev"
    test_tool "$XCODE_DEV/ibtool" "xcode-dev" "--version"
    test_tool "$XCODE_DEV/actool" "xcode-dev" "--version"
    test_tool "$XCODE_DEV/stapler" "xcode-dev"
    test_tool "$XCODE_DEV/notarytool" "xcode-dev" "--help"
    test_tool "$XCODE_DEV/altool" "xcode-dev" "--help"
    test_tool "$XCODE_DEV/agvtool" "xcode-dev"

    # Package/archive tools
    test_tool "$XCODE_DEV/pkgbuild" "xcode-dev" "--help"
    test_tool "$XCODE_DEV/productbuild" "xcode-dev" "--help"
    test_tool "$XCODE_DEV/productsign" "xcode-dev" "--help"
    test_tool "$XCODE_DEV/lsdistcc" "xcode-dev" "--help"

    # Testing tools
    test_tool "$XCODE_DEV/xcsresulttool" "xcode-dev" "--help"
    test_tool "$XCODE_DEV/xctest" "xcode-dev" "--help"
    test_tool "$XCODE_DEV/xctrace" "xcode-dev" "--help"

    # Instruments/profiling
    test_tool "$XCODE_DEV/instruments" "xcode-dev" "-s"
    test_tool "$XCODE_DEV/heap" "xcode-dev"
    test_tool "$XCODE_DEV/leaks" "xcode-dev"
    test_tool "$XCODE_DEV/vmmap" "xcode-dev"
    test_tool "$XCODE_DEV/malloc_history" "xcode-dev"
    test_tool "$XCODE_DEV/filtercalltree" "xcode-dev"

    # Localization
    test_tool "$XCODE_DEV/plutil" "xcode-dev" "-help"
    test_tool "$XCODE_DEV/plistbuddy" "xcode-dev" "--help"

    # Simulator
    test_tool "$XCODE_DEV/simctl" "xcode-dev" "--help"

    # Code signing
    test_tool "$XCODE_DEV/codesign" "xcode-dev" "--help"

    # Metal/GPU
    test_tool "$XCODE_DEV/metal" "xcode-dev" "--version"
    test_tool "$XCODE_DEV/metallib" "xcode-dev" "--help"
}

test_system_tools() {
    echo -e "\n${CYAN}═══ System CLI Tools (/usr/bin) ═══${NC}"
    echo "Testing Apple system utilities..."

    # Text/file processing
    test_tool "/usr/bin/grep" "system" "--version"
    test_tool "/usr/bin/sed" "system" "--version"
    test_tool "/usr/bin/awk" "system" "--version"
    test_tool "/usr/bin/sort" "system" "--version"
    test_tool "/usr/bin/uniq" "system" "--version"
    test_tool "/usr/bin/wc" "system" "--version"
    test_tool "/usr/bin/cut" "system" "--version"
    test_tool "/usr/bin/tr" "system" "--help"
    test_tool "/usr/bin/head" "system" "--help"
    test_tool "/usr/bin/tail" "system" "--help"
    test_tool "/usr/bin/tee" "system" "--help"
    test_tool "/usr/bin/diff" "system" "--version"
    test_tool "/usr/bin/comm" "system" "--help"
    test_tool "/usr/bin/paste" "system" "--help"
    test_tool "/usr/bin/expand" "system" "--help"

    # File system
    test_tool "/usr/bin/find" "system" "--version"
    test_tool "/usr/bin/stat" "system" "-f" "%z" "/usr/bin/true"
    test_tool "/usr/bin/file" "system" "/usr/bin/true"
    test_tool "/usr/bin/xattr" "system" "-l" "/usr/bin/true"
    test_tool "/usr/bin/mdls" "system" "-name" "kMDItemFSName" "/usr/bin/true"
    test_tool "/usr/bin/mdfind" "system" "-name" "this_should_not_exist_12345"
    test_tool "/usr/bin/xargs" "system" "--help"
    test_tool "/usr/bin/basename" "system" "/foo/bar.txt"
    test_tool "/usr/bin/dirname" "system" "/foo/bar.txt"
    test_tool "/usr/bin/readlink" "system" "/usr/bin/true"

    # Security/crypto
    test_tool "/usr/bin/openssl" "system" "version"
    test_tool "/usr/bin/ssh-keygen" "system" "--help"
    test_tool "/usr/bin/security" "system" "help"
    test_tool "/usr/bin/shasum" "system" "--help"

    # Archive/compression
    test_tool "/usr/bin/tar" "system" "--version"
    test_tool "/usr/bin/gzip" "system" "--version"
    test_tool "/usr/bin/zip" "system" "--help"
    test_tool "/usr/bin/unzip" "system" "--help"
    test_tool "/usr/bin/bzip2" "system" "--help"
    test_tool "/usr/bin/xz" "system" "--version"

    # System info
    test_tool "/usr/bin/uname" "system" "-a"
    test_tool "/usr/bin/sw_vers" "system"
    test_tool "/usr/bin/arch" "system"
    test_tool "/usr/bin/sysctl" "system" "-n" "hw.ncpu"
    test_tool "/usr/bin/vm_stat" "system"
    test_tool "/usr/bin/hostinfo" "system"
    test_tool "/usr/bin/uptime" "system"
    test_tool "/usr/bin/id" "system"
    test_tool "/usr/bin/whoami" "system"
    test_tool "/usr/bin/hostname" "system"
    test_tool "/usr/bin/env" "system"

    # Process management
    test_tool "/usr/bin/ps" "system" "-p" "1"
    test_tool "/usr/bin/top" "system" "-l" "1" "-n" "0"

    # Networking
    test_tool "/usr/bin/dscl" "system" "." "-list" "/"
    test_tool "/usr/bin/nslookup" "system" "localhost"
    test_tool "/usr/bin/dig" "system" "-v"

    # macOS-specific
    test_tool "/usr/bin/defaults" "system" "read-type" "NSGlobalDomain" "AppleLanguages"
    test_tool "/usr/bin/pbcopy" "system" "--help"
    test_tool "/usr/bin/pbpaste" "system"
    test_tool "/usr/bin/open" "system" "--help"
    test_tool "/usr/bin/osascript" "system" "--help"
    test_tool "/usr/bin/plutil" "system" "-help"
    test_tool "/usr/bin/log" "system" "help"
    test_tool "/usr/bin/profiles" "system" "help"

    # Developer
    test_tool "/usr/bin/lldb" "system" "--version"
    test_tool "/usr/bin/otool" "system" "-h" "/usr/bin/true"
    test_tool "/usr/bin/codesign" "system" "--help"
    test_tool "/usr/bin/dwarfdump" "system" "--version"
}

test_apple_frameworks() {
    echo -e "\n${CYAN}═══ Apple Framework Tools ═══${NC}"
    echo "Testing tools that use Apple frameworks (CoreFoundation, Security, etc.)..."

    # CoreServices/Launch Services
    test_tool "/usr/bin/lsregister" "frameworks" "--help"
    test_tool "/usr/bin/mdutil" "frameworks" "-s" "/"
    test_tool "/usr/bin/mdimport" "frameworks" "--help"

    # DiskArbitration
    test_tool "/usr/bin/diskutil" "frameworks" "list"

    # SystemConfiguration
    test_tool "/usr/bin/scutil" "frameworks" "--help"

    # IOKit
    test_tool "/usr/bin/ioreg" "frameworks" "-l" "-d" "1" "-c" "IOPlatformExpertDevice"
    test_tool "/usr/bin/system_profiler" "frameworks" "-listDataTypes"

    # Directory Services
    test_tool "/usr/bin/dscacheutil" "frameworks" "-q" "host" "-a" "name" "localhost"
}

# ── Main ────────────────────────────────────────────────────────────

echo "================================================================"
echo "  Apple CLI Tool Compatibility Test Suite"
echo "  Loader: $LOADER"
echo "  Date: $(date '+%Y-%m-%d %H:%M')"
echo "================================================================"

# Clear previous results
> "$RESULTS_FILE"

CATEGORY="${1:-all}"

case "$CATEGORY" in
    xcode-toolchain) test_xcode_toolchain ;;
    xcode-dev) test_xcode_dev ;;
    system) test_system_tools ;;
    apple-frameworks|frameworks) test_apple_frameworks ;;
    all)
        test_xcode_toolchain
        test_xcode_dev
        test_system_tools
        test_apple_frameworks
        ;;
    *) echo "Unknown category: $CATEGORY"; exit 1 ;;
esac

echo ""
echo "================================================================"
echo "  RESULTS SUMMARY"
echo "================================================================"
echo -e "  ${GREEN}PASS${NC}:  $PASS"
echo -e "  ${RED}CRASH${NC}: $CRASH"
echo -e "  ${YELLOW}FAIL${NC}:  $FAIL"
echo "  TOTAL: $((PASS + CRASH + FAIL))"
echo ""

# Breakdown by failure mode
echo "── Crash Analysis ──"
if grep -q 'CRASH\|ABORT' "$RESULTS_FILE" 2>/dev/null; then
    echo "  Signal breakdown:"
    grep 'CRASH\|ABORT' "$RESULTS_FILE" | awk -F'|' '{print $5}' | sort | uniq -c | sort -rn | head -20
    echo ""
    echo "  Crashed tools:"
    grep 'CRASH\|ABORT' "$RESULTS_FILE" | awk -F'|' '{printf "    %-30s %s  %s\n", $2, $3, $5}'
fi

echo ""
echo "── Dependency Failures ──"
if grep -q 'DEP_FAIL' "$RESULTS_FILE" 2>/dev/null; then
    grep 'DEP_FAIL' "$RESULTS_FILE" | awk -F'|' '{printf "    %-30s %s\n", $2, $5}'
fi

echo ""
echo "── Non-Crash Failures ──"
if grep -q '|FAIL|' "$RESULTS_FILE" 2>/dev/null; then
    grep '|FAIL|' "$RESULTS_FILE" | awk -F'|' '{printf "    %-30s exit=%s  %s\n", $2, $4, $5}'
fi

echo ""
echo "── Passing Tools ──"
grep '|PASS|' "$RESULTS_FILE" 2>/dev/null | awk -F'|' '{printf "    %-30s exit=%s\n", $2, $4}' || echo "    (none)"

echo ""
echo "Full results: $RESULTS_FILE"
