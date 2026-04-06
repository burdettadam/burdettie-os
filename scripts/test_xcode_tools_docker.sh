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
            grep|sed|sort|uniq|wc|cut|gzip|diff|bzip2)
                           args="" ;;
            tar)           args="-tf /dev/null" ;;
            awk)           args="--version" ;;
            find)          args="/tmp -maxdepth 0 -print" ;;
            file)          args="/etc/hostname" ;;
            stat)          args="/" ;;
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

# ── Functional output-validation tests ──────────────────────────────

# Extract just the tool output between the output markers.
# The loader prints lines containing " output " before and " end " after guest output.
extract_output() {
    LC_ALL=C awk '/  .* output .*/{found=1; next} /  .* end .*/{found=0} found'
}

# Run a functional test: execute tool, validate output contains expected string.
# Args: $1=description $2=binary $3=expected_grep_pattern $4...=args
test_functional() {
    local desc="$1"
    local bin="$2"
    local pattern="$3"
    shift 3
    local args="$*"

    printf "  %-35s " "$desc"

    if [ ! -f "$bin" ]; then
        printf "${YELLOW}SKIP${NC}     (binary not found)\n"
        echo "functional|$desc|SKIP|0|binary not found" >> "$RESULTS_FILE"
        return
    fi

    local exit_code=0
    local raw_out
    if [ -z "$args" ]; then
        raw_out=$(timeout 15 "$LOADER" --exec "$bin" 2>/dev/null) || exit_code=$?
    else
        raw_out=$(timeout 15 "$LOADER" --exec "$bin" --exec-args $args 2>/dev/null) || exit_code=$?
    fi

    if [ $exit_code -eq 124 ]; then
        printf "${YELLOW}TIMEOUT${NC}  (hung after 15s)\n"
        echo "functional|$desc|TIMEOUT|$exit_code|hung" >> "$RESULTS_FILE"
        ((FAIL++)) || true
        return
    fi

    if [ $exit_code -ge 128 ]; then
        printf "${RED}CRASH${NC}    (signal %d)\n" "$((exit_code - 128))"
        echo "functional|$desc|CRASH|$exit_code|signal" >> "$RESULTS_FILE"
        ((CRASH++)) || true
        return
    fi

    # Extract tool output
    local tool_out
    tool_out=$(echo "$raw_out" | extract_output)

    if echo "$tool_out" | grep -qE "$pattern"; then
        local preview
        preview=$(echo "$tool_out" | head -1 | head -c 60)
        printf "${GREEN}PASS${NC}     %s\n" "$preview"
        echo "functional|$desc|PASS|$exit_code|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        local preview
        preview=$(echo "$tool_out" | head -1 | head -c 60)
        printf "${YELLOW}WRONG${NC}    output: %s\n" "$preview"
        echo "functional|$desc|WRONG_OUTPUT|$exit_code|expected /$pattern/" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi
}

# Run a functional test that checks output line/byte count meets a minimum.
# Args: $1=description $2=binary $3=min_lines $4...=args
test_functional_minlines() {
    local desc="$1"
    local bin="$2"
    local min_lines="$3"
    shift 3
    local args="$*"

    printf "  %-35s " "$desc"

    if [ ! -f "$bin" ]; then
        printf "${YELLOW}SKIP${NC}     (binary not found)\n"
        echo "functional|$desc|SKIP|0|binary not found" >> "$RESULTS_FILE"
        return
    fi

    local exit_code=0
    local raw_out
    raw_out=$(timeout 15 "$LOADER" --exec "$bin" --exec-args $args 2>/dev/null) || exit_code=$?

    if [ $exit_code -eq 124 ]; then
        printf "${YELLOW}TIMEOUT${NC}  (hung after 15s)\n"
        echo "functional|$desc|TIMEOUT|$exit_code|hung" >> "$RESULTS_FILE"
        ((FAIL++)) || true
        return
    fi

    if [ $exit_code -ge 128 ]; then
        printf "${RED}CRASH${NC}    (signal %d)\n" "$((exit_code - 128))"
        echo "functional|$desc|CRASH|$exit_code|signal" >> "$RESULTS_FILE"
        ((CRASH++)) || true
        return
    fi

    local tool_out
    tool_out=$(echo "$raw_out" | extract_output)
    local actual_lines
    actual_lines=$(echo "$tool_out" | wc -l | tr -d ' ')

    if [ "$actual_lines" -ge "$min_lines" ]; then
        printf "${GREEN}PASS${NC}     (%d lines)\n" "$actual_lines"
        echo "functional|$desc|PASS|$exit_code|${actual_lines} lines" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    (%d lines, need %d)\n" "$actual_lines" "$min_lines"
        echo "functional|$desc|WRONG_OUTPUT|$exit_code|${actual_lines} lines < ${min_lines}" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi
}

test_functional_suite() {
    echo -e "\n${CYAN}═══ Functional Output Verification ═══${NC}"

    # Create test fixtures
    echo -e "cherry\napple\nbanana" > /tmp/func_sort.txt
    echo -e "hello world\nfoo bar\nhello again" > /tmp/func_grep.txt
    echo -e "line1\nline2\nline3" > /tmp/func_lines.txt
    echo "hello" > /tmp/func_hash.txt
    echo "test content for compression" > /tmp/func_compress.txt
    echo '<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>name</key><string>test</string></dict></plist>' > /tmp/func_plist.xml

    local TC="$TOOLS_DIR/xcode-tc"
    local SYS="$TOOLS_DIR/system"
    local DEV="$TOOLS_DIR/xcode-dev"
    local LIPO="$TC/lipo"

    # ── Xcode Toolchain: real binary analysis ──
    test_functional "lipo -detailed_info"  "$TC/lipo" "fat_magic|architecture" \
        -detailed_info "$LIPO"

    test_functional_minlines "strings <binary>"  "$TC/strings" 100 "$LIPO"

    test_functional_minlines "nm <binary>"  "$TC/nm" 50 "$LIPO"

    test_functional "size segments"  "$TC/size" "__TEXT.*__DATA" "$LIPO"

    test_functional "clang --version"  "$TC/clang" "clang version" --version

    test_functional "swift-demangle pipe"  "$TC/swift-demangle" "hello" \
        '_$s4main5helloyyF'

    test_functional "llvm-cxxfilt demangle"  "$TC/llvm-cxxfilt" "foo" \
        _Z3foov

    test_functional "vtool show"  "$TC/vtool" "minos|platform|sdk" \
        -show /opt/xcode-tools/xcode-tc/lipo

    # ── System tools: real file operations ──
    test_functional "basename path"  "$SYS/basename" "baz\\.txt" \
        /foo/bar/baz.txt

    test_functional "dirname path"  "$SYS/dirname" "/foo/bar" \
        /foo/bar/baz.txt

    test_functional "stat file size"  "$SYS/stat" "^[0-9]" \
        -f "%z" /tmp/func_lines.txt

    test_functional "file identify"  "$SYS/file" "." \
        /opt/xcode-tools/xcode-tc/lipo

    test_functional "uname kernel"  "$SYS/uname" "Darwin" -s

    test_functional "id output"  "$SYS/id" "uid=" ""

    test_functional "openssl version"  "$SYS/openssl" "LibreSSL|OpenSSL" version

    # ── System tools: data processing ──
    # tar create — just check exit code (no stdout)
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args cf /tmp/func_test.tar /tmp/func_compress.txt >/dev/null 2>/dev/null
    # Now verify by listing the archive
    test_functional "tar create+list"  "$SYS/tar" "func_compress" \
        tf /tmp/func_test.tar

    test_functional "zip create"  "$SYS/zip" "adding.*stored|adding.*deflated" \
        /tmp/func_test.zip /tmp/func_compress.txt
    if [ -f /tmp/func_test.zip ]; then
        test_functional "unzip list"  "$SYS/unzip" "func_compress\\.txt" \
            -l /tmp/func_test.zip
    fi

    test_functional "gzip compress"  "$SYS/gzip" "." \
        -c /tmp/func_compress.txt

    # ── System tools: text processing (regex) ──
    test_functional "grep match"  "$SYS/grep" "hello" \
        hello /tmp/func_grep.txt
    test_functional "grep count"  "$SYS/grep" "^2" \
        -c hello /tmp/func_grep.txt
    test_functional "sed subst"  "$SYS/sed" "goodbye" \
        "s/hello/goodbye/" /tmp/func_grep.txt

    # ── System tools: DNS resolution ──
    test_functional "nslookup DNS"  "$SYS/nslookup" "Address.*[0-9]+\\.[0-9]+" \
        github.com 192.168.65.7

    # ── Xcode Developer tools ──
    test_functional "xcodebuild -version"  "$DEV/xcodebuild" "Xcode" \
        -version

    test_functional "xcrun --version"  "$DEV/xcrun" "xcrun" --version

    # Clean up
    rm -f /tmp/func_*.txt /tmp/func_test.tar /tmp/func_test.zip
}

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
    xcode-tc)    test_xcode_tc ;;
    xcode-dev)   test_xcode_dev ;;
    system)      test_system ;;
    functional)  test_functional_suite ;;
    all)
        test_xcode_tc
        test_xcode_dev
        test_system
        test_functional_suite
        ;;
    *) echo "Unknown category: $CATEGORY (try: all, xcode-tc, xcode-dev, system, functional)"; exit 1 ;;
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
if grep -qE '\|FAIL\||\|WRONG_OUTPUT\|' "$RESULTS_FILE" 2>/dev/null; then
    echo "── Other Failures ──"
    grep -E '\|FAIL\||\|WRONG_OUTPUT\|' "$RESULTS_FILE" | awk -F'|' '{printf "    %-30s exit=%s  %s\n", $2, $4, $5}'
    echo ""
fi

# Passing tools
echo "── Passing Tools ──"
grep '|PASS|' "$RESULTS_FILE" 2>/dev/null | awk -F'|' '{printf "    %-30s exit=%s\n", $2, $4}' || echo "    (none)"

echo ""
echo "Full results: $RESULTS_FILE"
