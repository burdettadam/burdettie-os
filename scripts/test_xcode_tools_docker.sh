#!/bin/bash
# test_xcode_tools_docker.sh — Run Mach-O tool compatibility tests inside burdettie-os.
#
# This script iterates over extracted Xcode/system tool binaries and executes
# each through macho-inspect --exec, classifying results by outcome.
#
# Usage: test-xcode-tools [category]
# Categories: all, xcode-tc, xcode-dev, system

set -o pipefail

LOADER="macho-inspect"
TOOLS_DIR="/opt/xcode-tools"
TMPFILE=$(mktemp /tmp/loader_test.XXXXXX)
RESULTS_FILE="/tmp/burdettie_tool_results.txt"

PASS=0
FAIL=0
CRASH=0
SKIP=0
MISSING_SHIM=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

cleanup() { rm -f "$TMPFILE" "$TMPFILE.err"; }
trap cleanup EXIT

# Run a tool through the loader and classify the result.
# Args: $1=binary_path $2=category $3...=args
test_tool() {
    local bin="$1"
    local category="$2"
    shift 2
    local args="$*"
    local name
    name=$(basename "$bin")

    if [ ! -f "$bin" ]; then
        return
    fi

    printf "  %-35s " "$name"

    local exit_code=0
    timeout 15 "$LOADER" --exec "$bin" --exec-args $args >"$TMPFILE" 2>"$TMPFILE.err" || exit_code=$?

    local stdout_size
    stdout_size=$(wc -c < "$TMPFILE" | tr -d ' ')
    local stderr_content
    stderr_content=$(cat "$TMPFILE.err")
    local stdout_head
    stdout_head=$(head -c 200 "$TMPFILE")

    # Check for missing shim indicators in stderr
    local missing_shim=""
    if echo "$stderr_content" | grep -qiE 'unresolved.*stub|missing.*shim|unimplemented.*shim|stub.*not found'; then
        missing_shim=$(echo "$stderr_content" | grep -oiE 'unresolved.*stub[^"]*|missing.*shim[^"]*|stub.*not found[^"]*' | head -1 | head -c 120)
    fi

    if [ $exit_code -eq 124 ]; then
        printf "${YELLOW}TIMEOUT${NC}  (hung after 15s)\n"
        echo "$category|$name|TIMEOUT|$exit_code|hung after 15s" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    elif [ $exit_code -eq 139 ]; then
        if [ -n "$missing_shim" ]; then
            printf "${MAGENTA}SHIM${NC}     (SIGSEGV) %s\n" "$missing_shim"
            echo "$category|$name|MISSING_SHIM|139|$missing_shim" >> "$RESULTS_FILE"
            ((MISSING_SHIM++)) || true
        else
            local crash_detail
            crash_detail=$(echo "$stderr_content" | grep -oE 'SIGSEGV|signal 11|segfault' | head -1)
            [ -z "$crash_detail" ] && crash_detail="SIGSEGV"
            printf "${RED}CRASH${NC}    (SIGSEGV) %s\n" "$crash_detail"
            echo "$category|$name|CRASH|139|$crash_detail" >> "$RESULTS_FILE"
            ((CRASH++)) || true
        fi
    elif [ $exit_code -eq 134 ]; then
        if [ -n "$missing_shim" ]; then
            printf "${MAGENTA}SHIM${NC}     (SIGABRT) %s\n" "$missing_shim"
            echo "$category|$name|MISSING_SHIM|134|$missing_shim" >> "$RESULTS_FILE"
            ((MISSING_SHIM++)) || true
        else
            local abort_detail
            abort_detail=$(echo "$stderr_content" | grep -oE 'unrecognized selector|uncaught exception|abort|Assertion' | head -1)
            [ -z "$abort_detail" ] && abort_detail="SIGABRT"
            printf "${RED}ABORT${NC}    (SIGABRT) %s\n" "$abort_detail"
            echo "$category|$name|ABORT|134|$abort_detail" >> "$RESULTS_FILE"
            ((CRASH++)) || true
        fi
    elif [ $exit_code -eq 136 ] || [ $exit_code -eq 138 ]; then
        local sig_name="SIGFPE"
        [ $exit_code -eq 138 ] && sig_name="SIGBUS"
        printf "${RED}CRASH${NC}    (%s)\n" "$sig_name"
        echo "$category|$name|CRASH|$exit_code|$sig_name" >> "$RESULTS_FILE"
        ((CRASH++)) || true
    elif echo "$stderr_content" | grep -qi 'dependency.*not found\|failed to load\|missing.*framework\|bad magic'; then
        local dep_detail
        dep_detail=$(echo "$stderr_content" | grep -oiE 'dependency.*not found.*|failed to load.*|missing.*framework.*|bad magic.*' | head -1 | head -c 100)
        printf "${YELLOW}DEP${NC}      %s\n" "$dep_detail"
        echo "$category|$name|DEP_FAIL|$exit_code|$dep_detail" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    elif [ -n "$missing_shim" ]; then
        printf "${MAGENTA}SHIM${NC}     (exit %d) %s\n" "$exit_code" "$missing_shim"
        echo "$category|$name|MISSING_SHIM|$exit_code|$missing_shim" >> "$RESULTS_FILE"
        ((MISSING_SHIM++)) || true
    elif [ $exit_code -le 2 ] || [ $exit_code -eq 64 ]; then
        local output_preview
        output_preview=$(echo "$stdout_head" | tr '\n' ' ' | head -c 80)
        printf "${GREEN}PASS${NC}     (exit %d, %d bytes) %s\n" "$exit_code" "$stdout_size" "$output_preview"
        echo "$category|$name|PASS|$exit_code|${stdout_size}B output" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        local err_preview
        err_preview=$(echo "$stderr_content" | head -c 120 | tr '\n' ' ')
        printf "${YELLOW}FAIL${NC}     (exit %d) %s\n" "$exit_code" "$err_preview"
        echo "$category|$name|FAIL|$exit_code|$err_preview" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi
}

# ── Test suites ─────────────────────────────────────────────────────

# Dynamically test all tools in a directory with default args
test_dir() {
    local dir="$1"
    local category="$2"
    local label="$3"
    shift 3

    if [ ! -d "$dir" ]; then
        echo -e "  ${YELLOW}(no tools in $dir)${NC}"
        return
    fi

    echo -e "\n${CYAN}═══ $label ═══${NC}"

    for bin in "$dir"/*; do
        [ -f "$bin" ] || continue
        local name
        name=$(basename "$bin")

        # Choose sensible default args per tool
        local args=""
        case "$name" in
            lipo)          args="-info /opt/xcode-tools/xcode-tc/lipo" ;;  # self-inspect
            size|nm|strings|otool|llvm-nm|llvm-size|llvm-strings|llvm-otool|llvm-objdump|llvm-readobj|llvm-readelf)
                           args="/opt/xcode-tools/xcode-tc/lipo" ;;
            vtool|libtool|ranlib|strip|install_name_tool|bitcode_strip|nmedit|segedit|swift-demangle)
                           args="" ;;  # print usage
            ar)            args="--version" ;;
            ld)            args="-v" ;;
            clang|clang++|clangd|tapi|llvm-dwarfdump|llvm-profdata|llvm-cov|dsymutil|llvm-cxxfilt|llvm-libtool-darwin)
                           args="--version" ;;
            swift-api-digester|swift-symbolgraph-extract|sourcekit-lsp)
                           args="--help" ;;
            unifdef|indent) args="--version" ;;
            xcodebuild)    args="-version" ;;
            xcrun)         args="--version" ;;
            genstrings|agvtool|stapler)
                           args="" ;;
            ibtool|actool|plutil)
                           args="-help" ;;
            notarytool|altool)
                           args="--help" ;;
            xcstringstool)  args="" ;;
            grep|sed|sort|uniq|wc|cut|tar|gzip|diff|bzip2)
                           args="--version" ;;
            awk)           args="--version" ;;
            find)          args="--version" ;;
            file)          args="/opt/xcode-tools/xcode-tc/lipo" ;;
            stat)          args="/opt/xcode-tools/xcode-tc/lipo" ;;
            uname)         args="-a" ;;
            sw_vers|arch|hostinfo|uptime|id|whoami|hostname|vm_stat)
                           args="" ;;
            env)           args="" ;;
            openssl)       args="version" ;;
            shasum)        args="--help" ;;
            zip|unzip|xz)  args="--help" ;;
            basename)      args="/foo/bar.txt" ;;
            dirname)       args="/foo/bar.txt" ;;
            defaults)      args="help" ;;
            head|tail)     args="--help" ;;
            xattr|xargs)   args="--help" ;;
            *)             args="--help" ;;
        esac

        test_tool "$bin" "$category" $args
    done
}

test_xcode_tc()  { test_dir "$TOOLS_DIR/xcode-tc"  "xcode-tc"  "Xcode Toolchain Tools"; }
test_xcode_dev() { test_dir "$TOOLS_DIR/xcode-dev" "xcode-dev" "Xcode Developer Tools"; }
test_system()    { test_dir "$TOOLS_DIR/system"     "system"    "System CLI Tools"; }

# ── Main ────────────────────────────────────────────────────────────

echo "================================================================"
echo "  Burdettie OS — Mach-O Tool Compatibility Test"
echo "  Loader: $(which $LOADER)"
echo "  Platform: $(uname -m) $(cat /etc/burdettie-release 2>/dev/null || echo 'Linux')"
echo "  Date: $(date '+%Y-%m-%d %H:%M')"
echo "================================================================"

> "$RESULTS_FILE"

CATEGORY="${1:-all}"

case "$CATEGORY" in
    xcode-tc)  test_xcode_tc ;;
    xcode-dev) test_xcode_dev ;;
    system)    test_system ;;
    all)
        test_xcode_tc
        test_xcode_dev
        test_system
        ;;
    *) echo "Unknown category: $CATEGORY (try: all, xcode-tc, xcode-dev, system)"; exit 1 ;;
esac

echo ""
echo "================================================================"
echo "  RESULTS SUMMARY"
echo "================================================================"
echo -e "  ${GREEN}PASS${NC}:         $PASS"
echo -e "  ${RED}CRASH${NC}:        $CRASH"
echo -e "  ${MAGENTA}MISSING SHIM${NC}: $MISSING_SHIM"
echo -e "  ${YELLOW}FAIL${NC}:         $FAIL"
echo "  TOTAL:        $((PASS + CRASH + MISSING_SHIM + FAIL))"
echo ""

# Missing shim analysis — this is what needs to be implemented
if grep -q 'MISSING_SHIM' "$RESULTS_FILE" 2>/dev/null; then
    echo "── Missing Shims (need implementation) ──"
    grep 'MISSING_SHIM' "$RESULTS_FILE" | awk -F'|' '{printf "    %-30s %s\n", $2, $5}'
    echo ""
    echo "  Unique missing shims:"
    grep 'MISSING_SHIM' "$RESULTS_FILE" | awk -F'|' '{print $5}' | sort -u
    echo ""
fi

# Crash analysis
if grep -q 'CRASH\|ABORT' "$RESULTS_FILE" 2>/dev/null; then
    echo "── Crashes (bugs to fix) ──"
    grep 'CRASH\|ABORT' "$RESULTS_FILE" | awk -F'|' '{printf "    %-30s %s  %s\n", $2, $3, $5}'
    echo ""
fi

# Dependency failures
if grep -q 'DEP_FAIL' "$RESULTS_FILE" 2>/dev/null; then
    echo "── Dependency Failures ──"
    grep 'DEP_FAIL' "$RESULTS_FILE" | awk -F'|' '{printf "    %-30s %s\n", $2, $5}'
    echo ""
fi

# Other failures
if grep -q '|FAIL|' "$RESULTS_FILE" 2>/dev/null; then
    echo "── Other Failures ──"
    grep '|FAIL|' "$RESULTS_FILE" | awk -F'|' '{printf "    %-30s exit=%s  %s\n", $2, $4, $5}'
    echo ""
fi

# Passing tools
echo "── Passing Tools ──"
grep '|PASS|' "$RESULTS_FILE" 2>/dev/null | awk -F'|' '{printf "    %-30s exit=%s\n", $2, $4}' || echo "    (none)"

echo ""
echo "Full results: $RESULTS_FILE"
