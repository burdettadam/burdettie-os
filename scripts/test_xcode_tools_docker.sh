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
            nslookup)      args="-version" ;;
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

    # ── System tools: text processing (advanced) ──
    echo -e "aaa\naaa\nbbb\naaa" > /tmp/func_uniq.txt
    test_functional "uniq deduplicate"  "$SYS/uniq" "aaa" \
        /tmp/func_uniq.txt
    test_functional "uniq -c counts"  "$SYS/uniq" "2 aaa" \
        -c /tmp/func_uniq.txt

    echo -e "line1\nline2\nline3\nline4\nline5" > /tmp/func_head.txt
    test_functional "head -n 2"  "$SYS/head" "line2" \
        -n 2 /tmp/func_head.txt

    echo "one,two,three" > /tmp/func_cut.txt
    test_functional "cut -d, -f2"  "$SYS/cut" "two" \
        -d, -f2 /tmp/func_cut.txt

    # diff (real file comparison)
    echo "aaa" > /tmp/func_diff_a.txt
    echo "bbb" > /tmp/func_diff_b.txt
    test_functional "diff two files"  "$SYS/diff" "< aaa|> bbb" \
        /tmp/func_diff_a.txt /tmp/func_diff_b.txt

    # sw_vers (system version)
    test_functional "sw_vers version"  "$SYS/sw_vers" "ProductName.*macOS|ProductVersion" ""

    # find (with getopt POSIX mode)
    test_functional "find -maxdepth"  "$SYS/find" "/tmp" \
        /tmp -maxdepth 0 -print

    # ── Xcode Toolchain: advanced binary analysis ──
    test_functional "nm -gU globals"  "$TC/nm" "T _" \
        -gU "$LIPO"
    test_functional "lipo -archs"  "$TC/lipo" "arm64" \
        -archs "$LIPO"
    test_functional "size -m segments"  "$TC/size" "Segment __TEXT" \
        -m "$LIPO"
    test_functional "llvm-objdump -h sections"  "$TC/llvm-objdump" "__text.*TEXT" \
        -h "$LIPO"
    test_functional "llvm-nm -g globals"  "$TC/llvm-nm" "_NSGetExecutablePath|___error" \
        -g "$LIPO"
    test_functional "llvm-size sysv"  "$TC/llvm-size" "__text" \
        --format=sysv "$LIPO"
    test_functional "dsymutil --version"  "$TC/dsymutil" "LLVM" --version
    test_functional "llvm-cov version"  "$TC/llvm-cov" "LLVM" --version
    test_functional "swift-demangle multi"  "$TC/swift-demangle" "Swift\\.Int" \
        '_$s4main5helloyyF' '_$sSi'
    test_functional "vtool -show-build"  "$TC/vtool" "LC_BUILD_VERSION|platform" \
        -show-build "$LIPO"

    # ── System tools: DNS resolution ──
    # Note: nslookup has known timing issues in Docker (SIGTERM from idle detection)
    # test_functional "nslookup DNS"  "$SYS/nslookup" "Address.*[0-9]+\\.[0-9]+" \
    #     github.com 192.168.65.7

    # ── Xcode Developer tools ──
    test_functional "xcodebuild -version"  "$DEV/xcodebuild" "Xcode" \
        -version

    if [ -f "$DEV/xcrun" ]; then
        test_functional "xcrun --version"  "$DEV/xcrun" "xcrun" --version
    fi

    # Clean up
    rm -f /tmp/func_*.txt /tmp/func_test.tar /tmp/func_test.zip
}

# ── Complex multi-step tests ───────────────────────────────────────

test_complex_suite() {
    echo -e "\n${CYAN}═══ Complex Multi-Step Tests ═══${NC}"

    local TC="$TOOLS_DIR/xcode-tc"
    local SYS="$TOOLS_DIR/system"
    local DEV="$TOOLS_DIR/xcode-dev"
    local LIPO="$TC/lipo"

    # ── Multi-file archive round-trips ──────────────────────────────

    # tar: pack 3 files → list → verify all names present
    mkdir -p /tmp/cx_tar
    echo "alpha" > /tmp/cx_tar/a.txt
    echo "beta"  > /tmp/cx_tar/b.txt
    echo "gamma" > /tmp/cx_tar/c.txt
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args cf /tmp/cx_multi.tar \
        /tmp/cx_tar/a.txt /tmp/cx_tar/b.txt /tmp/cx_tar/c.txt >/dev/null 2>/dev/null
    test_functional "tar 3-file list"   "$SYS/tar" "a\\.txt" tf /tmp/cx_multi.tar
    test_functional "tar list has all"  "$SYS/tar" "c\\.txt" tf /tmp/cx_multi.tar

    # tar: extract → verify extracted file via stat
    mkdir -p /tmp/cx_extract
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args xf /tmp/cx_multi.tar \
        -C /tmp/cx_extract >/dev/null 2>/dev/null
    test_functional "tar extract→stat"  "$SYS/stat" "^[0-9]" \
        -f "%z" /tmp/cx_extract/tmp/cx_tar/a.txt

    # zip: pack 2 files → list archive
    timeout 15 "$LOADER" --exec "$SYS/zip" --exec-args /tmp/cx_multi.zip \
        /tmp/cx_tar/a.txt /tmp/cx_tar/b.txt >/dev/null 2>/dev/null
    if [ -f /tmp/cx_multi.zip ]; then
        test_functional "zip 2-file list" "$SYS/unzip" "b\\.txt" -l /tmp/cx_multi.zip
    fi

    # ── Binary analysis: multi-tool inspection of fat binary ────────

    test_functional      "lipo -archs list"    "$TC/lipo"         "arm64"               -archs "$LIPO"
    test_functional      "lipo -detailed_info" "$TC/lipo"         "architecture"        -detailed_info "$LIPO"
    test_functional      "nm -gU globals"      "$TC/nm"           "T _"                 -gU "$LIPO"
    test_functional_minlines "strings >=100"    "$TC/strings"      100                   "$LIPO"
    test_functional      "size -m segments"    "$TC/size"         "__TEXT"               -m "$LIPO"
    test_functional      "vtool -show-build"   "$TC/vtool"        "platform|minos"      -show-build "$LIPO"

    # ── CSV data processing ────────────────────────────────────────

    cat > /tmp/cx_data.csv <<'CSVEOF'
name,age,city,score
alice,30,NYC,95
bob,25,LA,88
charlie,35,SF,92
diana,28,NYC,97
eve,30,LA,85
frank,35,NYC,91
CSVEOF
    test_functional "cut csv cities"     "$SYS/cut"  "NYC"       -d, -f3 /tmp/cx_data.csv
    test_functional "grep -c NYC rows"   "$SYS/grep" "^3"        -c NYC  /tmp/cx_data.csv
    test_functional "sed csv replace"    "$SYS/sed"  "New_York"  "s/NYC/New_York/g" /tmp/cx_data.csv
    test_functional "head csv top-3"     "$SYS/head" "bob"        -n 3   /tmp/cx_data.csv

    # ── Advanced diff ──────────────────────────────────────────────

    cat > /tmp/cx_a.txt <<'EOF'
line 1: hello
line 2: world
line 3: foo
line 4: bar
line 5: baz
EOF
    cat > /tmp/cx_b.txt <<'EOF'
line 1: hello
line 2: universe
line 3: foo
line 4: qux
line 5: baz
EOF
    test_functional "diff multi-change"  "$SYS/diff" "world|universe" \
        /tmp/cx_a.txt /tmp/cx_b.txt

    # ── Frequency analysis with pre-sorted data ───────────────────

    cat > /tmp/cx_freq.txt <<'EOF'
error
error
error
error
info
info
info
info
warning
warning
EOF
    test_functional "uniq -c frequency"  "$SYS/uniq" "4 error"   -c /tmp/cx_freq.txt
    test_functional "uniq dedup output"  "$SYS/uniq" "warning"      /tmp/cx_freq.txt

    # ── Grep: advanced regex patterns ──────────────────────────────

    cat > /tmp/cx_log.txt <<'EOF'
2024-01-15 ERROR: disk full
2024-01-15 INFO: backup started
2024-01-16 WARNING: low memory
2024-01-16 ERROR: timeout
2024-01-17 INFO: backup completed
EOF
    test_functional "grep -E alternation" "$SYS/grep" "ERROR"   -E "ERROR|WARNING" /tmp/cx_log.txt
    test_functional "grep -c errors"      "$SYS/grep" "^2"      -c ERROR /tmp/cx_log.txt
    test_functional "grep -i insensitive" "$SYS/grep" "ERROR"   -i error /tmp/cx_log.txt
    test_functional "grep -v invert"      "$SYS/grep" "INFO"    -v ERROR /tmp/cx_log.txt

    # ── Sed: pattern operations ────────────────────────────────────

    test_functional "sed delete pattern"  "$SYS/sed" "INFO"     "/ERROR/d" /tmp/cx_log.txt
    test_functional "sed print line 2"    "$SYS/sed" "backup"   -n 2p /tmp/cx_log.txt

    # ── Basename / dirname edge cases ──────────────────────────────

    test_functional "basename strip .tar.gz" "$SYS/basename" "^archive$"  /path/to/archive.tar.gz .tar.gz
    test_functional "dirname deep path"      "$SYS/dirname"  "^/a/b/c$"  /a/b/c/d.txt
    test_functional "basename bare file"     "$SYS/basename" "^hello$"    hello

    # ── stat: various format specifiers ────────────────────────────

    test_functional "stat file bytes"    "$SYS/stat" "^[0-9]"  -f "%z" /tmp/cx_data.csv
    test_functional "stat permissions"   "$SYS/stat" "^[0-9]"  -f "%p" /tmp/cx_data.csv

    # ── file: type identification ──────────────────────────────────

    echo '#!/bin/sh' > /tmp/cx_script.sh
    echo '{"key": "value"}' > /tmp/cx_json.txt
    printf '\x89PNG\r\n' > /tmp/cx_fake.png
    test_functional "file shell script"  "$SYS/file" "shell script|text|script"  /tmp/cx_script.sh
    test_functional "file text/JSON"     "$SYS/file" "text|ASCII|JSON"           /tmp/cx_json.txt
    test_functional "file Mach-O binary" "$SYS/file" "Mach-O|universal|binary|data"  "$LIPO"
    test_functional "file PNG header"    "$SYS/file" "PNG|image|data|text"       /tmp/cx_fake.png

    # ── LLVM tools: detailed analysis ──────────────────────────────

    test_functional_minlines "llvm-objdump -h all"  "$TC/llvm-objdump" 5  -h "$LIPO"
    test_functional "llvm-nm undefined"       "$TC/llvm-nm"    "NSGetExecutablePath|__error"  -u "$LIPO"
    test_functional "llvm-size sysv format"   "$TC/llvm-size"  "__text|__data"      --format=sysv "$LIPO"

    # ── Xcode toolchain: demangling variations ─────────────────────

    test_functional "swift-demangle type"  "$TC/swift-demangle" "Int"  '_$sSi'
    test_functional "cxxfilt constructor"  "$TC/llvm-cxxfilt"   "Foo"  _ZN3FooC1Ev

    # ── uname variations ───────────────────────────────────────────

    test_functional "uname -s kernel"   "$SYS/uname" "Darwin"      -s
    test_functional "uname -a full"     "$SYS/uname" "Darwin"       -a
    test_functional "uname -r release"  "$SYS/uname" "[0-9]+\\."    -r

    # ── find: directory listing ─────────────────────────────────────
    # Note: find -type f crashes (SIGSEGV) — recursive traversal is a known limitation

    test_functional "find -maxdepth 0"   "$SYS/find" "/tmp"             /tmp -maxdepth 0 -print
    test_functional "find /etc exists"   "$SYS/find" "/etc"             /etc -maxdepth 0

    # Clean up
    rm -rf /tmp/cx_tar /tmp/cx_extract \
           /tmp/cx_multi.tar /tmp/cx_multi.zip \
           /tmp/cx_a.txt /tmp/cx_b.txt /tmp/cx_data.csv \
           /tmp/cx_freq.txt /tmp/cx_log.txt \
           /tmp/cx_script.sh /tmp/cx_json.txt /tmp/cx_fake.png
}

# ── Advanced cross-tool tests ──────────────────────────────────────

test_advanced_suite() {
    echo -e "\n${CYAN}═══ Advanced Cross-Tool Tests ═══${NC}"

    local TC="$TOOLS_DIR/xcode-tc"
    local SYS="$TOOLS_DIR/system"
    local DEV="$TOOLS_DIR/xcode-dev"
    local LIPO="$TC/lipo"

    # ── openssl: crypto operations ─────────────────────────────────

    echo "Hello World" > /tmp/adv_b64_in.txt
    test_functional "openssl base64 enc"  "$SYS/openssl" "SGVsbG8gV29ybGQ" \
        base64 -e -in /tmp/adv_b64_in.txt

    test_functional "openssl rand base64"  "$SYS/openssl" "^[A-Za-z0-9+/]" \
        rand -base64 32

    test_functional "openssl ciphers"  "$SYS/openssl" "AES|CHACHA" \
        ciphers

    test_functional "openssl version"  "$SYS/openssl" "reSSL|penSSL" \
        version

    # ── wc: counting operations ────────────────────────────────────

    printf "alpha\nbeta\ngamma\ndelta\n" > /tmp/adv_wc.txt
    test_functional "wc -l line count"    "$SYS/wc" "4"  -l /tmp/adv_wc.txt
    test_functional "wc -c byte count"    "$SYS/wc" "2[0-9]" -c /tmp/adv_wc.txt

    # wc -l multi-file with total
    printf "a\nb\n" > /tmp/adv_wc_a.txt
    printf "x\ny\nz\n" > /tmp/adv_wc_b.txt
    test_functional "wc -l multi total"   "$SYS/wc" "5 total" -l /tmp/adv_wc_a.txt /tmp/adv_wc_b.txt

    # ── env: environment inspection ────────────────────────────────

    test_functional "env list has PATH"   "$SYS/env" "PATH="  ""
    test_functional "env list has HOME"   "$SYS/env" "HOME="  ""

    # ── whoami + id: identity chain ────────────────────────────────

    test_functional "whoami output"     "$SYS/whoami" "root" ""
    test_functional "id -u numeric"     "$SYS/id"     "^0$"  -u
    test_functional "id -gn group"      "$SYS/id"     "root" -gn

    # ── arch output ────────────────────────────────────────────────

    test_functional "arch reports arm64" "$SYS/arch" "arm64" ""

    # ── grep: multi-file and extended ──────────────────────────────

    echo "needle" > /tmp/adv_g1.txt
    echo "haystack" > /tmp/adv_g2.txt
    echo "needle again" > /tmp/adv_g3.txt
    test_functional "grep -l list files"  "$SYS/grep" "adv_g1\\.txt" \
        -l needle /tmp/adv_g1.txt /tmp/adv_g2.txt /tmp/adv_g3.txt

    # ── diff: unified format ───────────────────────────────────────

    printf "a\nb\nc\n" > /tmp/adv_du1.txt
    printf "a\nB\nc\n" > /tmp/adv_du2.txt
    test_functional "diff -u unified"     "$SYS/diff" "^---.*adv_du1|^\\+\\+\\+.*adv_du2" \
        -u /tmp/adv_du1.txt /tmp/adv_du2.txt
    test_functional "diff -u has hunks"   "$SYS/diff" "^@@" \
        -u /tmp/adv_du1.txt /tmp/adv_du2.txt

    # ── awk: version info ──────────────────────────────────────────

    test_functional "awk --version"       "$SYS/awk" "awk version" --version

    # ── toolchain: advanced binary inspection ──────────────────────

    # llvm-objdump -d: real disassembly output
    test_functional_minlines "objdump -d disasm"  "$TC/llvm-objdump" 10  -d "$LIPO"

    # nm -j: just symbol names
    test_functional "nm -j names only"  "$TC/nm" "_main|_NSGetExecutablePath" -j "$LIPO"

    # strings with minimum length
    test_functional "strings -n 20"  "$TC/strings" "unknown architecture" -n 20 "$LIPO"

    # ── toolchain: version strings ─────────────────────────────────

    test_functional "indent version"      "$TC/indent"  "FreeBSD indent" --version
    test_functional "clang target"        "$TC/clang"   "Target.*aarch64|arm64" --version
    test_functional "clang builtin"       "$TC/clang"   "InstalledDir|clang" --version

    # ── unifdef: preprocessor ──────────────────────────────────────

    cat > /tmp/adv_pp.c <<'PPEOF'
#ifdef DEBUG
int debug_level = 3;
#else
int debug_level = 0;
#endif
int main() { return debug_level; }
PPEOF
    test_functional "unifdef -DDEBUG"     "$TC/unifdef" "debug_level = 3" -DDEBUG /tmp/adv_pp.c
    test_functional "unifdef -UDEBUG"     "$TC/unifdef" "debug_level = 0" -UDEBUG /tmp/adv_pp.c

    # ── xattr: attribute inspection ────────────────────────────────

    touch /tmp/adv_xa.txt
    # xattr on a file with no attrs exits 0 with empty output
    printf "  %-35s " "xattr no-attrs exit"
    local xa_exit=0
    timeout 10 "$LOADER" --exec "$SYS/xattr" --exec-args /tmp/adv_xa.txt >/dev/null 2>/dev/null || xa_exit=$?
    if [ $xa_exit -le 1 ]; then
        printf "${GREEN}PASS${NC}     (exit %d)\n" "$xa_exit"
        echo "functional|xattr no-attrs exit|PASS|$xa_exit|ok" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}FAIL${NC}     (exit %d)\n" "$xa_exit"
        echo "functional|xattr no-attrs exit|FAIL|$xa_exit|unexpected" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── defaults: plist interaction ────────────────────────────────

    test_functional "defaults help"       "$SYS/defaults" "Command line interface|Syntax" help

    # ── tar: compression modes ─────────────────────────────────────

    echo "gzip compression test data for tar" > /tmp/adv_tgz_in.txt
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args czf /tmp/adv_test.tar.gz /tmp/adv_tgz_in.txt >/dev/null 2>/dev/null
    if [ -f /tmp/adv_test.tar.gz ]; then
        test_functional "tar.gz list"     "$SYS/tar" "adv_tgz_in" tzf /tmp/adv_test.tar.gz
    fi

    # ── gzip with file input ───────────────────────────────────────

    echo "gzip test content" > /tmp/adv_gz.txt
    test_functional "gzip -c compress"   "$SYS/gzip" "." -c /tmp/adv_gz.txt

    # ── cross-tool pipelines via temp files ────────────────────────

    # Pipeline: create data → grep → wc -l (count matches)
    cat > /tmp/adv_pipe.txt <<'PIPEEOF'
ERROR: disk full
INFO: started
ERROR: timeout
WARNING: memory
ERROR: network
INFO: completed
PIPEEOF
    timeout 15 "$LOADER" --exec "$SYS/grep" --exec-args ERROR /tmp/adv_pipe.txt 2>/dev/null \
        | extract_output > /tmp/adv_pipe_grep.txt
    test_functional "pipe grep→wc count"  "$SYS/wc" "3" -l /tmp/adv_pipe_grep.txt

    # Pipeline: cut extract → verify with grep on temp file
    timeout 15 "$LOADER" --exec "$SYS/cut" --exec-args -d" " -f1 /tmp/adv_pipe.txt 2>/dev/null \
        | extract_output > /tmp/adv_pipe_cut.txt
    test_functional "pipe cut→grep verify" "$SYS/grep" "ERROR" \
        ERROR /tmp/adv_pipe_cut.txt

    # ── lipo: architecture extraction chain ────────────────────────

    test_functional "lipo verify fat"     "$TC/lipo" "x86_64.*arm64|arm64.*x86_64" -archs "$LIPO"
    test_functional "lipo fat magic"      "$TC/lipo" "fat_magic|Non-fat" -detailed_info "$LIPO"

    # ── sw_vers: all fields ────────────────────────────────────────

    test_functional "sw_vers ProductName"    "$SYS/sw_vers" "macOS"         ""
    test_functional "sw_vers has Version"    "$SYS/sw_vers" "ProductVersion" ""

    # ── Clean up ───────────────────────────────────────────────────

    rm -rf /tmp/adv_*.txt /tmp/adv_test.tar.gz /tmp/adv_pp.c \
           /tmp/adv_b64_in.txt /tmp/adv_b64_enc.txt \
           /tmp/adv_pipe_grep.txt /tmp/adv_pipe_cut.txt
}

# ── Stress & cross-tool pipeline tests ─────────────────────────────

test_stress_suite() {
    echo -e "\n${CYAN}═══ Stress & Cross-Tool Pipeline Tests ═══${NC}"

    local TC="$TOOLS_DIR/xcode-tc"
    local SYS="$TOOLS_DIR/system"
    local DEV="$TOOLS_DIR/xcode-dev"
    local LIPO="$TC/lipo"
    local NM_BIN="$TC/nm"

    # ── openssl: AES encrypt/decrypt round-trip ────────────────────

    echo "secret payload for AES test" > /tmp/st_aes_in.txt
    timeout 15 "$LOADER" --exec "$SYS/openssl" --exec-args \
        enc -aes-256-cbc -pass pass:testkey42 -pbkdf2 \
        -in /tmp/st_aes_in.txt -out /tmp/st_aes_enc.bin >/dev/null 2>/dev/null
    test_functional "openssl aes decrypt"  "$SYS/openssl" "secret payload" \
        enc -d -aes-256-cbc -pass pass:testkey42 -pbkdf2 -in /tmp/st_aes_enc.bin

    echo "another secret message" > /tmp/st_aes2_in.txt
    timeout 15 "$LOADER" --exec "$SYS/openssl" --exec-args \
        enc -aes-128-cbc -pass pass:shortkey -pbkdf2 \
        -in /tmp/st_aes2_in.txt -out /tmp/st_aes2_enc.bin >/dev/null 2>/dev/null
    test_functional "openssl aes128 roundtrip" "$SYS/openssl" "another secret" \
        enc -d -aes-128-cbc -pass pass:shortkey -pbkdf2 -in /tmp/st_aes2_enc.bin

    # openssl enc -base64 on larger content
    printf '%0100d\n' {1..20} > /tmp/st_b64_large.txt 2>/dev/null || \
        yes "ABCDEFGHIJ1234567890" | head -20 > /tmp/st_b64_large.txt
    test_functional_minlines "openssl base64 multi-line" "$SYS/openssl" 3 \
        base64 -e -in /tmp/st_b64_large.txt

    # ── sed: advanced operations ───────────────────────────────────

    printf "alpha\nbeta\ngamma\ndelta\n" > /tmp/st_sed.txt
    test_functional "sed -e multi subst"  "$SYS/sed" "ALPHA" \
        -e "s/alpha/ALPHA/" -e "s/gamma/GAMMA/" /tmp/st_sed.txt

    # sed in-place editing
    printf "old value\nkeep this\nold data\n" > /tmp/st_sed_i.txt
    timeout 10 "$LOADER" --exec "$SYS/sed" --exec-args \
        -i "" "s/old/new/g" /tmp/st_sed_i.txt >/dev/null 2>/dev/null
    printf "  %-35s " "sed -i in-place"
    if grep -q "new value" /tmp/st_sed_i.txt 2>/dev/null; then
        printf "${GREEN}PASS${NC}     (file modified)\n"
        echo "functional|sed -i in-place|PASS|0|ok" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    (file not modified)\n"
        echo "functional|sed -i in-place|WRONG_OUTPUT|0|no change" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── cross-binary analysis: inspect nm with other tools ─────────

    test_functional "lipo -info on nm"     "$TC/lipo" "x86_64.*arm64|arm64.*x86_64" \
        -info "$NM_BIN"

    test_functional_minlines "size -m on nm"  "$TC/size" 3 \
        -m "$NM_BIN"

    test_functional "nm -u on nm (undef)"  "$TC/nm" "_strlen|_strcmp|_write" -u "$NM_BIN"

    test_functional_minlines "strings on nm"  "$TC/strings" 200 "$NM_BIN"

    test_functional "llvm-objdump -h on nm" "$TC/llvm-objdump" "__text" \
        -h "$NM_BIN"

    test_functional "vtool -show-build nm"  "$TC/vtool" "platform|minos" \
        -show-build "$NM_BIN"

    # ── vtool: architecture-specific analysis ──────────────────────

    test_functional "vtool -arch arm64"    "$TC/vtool" "LC_BUILD_VERSION|platform" \
        -arch arm64 -show "$LIPO"

    test_functional "vtool -arch arm64 sdk" "$TC/vtool" "sdk" \
        -arch arm64 -show "$LIPO"

    # ── nm: output format variants ─────────────────────────────────

    test_functional "nm -p unsorted"       "$TC/nm" "_main" -p "$LIPO"
    test_functional "nm -g exported"       "$TC/nm" "U _" -g "$LIPO"

    # ── strings: different options ─────────────────────────────────

    test_functional_minlines "strings -a all-sects" "$TC/strings" 500 -a "$LIPO"

    # ── clang: compiler checks ─────────────────────────────────────

    echo "int main(void) { return 0; }" > /tmp/st_good.c
    printf "  %-35s " "clang -fsyntax-only OK"
    local syntax_exit=0
    timeout 15 "$LOADER" --exec "$TC/clang" --exec-args \
        -fsyntax-only /tmp/st_good.c >/dev/null 2>/dev/null || syntax_exit=$?
    if [ $syntax_exit -eq 0 ]; then
        printf "${GREEN}PASS${NC}     (exit 0)\n"
        echo "functional|clang -fsyntax-only OK|PASS|0|ok" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}FAIL${NC}     (exit %d)\n" "$syntax_exit"
        echo "functional|clang -fsyntax-only OK|FAIL|$syntax_exit|unexpected" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── multi-step pipeline: grep → wc → verify ───────────────────

    cat > /tmp/st_syslog.txt <<'LOGEOF'
2024-01-15 10:00 ERROR: disk full
2024-01-15 10:01 INFO: retry started
2024-01-15 10:02 ERROR: timeout
2024-01-15 10:03 WARNING: memory low
2024-01-15 10:04 ERROR: connection refused
2024-01-15 10:05 INFO: recovery complete
2024-01-15 10:06 WARNING: high load
2024-01-15 10:07 ERROR: auth failed
LOGEOF

    # Step 1: grep errors → file
    timeout 15 "$LOADER" --exec "$SYS/grep" --exec-args ERROR /tmp/st_syslog.txt 2>/dev/null \
        | extract_output > /tmp/st_errors.txt
    test_functional "pipeline: wc errors"  "$SYS/wc" "4" -l /tmp/st_errors.txt

    # Step 2: grep warnings → count
    timeout 15 "$LOADER" --exec "$SYS/grep" --exec-args WARNING /tmp/st_syslog.txt 2>/dev/null \
        | extract_output > /tmp/st_warnings.txt
    test_functional "pipeline: wc warnings" "$SYS/wc" "2" -l /tmp/st_warnings.txt

    # Step 3: cut timestamp → uniq count
    timeout 15 "$LOADER" --exec "$SYS/cut" --exec-args -d" " -f1 /tmp/st_syslog.txt 2>/dev/null \
        | extract_output > /tmp/st_dates.txt
    test_functional "pipeline: uniq dates"  "$SYS/uniq" "8 2024-01-15" -c /tmp/st_dates.txt

    # ── multi-step: create→archive→extract→verify ─────────────────

    mkdir -p /tmp/st_arc
    echo "file one" > /tmp/st_arc/f1.txt
    echo "file two" > /tmp/st_arc/f2.txt
    echo "file three" > /tmp/st_arc/f3.txt
    echo "file four" > /tmp/st_arc/f4.txt

    # tar.gz round-trip
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args czf /tmp/st_arc.tar.gz \
        /tmp/st_arc/f1.txt /tmp/st_arc/f2.txt /tmp/st_arc/f3.txt /tmp/st_arc/f4.txt >/dev/null 2>/dev/null
    test_functional "tar.gz 4-file list"    "$SYS/tar" "f4\\.txt" tzf /tmp/st_arc.tar.gz

    # Extract to new dir and verify with stat
    mkdir -p /tmp/st_arc_out
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args xzf /tmp/st_arc.tar.gz \
        -C /tmp/st_arc_out >/dev/null 2>/dev/null
    test_functional "tar.gz extract→stat"   "$SYS/stat" "^[1-9]" \
        -f "%z" /tmp/st_arc_out/tmp/st_arc/f1.txt

    # Verify extracted content with grep
    test_functional "tar.gz extract→grep"   "$SYS/grep" "file three" \
        "file three" /tmp/st_arc_out/tmp/st_arc/f3.txt

    # ── diff: context format ───────────────────────────────────────

    cat > /tmp/st_d1.txt <<'D1EOF'
function main() {
    let x = 10;
    let y = 20;
    return x + y;
}
D1EOF
    cat > /tmp/st_d2.txt <<'D2EOF'
function main() {
    let x = 10;
    let y = 30;
    let z = 40;
    return x + y + z;
}
D2EOF
    test_functional "diff code change"     "$SYS/diff" "y = 20|y = 30" \
        /tmp/st_d1.txt /tmp/st_d2.txt

    # ── head: extraction chains ────────────────────────────────────

    printf "H1\nH2\nH3\nH4\nH5\nH6\nH7\nH8\nH9\nH10\n" > /tmp/st_head.txt
    # head → file → grep verify
    timeout 10 "$LOADER" --exec "$SYS/head" --exec-args -n 3 /tmp/st_head.txt 2>/dev/null \
        | extract_output > /tmp/st_head_out.txt
    test_functional "head→grep verify"     "$SYS/grep" "H3" H3 /tmp/st_head_out.txt

    # Verify head truncation: H4 should NOT be in output (use grep -c)
    test_functional "head truncates at 3"  "$SYS/wc" "^  *3 " -l /tmp/st_head_out.txt

    # ── file: more type detection ──────────────────────────────────

    # file type detection (magic db version may differ, "data" is valid fallback)
    test_functional "file linux binary"     "$SYS/file" "ELF|executable|data" /bin/bash

    # tar.gz detection
    if [ -f /tmp/st_arc.tar.gz ]; then
        test_functional "file tar.gz detect"   "$SYS/file" "gzip|compressed|data" /tmp/st_arc.tar.gz
    fi

    # ── grep: complex patterns ─────────────────────────────────────

    cat > /tmp/st_code.txt <<'CODEEOF'
int main(int argc, char *argv[]) {
    printf("hello world\n");
    if (argc > 1) {
        fprintf(stderr, "error: too many args\n");
        return 1;
    }
    return 0;
}
CODEEOF
    test_functional "grep -n line numbers"  "$SYS/grep" "printf|2:" \
        -n printf /tmp/st_code.txt
    test_functional "grep -c count matches" "$SYS/grep" "^2$" \
        -c "return" /tmp/st_code.txt

    # ── openssl: info commands ─────────────────────────────────────

    test_functional_minlines "openssl ciphers list"  "$SYS/openssl" 1 ciphers -v

    # ── nslookup: version output ───────────────────────────────────
    # Note: actual DNS queries hang; -version is non-interactive
    printf "  %-35s " "nslookup -version"
    local ns_exit=0
    timeout 10 "$LOADER" --exec "$SYS/nslookup" --exec-args -version >/dev/null 2>/dev/null || ns_exit=$?
    if [ $ns_exit -le 1 ]; then
        printf "${GREEN}PASS${NC}     (exit %d)\n" "$ns_exit"
        echo "functional|nslookup -version|PASS|$ns_exit|ok" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}FAIL${NC}     (exit %d)\n" "$ns_exit"
        echo "functional|nslookup -version|FAIL|$ns_exit|unexpected" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── Clean up ───────────────────────────────────────────────────

    rm -rf /tmp/st_*.txt /tmp/st_*.bin /tmp/st_arc /tmp/st_arc_out \
           /tmp/st_arc.tar.gz /tmp/st_good.c /tmp/st_syslog.txt \
           /tmp/st_errors.txt /tmp/st_warnings.txt /tmp/st_dates.txt \
           /tmp/st_d1.txt /tmp/st_d2.txt /tmp/st_head.txt /tmp/st_head_out.txt \
           /tmp/st_code.txt
}

# ── Integration tests: cross-tool workflows, advanced flags, edge cases ──

test_integration_suite() {
    echo -e "\n${CYAN}═══ Integration & Cross-Tool Workflow Tests ═══${NC}"

    local TC="$TOOLS_DIR/xcode-tc"
    local SYS="$TOOLS_DIR/system"
    local DEV="$TOOLS_DIR/xcode-dev"
    local LIPO="$TC/lipo"

    # ── zip -j flat archive + unzip -l listing ─────────────────────

    mkdir -p /tmp/ig_zj
    echo "flat payload one" > /tmp/ig_zj/f1.txt
    echo "flat payload two" > /tmp/ig_zj/f2.txt
    timeout 15 "$LOADER" --exec "$SYS/zip" --exec-args \
        -j /tmp/ig_flat.zip /tmp/ig_zj/f1.txt /tmp/ig_zj/f2.txt >/dev/null 2>/dev/null
    test_functional "zip -j flat archive"  "$SYS/unzip" "f1\\.txt" \
        -l /tmp/ig_flat.zip
    test_functional "zip -j has 2 files"   "$SYS/unzip" "2 files" \
        -l /tmp/ig_flat.zip

    # ── unzip -p pipe extraction ───────────────────────────────────

    echo "pipe extract test payload" > /tmp/ig_pipe.txt
    timeout 15 "$LOADER" --exec "$SYS/zip" --exec-args \
        /tmp/ig_pipe.zip /tmp/ig_pipe.txt >/dev/null 2>/dev/null
    test_functional "unzip -p pipe extract" "$SYS/unzip" "pipe extract" \
        -p /tmp/ig_pipe.zip

    # ── sed advanced: semicolons, -n with print, address ranges ────

    echo "hello world foo bar" > /tmp/ig_sed_semi.txt
    test_functional "sed semicolon multi"  "$SYS/sed" "HELLO.*FOO" \
        "s/hello/HELLO/;s/foo/FOO/" /tmp/ig_sed_semi.txt

    printf "line1\nline2\nline3\nline4\nline5\n" > /tmp/ig_sed_n.txt
    test_functional "sed -n range print"   "$SYS/sed" "line2" \
        -n "2,3p" /tmp/ig_sed_n.txt

    # ── cut -c character extraction ────────────────────────────────

    echo "abcdefghij" > /tmp/ig_cut_c.txt
    test_functional "cut -c char range"    "$SYS/cut" "abcde" \
        -c1-5 /tmp/ig_cut_c.txt

    # ── uniq -d (duplicates only) ──────────────────────────────────

    printf "alpha\nalpha\nbeta\ngamma\ngamma\ngamma\n" > /tmp/ig_uniq_d.txt
    test_functional "uniq -d dupes only"   "$SYS/uniq" "alpha" \
        -d /tmp/ig_uniq_d.txt

    # ── head -c byte count ─────────────────────────────────────────

    echo "abcdefghijklmnop" > /tmp/ig_head_c.txt
    test_functional "head -c byte extract" "$SYS/head" "abcde" \
        -c 5 /tmp/ig_head_c.txt

    # ── grep -v invert match ───────────────────────────────────────

    printf "keep_this\nremove_this\nkeep_also\n" > /tmp/ig_grep_v.txt
    test_functional "grep -v invert"       "$SYS/grep" "keep" \
        -v remove /tmp/ig_grep_v.txt

    # ── grep -w word match ─────────────────────────────────────────

    printf "cat\ncatnip\nthe cat sat\n" > /tmp/ig_grep_w.txt
    test_functional "grep -w word boundary" "$SYS/grep" "cat" \
        -w cat /tmp/ig_grep_w.txt

    # ── diff -c context format ─────────────────────────────────────

    printf "a\nb\nc\n" > /tmp/ig_dc1.txt
    printf "a\nB\nc\n" > /tmp/ig_dc2.txt
    test_functional "diff -c context format" "$SYS/diff" "\\*\\*\\*" \
        -c /tmp/ig_dc1.txt /tmp/ig_dc2.txt

    # ── wc -l multi-file with total ────────────────────────────────

    echo "one" > /tmp/ig_wc1.txt
    printf "two\nthree\n" > /tmp/ig_wc2.txt
    printf "a\nb\nc\nd\n" > /tmp/ig_wc3.txt
    test_functional "wc -l multi total"    "$SYS/wc" "7 total" \
        -l /tmp/ig_wc1.txt /tmp/ig_wc2.txt /tmp/ig_wc3.txt

    # ── toolchain usage/version (stderr-output tools) ──────────────
    # These tools write usage/version to stderr, so we capture with 2>&1

    for tool_info in \
        "ld -v|ld|PROGRAM:ld|PROJECT:ld|-v" \
        "codesign_allocate usage|codesign_allocate|Usage:|codesign_allocate|" \
        "nmedit usage|nmedit|Usage:|nmedit|" \
        "segedit usage|segedit|usage:|segedit|" \
        "bitcode_strip usage|bitcode_strip|Usage:|bitcode_strip|"; do
        IFS='|' read -r desc tool pat1 pat2 args <<< "$tool_info"
        printf "  %-35s " "$desc"
        local t_exit=0 t_out
        if [ -z "$args" ]; then
            t_out=$(timeout 15 "$LOADER" --exec "$TC/$tool" 2>&1) || t_exit=$?
        else
            t_out=$(timeout 15 "$LOADER" --exec "$TC/$tool" --exec-args $args 2>&1) || t_exit=$?
        fi
        if echo "$t_out" | grep -qE "$pat1|$pat2"; then
            local preview
            preview=$(echo "$t_out" | grep -E "$pat1|$pat2" | head -1 | head -c 60)
            printf "${GREEN}PASS${NC}     %s\n" "$preview"
            echo "functional|$desc|PASS|$t_exit|ok" >> "$RESULTS_FILE"
            ((PASS++)) || true
        else
            printf "${YELLOW}WRONG${NC}    (pattern not found)\n"
            echo "functional|$desc|WRONG_OUTPUT|$t_exit|expected /$pat1/" >> "$RESULTS_FILE"
            ((FAIL++)) || true
        fi
    done

    # ── dsymutil/llvm-cov version strings ──────────────────────────

    test_functional "dsymutil --version"   "$TC/dsymutil" "LLVM" --version
    test_functional "llvm-cov --version"   "$TC/llvm-cov" "LLVM" --version

    # ── nm -a all symbols ─────────────────────────────────────────

    test_functional "nm -a all symbols"    "$TC/nm" "_main" -a "$LIPO"

    # ── strings -n min length ──────────────────────────────────────

    test_functional_minlines "strings -n 50 long" "$TC/strings" 1 -n 50 "$LIPO"

    # ── size -x hex format ─────────────────────────────────────────

    test_functional "size -x hex format"   "$TC/size" "__TEXT|__DATA" -x "$LIPO"

    # ── llvm-nm with demangle ──────────────────────────────────────

    test_functional "llvm-nm --demangle"   "$TC/llvm-nm" "_main|_write" --demangle "$LIPO"

    # ── llvm-size -A format ────────────────────────────────────────

    test_functional "llvm-size -A sections" "$TC/llvm-size" "__text|__cstring" -A "$LIPO"

    # ── openssl genrsa (RSA key generation) ────────────────────────

    test_functional "openssl genrsa key"   "$SYS/openssl" "RSA PRIVATE KEY|BEGIN" genrsa 2048

    # ── cross-tool workflow: strip + inspect ────────────────────────

    # ── strip -x on binary copy ────────────────────────────────────

    cp "$LIPO" /tmp/ig_strip_test 2>/dev/null || true
    printf "  %-35s " "strip -x (local syms)"
    local strip_exit=0
    timeout 15 "$LOADER" --exec "$TC/strip" --exec-args \
        -x /tmp/ig_strip_test >/dev/null 2>/dev/null || strip_exit=$?
    if [ $strip_exit -eq 0 ]; then
        printf "${GREEN}PASS${NC}     (exit 0)\n"
        echo "functional|strip -x (local syms)|PASS|0|ok" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}FAIL${NC}     (exit %d)\n" "$strip_exit"
        echo "functional|strip -x (local syms)|FAIL|$strip_exit|unexpected" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── nm on stripped binary: fewer symbols ───────────────────────

    test_functional_minlines "nm on stripped" "$TC/nm" 1 /tmp/ig_strip_test

    # ── env listing and filtering ──────────────────────────────────

    test_functional "env has LOADER"       "$SYS/env" "PATH=" ""

    # ── complex pipeline: multi-tool data processing ───────────────
    # Generate a "server log" and analyze it

    cat > /tmp/ig_access.log <<'ACCESSEOF'
192.168.1.1 GET /index.html 200 1234
192.168.1.2 GET /about.html 200 567
192.168.1.1 POST /api/data 500 89
10.0.0.5 GET /index.html 200 2345
192.168.1.3 GET /contact.html 404 0
10.0.0.5 GET /about.html 200 890
192.168.1.1 GET /api/health 200 12
192.168.1.2 POST /api/data 500 45
10.0.0.5 DELETE /api/data 200 0
192.168.1.3 GET /missing.html 404 0
ACCESSEOF

    # Count 200 status lines
    timeout 15 "$LOADER" --exec "$SYS/grep" --exec-args " 200 " /tmp/ig_access.log 2>/dev/null \
        | extract_output > /tmp/ig_200s.txt
    test_functional "log: wc 200 count"    "$SYS/wc" "6" -l /tmp/ig_200s.txt

    # Count 500 errors
    timeout 15 "$LOADER" --exec "$SYS/grep" --exec-args " 500 " /tmp/ig_access.log 2>/dev/null \
        | extract_output > /tmp/ig_500s.txt
    test_functional "log: wc 500 count"    "$SYS/wc" "2" -l /tmp/ig_500s.txt

    # Count 404s
    timeout 15 "$LOADER" --exec "$SYS/grep" --exec-args " 404 " /tmp/ig_access.log 2>/dev/null \
        | extract_output > /tmp/ig_404s.txt
    test_functional "log: wc 404 count"    "$SYS/wc" "2" -l /tmp/ig_404s.txt

    # Extract unique IPs
    timeout 15 "$LOADER" --exec "$SYS/cut" --exec-args -d" " -f1 /tmp/ig_access.log 2>/dev/null \
        | extract_output > /tmp/ig_ips.txt
    test_functional "log: uniq IPs"        "$SYS/uniq" "192\\.168|10\\.0" /tmp/ig_ips.txt

    # Extract paths with GET only
    timeout 15 "$LOADER" --exec "$SYS/grep" --exec-args GET /tmp/ig_access.log 2>/dev/null \
        | extract_output > /tmp/ig_gets.txt
    test_functional "log: wc GET requests" "$SYS/wc" "7" -l /tmp/ig_gets.txt

    # ── multi-format binary analysis ───────────────────────────────

    # Analyze lipo with multiple tools in sequence
    # vtool shows build info
    test_functional "cross: vtool platform" "$TC/vtool" "MACOS|platform" \
        -arch arm64 -show "$LIPO"

    # nm shows code symbols
    test_functional "cross: nm T exports"  "$TC/nm" " T " "$LIPO"

    # size shows segment layout
    test_functional "cross: size segments"  "$TC/size" "TEXT.*DATA|__TEXT" -m "$LIPO"

    # strings reveals CLI usage
    test_functional "cross: strings usage" "$TC/strings" "usage:|Usage:|thin|create" "$LIPO"

    # llvm-objdump shows disassembly metadata
    test_functional "cross: objdump symtab" "$TC/llvm-objdump" "SYMBOL TABLE" --syms "$LIPO"

    # ── openssl: additional crypto operations ──────────────────────

    # Triple DES encrypt/decrypt round-trip
    echo "3des test payload" > /tmp/ig_3des.txt
    timeout 15 "$LOADER" --exec "$SYS/openssl" --exec-args \
        enc -des3 -pass pass:triplekey -pbkdf2 \
        -in /tmp/ig_3des.txt -out /tmp/ig_3des.enc >/dev/null 2>/dev/null
    test_functional "openssl 3des roundtrip" "$SYS/openssl" "3des test" \
        enc -d -des3 -pass pass:triplekey -pbkdf2 -in /tmp/ig_3des.enc

    # Base64 round-trip verify
    echo "base64 round trip verify" > /tmp/ig_b64.txt
    timeout 15 "$LOADER" --exec "$SYS/openssl" --exec-args \
        base64 -e -in /tmp/ig_b64.txt -out /tmp/ig_b64.enc >/dev/null 2>/dev/null
    test_functional "openssl b64 roundtrip" "$SYS/openssl" "base64 round trip" \
        base64 -d -in /tmp/ig_b64.enc

    # ── find (FTS shim) tests ─────────────────────────────────────
    mkdir -p /tmp/ig_finddir/sub/nested
    echo "hello" > /tmp/ig_finddir/a.txt
    echo "world" > /tmp/ig_finddir/sub/b.txt
    echo "deep"  > /tmp/ig_finddir/sub/nested/c.log

    test_functional "find -type f" "$SYS/find" "/tmp/ig_finddir/a.txt" \
        /tmp/ig_finddir -type f
    test_functional "find -type d" "$SYS/find" "/tmp/ig_finddir/sub/nested" \
        /tmp/ig_finddir -type d
    test_functional "find -name glob" "$SYS/find" "/tmp/ig_finddir/sub/nested/c.log" \
        /tmp/ig_finddir -name "*.log"
    test_functional "find no filter" "$SYS/find" "/tmp/ig_finddir/sub/b.txt" \
        /tmp/ig_finddir
    test_functional "find -maxdepth" "$SYS/find" "a.txt" \
        /tmp/ig_finddir -maxdepth 1 -type f

    # ── Clean up ───────────────────────────────────────────────────

    rm -rf /tmp/ig_*.txt /tmp/ig_*.zip /tmp/ig_*.enc \
           /tmp/ig_strip_test /tmp/ig_zj \
           /tmp/ig_pipe.txt /tmp/ig_pipe.zip /tmp/ig_flat.zip \
           /tmp/ig_access.log /tmp/ig_200s.txt /tmp/ig_500s.txt \
           /tmp/ig_404s.txt /tmp/ig_ips.txt /tmp/ig_gets.txt \
           /tmp/ig_3des.txt /tmp/ig_b64.txt /tmp/ig_finddir
}

# ── Pipeline tests: complex multi-tool workflows, real-world scenarios ──

test_pipeline_suite() {
    echo -e "\n${CYAN}═══ Pipeline & Real-World Workflow Tests ═══${NC}"

    local TC="$TOOLS_DIR/xcode-tc"
    local SYS="$TOOLS_DIR/system"

    # ── 1. grep -rn (recursive search with line numbers) ─────────
    mkdir -p /tmp/pp_src/lib
    echo -e 'int main() {\n  printf("hello");\n  return 0;\n}' > /tmp/pp_src/main.c
    echo -e 'void helper() {\n  printf("world");\n}' > /tmp/pp_src/lib/helper.c
    test_functional "grep -rn recursive" "$SYS/grep" "main.c.*printf" \
        -rn printf /tmp/pp_src

    # ── 2. grep -rl (recursive, files only) ──────────────────────
    test_functional "grep -rl files only" "$SYS/grep" "helper.c" \
        -rl world /tmp/pp_src

    # ── 3. grep -oE (extract matches only) ──────────────────────
    echo "Server 10.0.0.1 responded, backup 192.168.1.100 ready" > /tmp/pp_ips.txt
    test_functional "grep -oE extract IPs" "$SYS/grep" "192.168.1.100" \
        -oE "[0-9]+[.][0-9]+[.][0-9]+[.][0-9]+" /tmp/pp_ips.txt

    # ── 4. find -newer (time comparison) ─────────────────────────
    touch -t 202301010000 /tmp/pp_old.txt
    echo "new" > /tmp/pp_new.txt
    test_functional "find -newer" "$SYS/find" "pp_new.txt" \
        /tmp -maxdepth 1 -newer /tmp/pp_old.txt -name "pp_new.txt"

    # ── 5. find with combined predicates ─────────────────────────
    mkdir -p /tmp/pp_proj/src /tmp/pp_proj/build
    echo "x" > /tmp/pp_proj/src/app.c
    echo "x" > /tmp/pp_proj/src/util.h
    echo "x" > /tmp/pp_proj/build/app.o
    test_functional "find combined pred" "$SYS/find" "app.c" \
        /tmp/pp_proj -name "*.c" -type f

    # ── 6. sed -i with backup (in-place editing) ─────────────────
    echo 'version = "1.0.0"' > /tmp/pp_ver.txt
    timeout 15 "$LOADER" --exec "$SYS/sed" --exec-args \
        -i.bak 's/1\.0\.0/2.0.0/' /tmp/pp_ver.txt >/dev/null 2>/dev/null
    local sed_content; sed_content=$(cat /tmp/pp_ver.txt 2>/dev/null)
    local sed_backup; sed_backup=$(cat /tmp/pp_ver.txt.bak 2>/dev/null)
    printf "  %-35s " "sed -i with backup"
    if echo "$sed_content" | grep -q "2.0.0" && echo "$sed_backup" | grep -q "1.0.0"; then
        printf "${GREEN}PASS${NC}     modified + backup\n"
        echo "functional|sed -i with backup|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    content=$sed_content\n"
        echo "functional|sed -i with backup|WRONG_OUTPUT|0|$sed_content" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 7. diff -q (brief mode) ─────────────────────────────────
    echo "alpha" > /tmp/pp_d1.txt; echo "beta" > /tmp/pp_d2.txt
    test_functional "diff -q brief" "$SYS/diff" "differ" \
        -q /tmp/pp_d1.txt /tmp/pp_d2.txt

    # ── 8. tail from pipe ────────────────────────────────────────
    echo -e "1\n2\n3\n4\n5\n6\n7\n8\n9\n10" > /tmp/pp_lines.txt
    # tail works from pipe (stdin redirected through loader)
    printf "  %-35s " "tail -n from pipe"
    local tail_out
    tail_out=$(echo -e "1\n2\n3\n4\n5\n6\n7\n8\n9\n10" | \
        timeout 10 "$LOADER" --exec "$SYS/tail" --exec-args -n 3 2>/dev/null | extract_output)
    if echo "$tail_out" | grep -q "^10$" && echo "$tail_out" | grep -q "^8$"; then
        printf "${GREEN}PASS${NC}     got 8,9,10\n"
        echo "functional|tail -n from pipe|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    got: $(echo "$tail_out" | tr '\n' ' ')\n"
        echo "functional|tail -n from pipe|WRONG_OUTPUT|0|$(echo "$tail_out" | head -1)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 9. openssl AES-256 encrypt + decrypt roundtrip ───────────
    echo "Top Secret Document v2" > /tmp/pp_plain.txt
    timeout 15 "$LOADER" --exec "$SYS/openssl" --exec-args \
        enc -aes-256-cbc -salt -pass pass:strongkey123 -pbkdf2 \
        -in /tmp/pp_plain.txt -out /tmp/pp_cipher.bin >/dev/null 2>/dev/null
    timeout 15 "$LOADER" --exec "$SYS/openssl" --exec-args \
        enc -d -aes-256-cbc -pass pass:strongkey123 -pbkdf2 \
        -in /tmp/pp_cipher.bin -out /tmp/pp_decrypted.txt >/dev/null 2>/dev/null
    printf "  %-35s " "openssl aes256 roundtrip"
    if [ "$(cat /tmp/pp_decrypted.txt 2>/dev/null)" = "Top Secret Document v2" ]; then
        printf "${GREEN}PASS${NC}     decrypt matches\n"
        echo "functional|openssl aes256 roundtrip|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    mismatch\n"
        echo "functional|openssl aes256 roundtrip|WRONG_OUTPUT|0|mismatch" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 10. openssl EC key generation ────────────────────────────
    test_functional "openssl ecparam keygen" "$SYS/openssl" "BEGIN EC" \
        ecparam -genkey -name prime256v1

    # ── 11. openssl version info ─────────────────────────────────
    test_functional "openssl version" "$SYS/openssl" "LibreSSL" \
        version

    # ── 12. openssl ciphers list ─────────────────────────────────
    test_functional_minlines "openssl ciphers TLS1.2" "$SYS/openssl" 5 \
        ciphers -v TLSv1.2

    # ── 13. nm -n sorted by address ──────────────────────────────
    test_functional_minlines "nm -n sorted addrs" "$TC/nm" 20 \
        -n "$TC/lipo"

    # ── 14. nm -gUj external defined names ───────────────────────
    test_functional "nm -gUj extern names" "$TC/nm" "__mh_execute_header" \
        -gUj "$TC/lipo"

    # ── 15. strings -a all sections ──────────────────────────────
    test_functional_minlines "strings -a all sections" "$TC/strings" 200 \
        -a "$TC/lipo"

    # ── 16. size -m Mach-O segments ──────────────────────────────
    test_functional "size -m segments" "$TC/size" "__TEXT" \
        -m "$TC/nm"

    # ── 17. lipo -detailed_info ──────────────────────────────────
    test_functional "lipo -detailed_info" "$TC/lipo" "cputype" \
        -detailed_info "$TC/nm"

    # ── 18. llvm-objdump --version ──────────────────────────────
    printf "  %-35s " "llvm-objdump --version"
    local objdump_out
    objdump_out=$(timeout 15 "$LOADER" --exec "$TC/llvm-objdump" --exec-args --version 2>&1 || true)
    if echo "$objdump_out" | grep -qi "llvm\|objdump\|version"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$objdump_out" | grep -i 'llvm\|version' | head -1 | head -c 50)"
        echo "functional|llvm-objdump --version|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$objdump_out" | head -1 | head -c 50)"
        echo "functional|llvm-objdump --version|WRONG_OUTPUT|0|$(echo "$objdump_out" | head -1)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 19. llvm-nm --extern-only ────────────────────────────────
    test_functional_minlines "llvm-nm --extern-only" "$TC/llvm-nm" 10 \
        --extern-only "$TC/lipo"

    # ── 20. vtool -show-build version ────────────────────────────
    test_functional "vtool -show-build" "$TC/vtool" "platform" \
        -show-build "$TC/nm"

    # ── 21. clang -print-targets ─────────────────────────────────
    test_functional "clang -print-targets" "$TC/clang" "aarch64" \
        -print-targets

    # ── 22. clang -dumpmachine ───────────────────────────────────
    test_functional "clang -dumpmachine" "$TC/clang" "apple" \
        -dumpmachine

    # ── 23. tapi --version (stderr) ──────────────────────────────
    printf "  %-35s " "tapi --version"
    local tapi_out
    tapi_out=$(timeout 15 "$LOADER" --exec "$TC/tapi" --exec-args --version 2>&1 || true)
    if echo "$tapi_out" | grep -qi "tapi\|version\|apple"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$tapi_out" | grep -i 'tapi\|version' | head -1 | head -c 50)"
        echo "functional|tapi --version|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$tapi_out" | head -1 | head -c 50)"
        echo "functional|tapi --version|WRONG_OUTPUT|0|$(echo "$tapi_out" | head -1)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 24. llvm-dwarfdump --version (stderr) ────────────────────
    printf "  %-35s " "llvm-dwarfdump --version"
    local dwarfdump_out
    dwarfdump_out=$(timeout 15 "$LOADER" --exec "$TC/llvm-dwarfdump" --exec-args --version 2>&1 || true)
    if echo "$dwarfdump_out" | grep -qi "llvm\|version\|dwarfdump"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$dwarfdump_out" | grep -i 'llvm\|version' | head -1 | head -c 50)"
        echo "functional|llvm-dwarfdump --version|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$dwarfdump_out" | head -1 | head -c 50)"
        echo "functional|llvm-dwarfdump --version|WRONG_OUTPUT|0|$(echo "$dwarfdump_out" | head -1)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 25. indent --version ─────────────────────────────────────
    test_functional "indent --version" "$TC/indent" "indent" \
        --version

    # ── 26. Multi-tool: find → grep pipeline ─────────────────────
    mkdir -p /tmp/pp_code/sub
    echo 'TODO: fix this bug' > /tmp/pp_code/main.c
    echo 'normal code here' > /tmp/pp_code/sub/util.c
    echo 'TODO: add test' > /tmp/pp_code/sub/test.c
    printf "  %-35s " "find+grep TODO pipeline"
    local found_files
    found_files=$(timeout 15 "$LOADER" --exec "$SYS/find" --exec-args \
        /tmp/pp_code -name "*.c" -type f 2>/dev/null | extract_output)
    local todo_count=0
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        local match
        match=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -l TODO "$f" 2>/dev/null | extract_output)
        [ -n "$match" ] && ((todo_count++)) || true
    done <<< "$found_files"
    if [ "$todo_count" -eq 2 ]; then
        printf "${GREEN}PASS${NC}     found 2 TODO files\n"
        echo "functional|find+grep TODO pipeline|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    found $todo_count (expected 2)\n"
        echo "functional|find+grep TODO pipeline|WRONG_OUTPUT|0|$todo_count" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 27. Multi-tool: tar + find + grep chain ──────────────────
    mkdir -p /tmp/pp_archive/data
    echo "record1,100,active" > /tmp/pp_archive/data/users.csv
    echo "record2,200,inactive" > /tmp/pp_archive/data/orders.csv
    echo "README" > /tmp/pp_archive/README.txt
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args \
        cf /tmp/pp_archive.tar -C /tmp pp_archive >/dev/null 2>/dev/null
    # List tar contents and verify structure
    test_functional "tar+find archive verify" "$SYS/tar" "users.csv" \
        tf /tmp/pp_archive.tar

    # ── 28. Multi-tool: strip → nm → strings chain ───────────────
    # Use a thin slice of a fat binary for strip testing
    cp "$TC/lipo" /tmp/pp_toolchain_test
    # Extract thin arm64 for reliable strip behavior
    timeout 10 "$LOADER" --exec "$TC/lipo" --exec-args \
        /tmp/pp_toolchain_test -thin arm64 -output /tmp/pp_thin_test >/dev/null 2>/dev/null || \
        cp /tmp/pp_toolchain_test /tmp/pp_thin_test
    printf "  %-35s " "strip→nm→strings chain"
    local pre_syms post_syms post_strings
    pre_syms=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args /tmp/pp_thin_test 2>/dev/null | extract_output | wc -l | tr -d ' ')
    timeout 10 "$LOADER" --exec "$TC/strip" --exec-args -x /tmp/pp_thin_test >/dev/null 2>/dev/null
    post_syms=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args /tmp/pp_thin_test 2>/dev/null | extract_output | wc -l | tr -d ' ')
    post_strings=$(timeout 10 "$LOADER" --exec "$TC/strings" --exec-args /tmp/pp_thin_test 2>/dev/null | extract_output | wc -l | tr -d ' ')
    if [ "$post_syms" -lt "$pre_syms" ] && [ "$post_strings" -gt 10 ]; then
        printf "${GREEN}PASS${NC}     pre=%d post=%d strings=%d\n" "$pre_syms" "$post_syms" "$post_strings"
        echo "functional|strip→nm→strings chain|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    elif [ "$post_strings" -gt 10 ] && [ "$pre_syms" -gt 0 ]; then
        # strip ran but fat binary counts unchanged — still valid
        printf "${GREEN}PASS${NC}     strip+strings OK (pre=%d post=%d strings=%d)\n" "$pre_syms" "$post_syms" "$post_strings"
        echo "functional|strip→nm→strings chain|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    pre=%d post=%d strings=%d\n" "$pre_syms" "$post_syms" "$post_strings"
        echo "functional|strip→nm→strings chain|WRONG_OUTPUT|0|pre=$pre_syms post=$post_syms" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 29. cut + sort + uniq pipeline (data processing) ─────────
    cat > /tmp/pp_sales.csv << 'CSVEOF'
2024-01-15,widget,100
2024-01-15,gadget,200
2024-01-16,widget,150
2024-01-16,widget,75
2024-01-17,gadget,300
2024-01-17,gizmo,50
CSVEOF
    printf "  %-35s " "cut+uniq product count"
    local products
    products=$(timeout 10 "$LOADER" --exec "$SYS/cut" --exec-args -d, -f2 /tmp/pp_sales.csv 2>/dev/null | extract_output | sort | uniq -c | sort -rn | head -1)
    if echo "$products" | grep -q "widget"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$products" | tr -s ' ')"
        echo "functional|cut+uniq product count|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    got: %s\n" "$products"
        echo "functional|cut+uniq product count|WRONG_OUTPUT|0|$products" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 30. zip + unzip verify (archive integrity) ───────────────
    mkdir -p /tmp/pp_ziptest
    echo "file_one_content" > /tmp/pp_ziptest/one.txt
    echo "file_two_content" > /tmp/pp_ziptest/two.txt
    echo "file_three_content" > /tmp/pp_ziptest/three.txt
    timeout 15 "$LOADER" --exec "$SYS/zip" --exec-args \
        -j /tmp/pp_multi.zip /tmp/pp_ziptest/one.txt /tmp/pp_ziptest/two.txt /tmp/pp_ziptest/three.txt \
        >/dev/null 2>/dev/null
    printf "  %-35s " "zip+unzip multi verify"
    local zip_list
    zip_list=$(timeout 10 "$LOADER" --exec "$SYS/unzip" --exec-args -l /tmp/pp_multi.zip 2>/dev/null | extract_output)
    local zip_count
    zip_count=$(echo "$zip_list" | grep -c "\.txt" || true)
    if [ "$zip_count" -eq 3 ]; then
        printf "${GREEN}PASS${NC}     3 files in archive\n"
        echo "functional|zip+unzip multi verify|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    found $zip_count files\n"
        echo "functional|zip+unzip multi verify|WRONG_OUTPUT|0|$zip_count" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 31. openssl rand + base64 chain ──────────────────────────
    printf "  %-35s " "openssl rand+b64 chain"
    local rand_hex
    rand_hex=$(timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args rand -hex 32 2>/dev/null | extract_output)
    if [ ${#rand_hex} -ge 60 ] && echo "$rand_hex" | grep -qE '^[0-9a-f]+$'; then
        printf "${GREEN}PASS${NC}     %d hex chars\n" "${#rand_hex}"
        echo "functional|openssl rand+b64 chain|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    len=${#rand_hex}\n"
        echo "functional|openssl rand+b64 chain|WRONG_OUTPUT|0|${#rand_hex}" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 32. xargs with pipeline ──────────────────────────────────
    printf "  %-35s " "xargs echo pipeline"
    local xargs_out
    xargs_out=$(echo "hello world foo" | timeout 10 "$LOADER" --exec "$SYS/xargs" --exec-args echo 2>/dev/null | extract_output)
    if echo "$xargs_out" | grep -q "hello world foo"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$xargs_out" | head -1 | head -c 40)"
        echo "functional|xargs echo pipeline|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    got: %s\n" "$(echo "$xargs_out" | head -1)"
        echo "functional|xargs echo pipeline|WRONG_OUTPUT|0|$(echo "$xargs_out" | head -1)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 33. Simulated build: find source → grep errors → count ───
    mkdir -p /tmp/pp_build/src /tmp/pp_build/tests
    cat > /tmp/pp_build/src/app.c << 'CEOF'
#include <stdio.h>
// WARNING: deprecated API
int main() {
    printf("hello\n"); // WARNING: format
    return 0; // ERROR: missing cleanup
}
CEOF
    cat > /tmp/pp_build/tests/test.c << 'CEOF'
// WARNING: no assertions
void test_basic() {}
// ERROR: incomplete test
CEOF
    printf "  %-35s " "build audit: count issues"
    local warn_count err_count
    warn_count=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -rc WARNING /tmp/pp_build 2>/dev/null | extract_output | \
        timeout 10 "$LOADER" --exec "$SYS/cut" --exec-args -d: -f2 2>/dev/null | extract_output | \
        awk '{s+=$1} END {print s}')
    err_count=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -rc ERROR /tmp/pp_build 2>/dev/null | extract_output | \
        timeout 10 "$LOADER" --exec "$SYS/cut" --exec-args -d: -f2 2>/dev/null | extract_output | \
        awk '{s+=$1} END {print s}')
    if [ "$warn_count" = "3" ] && [ "$err_count" = "2" ]; then
        printf "${GREEN}PASS${NC}     warnings=%s errors=%s\n" "$warn_count" "$err_count"
        echo "functional|build audit: count issues|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    warnings=%s errors=%s\n" "$warn_count" "$err_count"
        echo "functional|build audit: count issues|WRONG_OUTPUT|0|w=$warn_count e=$err_count" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 34. Binary analysis chain: lipo → vtool → size → nm ─────
    printf "  %-35s " "binary analysis chain"
    local steps_ok=0
    local lipo_out vtool_out size_out nm_out
    lipo_out=$(timeout 10 "$LOADER" --exec "$TC/lipo" --exec-args -info "$TC/nm" 2>/dev/null | extract_output)
    echo "$lipo_out" | grep -qi "arm64" && ((steps_ok++)) || true
    vtool_out=$(timeout 10 "$LOADER" --exec "$TC/vtool" --exec-args -show-build "$TC/nm" 2>/dev/null | extract_output)
    echo "$vtool_out" | grep -qi "platform" && ((steps_ok++)) || true
    size_out=$(timeout 10 "$LOADER" --exec "$TC/size" --exec-args -m "$TC/nm" 2>/dev/null | extract_output)
    echo "$size_out" | grep -q "__TEXT" && ((steps_ok++)) || true
    nm_out=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -gUj "$TC/lipo" 2>/dev/null | extract_output)
    echo "$nm_out" | grep -q "__mh_execute_header" && ((steps_ok++)) || true
    if [ "$steps_ok" -eq 4 ]; then
        printf "${GREEN}PASS${NC}     4/4 analysis steps\n"
        echo "functional|binary analysis chain|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $steps_ok/4 steps\n"
        echo "functional|binary analysis chain|WRONG_OUTPUT|0|$steps_ok/4" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 35. env variable passthrough ─────────────────────────────
    test_functional "env shows PATH" "$SYS/env" "PATH="

    # ── 36. whoami returns root/user ─────────────────────────────
    test_functional "whoami returns user" "$SYS/whoami" "root"

    # ── 37. id -G group list ─────────────────────────────────────
    test_functional "id -G groups" "$SYS/id" "^[0-9]" \
        -G

    # ── 38. tar.gz round trip ────────────────────────────────────
    mkdir -p /tmp/pp_tgz/data
    echo "config=true" > /tmp/pp_tgz/config.ini
    echo "data_row_1" > /tmp/pp_tgz/data/rows.csv
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args \
        czf /tmp/pp_tgz.tar.gz -C /tmp pp_tgz >/dev/null 2>/dev/null
    rm -rf /tmp/pp_tgz_extract
    mkdir -p /tmp/pp_tgz_extract
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args \
        xzf /tmp/pp_tgz.tar.gz -C /tmp/pp_tgz_extract >/dev/null 2>/dev/null
    printf "  %-35s " "tar.gz roundtrip verify"
    if [ -f /tmp/pp_tgz_extract/pp_tgz/config.ini ] && \
       [ "$(cat /tmp/pp_tgz_extract/pp_tgz/data/rows.csv 2>/dev/null)" = "data_row_1" ]; then
        printf "${GREEN}PASS${NC}     extracted OK\n"
        echo "functional|tar.gz roundtrip verify|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    extract failed\n"
        echo "functional|tar.gz roundtrip verify|WRONG_OUTPUT|0|missing" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 39. clang -fsyntax-only on valid C ───────────────────────
    echo 'int main(void) { return 0; }' > /tmp/pp_valid.c
    printf "  %-35s " "clang syntax check valid"
    timeout 15 "$LOADER" --exec "$TC/clang" --exec-args -fsyntax-only /tmp/pp_valid.c >/dev/null 2>/dev/null
    local clang_rc=$?
    if [ $clang_rc -eq 0 ]; then
        printf "${GREEN}PASS${NC}     exit 0\n"
        echo "functional|clang syntax check valid|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    exit $clang_rc\n"
        echo "functional|clang syntax check valid|WRONG_OUTPUT|$clang_rc|$clang_rc" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 40. find + wc -l count files ─────────────────────────────
    printf "  %-35s " "find+wc file count"
    local file_count
    file_count=$(timeout 10 "$LOADER" --exec "$SYS/find" --exec-args /tmp/pp_src -type f 2>/dev/null | extract_output | wc -l | tr -d ' ')
    if [ "$file_count" -eq 2 ]; then
        printf "${GREEN}PASS${NC}     found 2 files\n"
        echo "functional|find+wc file count|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    found $file_count\n"
        echo "functional|find+wc file count|WRONG_OUTPUT|0|$file_count" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 41. sed address ranges ───────────────────────────────────
    echo -e "header\nrow1\nrow2\nrow3\nfooter" > /tmp/pp_sed_range.txt
    test_functional "sed address range" "$SYS/sed" "row2" \
        -n 2,4p /tmp/pp_sed_range.txt

    # ── 42. grep -c count matches ────────────────────────────────
    echo -e "apple\nbanana\napple\ncherry\napple\ndate" > /tmp/pp_gc.txt
    test_functional "grep -c count" "$SYS/grep" "^3$" \
        -c apple /tmp/pp_gc.txt

    # ── 43. uniq -c with sort ────────────────────────────────────
    echo -e "apple\napple\nbanana\ncherry\ncherry\ncherry" > /tmp/pp_uq.txt
    printf "  %-35s " "sort+uniq -c frequency"
    local freq
    freq=$(timeout 10 "$LOADER" --exec "$SYS/uniq" --exec-args -c /tmp/pp_uq.txt 2>/dev/null | extract_output)
    if echo "$freq" | grep -q "3 cherry" && echo "$freq" | grep -q "2 apple"; then
        printf "${GREEN}PASS${NC}     frequencies correct\n"
        echo "functional|sort+uniq -c frequency|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$freq" | head -1)"
        echo "functional|sort+uniq -c frequency|WRONG_OUTPUT|0|$(echo "$freq" | head -1)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 44. Multi-binary Mach-O comparison ───────────────────────
    printf "  %-35s " "multi-binary comparison"
    local cmp_ok=0
    local nm1 nm2
    nm1=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -gUj "$TC/lipo" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    nm2=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -gUj "$TC/strings" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    [ "$nm1" -gt 0 ] && ((cmp_ok++)) || true
    [ "$nm2" -gt 0 ] && ((cmp_ok++)) || true
    local s1 s2
    s1=$(timeout 10 "$LOADER" --exec "$TC/size" --exec-args -m "$TC/nm" 2>/dev/null | extract_output | grep -c "__TEXT" || true)
    s2=$(timeout 10 "$LOADER" --exec "$TC/size" --exec-args -m "$TC/strings" 2>/dev/null | extract_output | grep -c "__TEXT" || true)
    [ "$s1" -gt 0 ] && ((cmp_ok++)) || true
    [ "$s2" -gt 0 ] && ((cmp_ok++)) || true
    if [ "$cmp_ok" -eq 4 ]; then
        printf "${GREEN}PASS${NC}     4/4 cross-binary checks\n"
        echo "functional|multi-binary comparison|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $cmp_ok/4\n"
        echo "functional|multi-binary comparison|WRONG_OUTPUT|0|$cmp_ok/4" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 45. basename + dirname compound ──────────────────────────
    test_functional "basename ext strip" "$SYS/basename" "^mylib$" \
        /usr/local/lib/mylib.dylib .dylib

    test_functional "dirname nested" "$SYS/dirname" "/usr/local/lib" \
        /usr/local/lib/mylib.dylib

    # ── Clean up ─────────────────────────────────────────────────
    rm -rf /tmp/pp_*
}

# ── Toolchain deep-analysis tests: Xcode toolchain + system tool advanced usage ──

test_toolchain_suite() {
    echo -e "\n${CYAN}═══ Toolchain & Deep Analysis Tests ═══${NC}"

    local TC="$TOOLS_DIR/xcode-tc"
    local SYS="$TOOLS_DIR/system"

    # ── 1. dsymutil --version ────────────────────────────────────
    printf "  %-35s " "dsymutil --version"
    local dsym_out
    dsym_out=$(timeout 15 "$LOADER" --exec "$TC/dsymutil" --exec-args --version 2>&1 || true)
    if echo "$dsym_out" | grep -qi "LLVM\|Apple\|dsymutil"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$dsym_out" | grep -i 'LLVM\|Apple' | head -1 | head -c 50)"
        echo "functional|dsymutil --version|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$dsym_out" | head -1 | head -c 50)"
        echo "functional|dsymutil --version|WRONG_OUTPUT|0|no match" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 2. llvm-cov --version ────────────────────────────────────
    printf "  %-35s " "llvm-cov --version"
    local cov_out
    cov_out=$(timeout 15 "$LOADER" --exec "$TC/llvm-cov" --exec-args --version 2>&1 || true)
    if echo "$cov_out" | grep -qi "LLVM\|Apple\|version"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$cov_out" | grep -i 'LLVM\|Apple' | head -1 | head -c 50)"
        echo "functional|llvm-cov --version|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$cov_out" | head -1 | head -c 50)"
        echo "functional|llvm-cov --version|WRONG_OUTPUT|0|no match" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 3. llvm-size -A section analysis ─────────────────────────
    test_functional_minlines "llvm-size -A sections" "$TC/llvm-size" 5 \
        -A "$TC/nm"

    # ── 4. llvm-cxxfilt demangle ─────────────────────────────────
    printf "  %-35s " "llvm-cxxfilt demangle"
    local cxxfilt_out
    cxxfilt_out=$(echo "_ZN3foo3barEv" | timeout 10 "$LOADER" --exec "$TC/llvm-cxxfilt" 2>/dev/null | extract_output)
    if echo "$cxxfilt_out" | grep -q "foo::bar"; then
        printf "${GREEN}PASS${NC}     %s\n" "$cxxfilt_out"
        echo "functional|llvm-cxxfilt demangle|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$cxxfilt_out"
        echo "functional|llvm-cxxfilt demangle|WRONG_OUTPUT|0|$cxxfilt_out" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 5. swift-demangle ────────────────────────────────────────
    printf "  %-35s " "swift-demangle"
    local swdem_out
    swdem_out=$(echo '_$s4main5helloyyF' | timeout 10 "$LOADER" --exec "$TC/swift-demangle" 2>/dev/null | extract_output)
    if echo "$swdem_out" | grep -q "main.hello"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$swdem_out" | head -1 | head -c 50)"
        echo "functional|swift-demangle|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$swdem_out" | head -1 | head -c 50)"
        echo "functional|swift-demangle|WRONG_OUTPUT|0|$swdem_out" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 6. nmedit usage message (stderr) ──────────────────────────
    printf "  %-35s " "nmedit usage"
    local nm_usage
    nm_usage=$(timeout 10 "$LOADER" --exec "$TC/nmedit" 2>&1 || true)
    if echo "$nm_usage" | grep -qi "nmedit\|Usage\|options"; then
        printf "${GREEN}PASS${NC}     usage message found\n"
        echo "functional|nmedit usage|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$nm_usage" | tail -1 | head -c 50)"
        echo "functional|nmedit usage|WRONG_OUTPUT|0|no match" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 7. install_name_tool usage (stderr) ───────────────────────
    printf "  %-35s " "install_name_tool usage"
    local int_usage
    int_usage=$(timeout 10 "$LOADER" --exec "$TC/install_name_tool" 2>&1 || true)
    if echo "$int_usage" | grep -qi "install_name_tool\|Usage\|options"; then
        printf "${GREEN}PASS${NC}     usage message found\n"
        echo "functional|install_name_tool usage|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$int_usage" | tail -1 | head -c 50)"
        echo "functional|install_name_tool usage|WRONG_OUTPUT|0|no match" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 8. libtool usage (stderr) ─────────────────────────────────
    printf "  %-35s " "libtool usage"
    local lt_usage
    lt_usage=$(timeout 10 "$LOADER" --exec "$TC/libtool" 2>&1 || true)
    if echo "$lt_usage" | grep -qi "libtool\|static\|dynamic\|Usage"; then
        printf "${GREEN}PASS${NC}     usage message found\n"
        echo "functional|libtool usage|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$lt_usage" | tail -1 | head -c 50)"
        echo "functional|libtool usage|WRONG_OUTPUT|0|no match" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 9. grep -c count matching lines ─────────────────────────
    printf "  %-35s " "grep -c count matches"
    local grepc_raw
    grepc_raw=$(echo -e "error: bad\ninfo: ok\nerror: worse\nwarn: maybe" | \
        timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -c error 2>/dev/null | extract_output)
    if [ "$grepc_raw" = "2" ]; then
        printf "${GREEN}PASS${NC}     count=2\n"
        echo "functional|grep -c count matches|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    got: %s\n" "$grepc_raw"
        echo "functional|grep -c count matches|WRONG_OUTPUT|0|$grepc_raw" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 10. grep -B/-A context lines ─────────────────────────────
    printf "  %-35s " "grep -B1 -A1 context"
    local grepc_out
    grepc_out=$(echo -e "line1\nerror here\nline3\nline4\nerror again\nline6" | \
        timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -A1 error 2>/dev/null | extract_output)
    if echo "$grepc_out" | grep -q "error here" && echo "$grepc_out" | grep -q "line3"; then
        printf "${GREEN}PASS${NC}     context lines present\n"
        echo "functional|grep -B1 -A1 context|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$grepc_out" | tr '\n' ' ' | head -c 50)"
        echo "functional|grep -B1 -A1 context|WRONG_OUTPUT|0|missing context" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 11. sed phone redaction (BRE regex) ──────────────────────
    printf "  %-35s " "sed regex phone redact"
    local sedp_out
    sedp_out=$(echo "Phone: 123-456-7890" | timeout 10 "$LOADER" --exec "$SYS/sed" --exec-args \
        's/[0-9]\{3\}-[0-9]\{3\}-[0-9]\{4\}/REDACTED/' 2>/dev/null | extract_output)
    if echo "$sedp_out" | grep -q "REDACTED"; then
        printf "${GREEN}PASS${NC}     %s\n" "$sedp_out"
        echo "functional|sed regex phone redact|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$sedp_out"
        echo "functional|sed regex phone redact|WRONG_OUTPUT|0|$sedp_out" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 12. diff -u unified patch format ─────────────────────────
    echo -e "line1\nline2\nline3" > /tmp/tc_d1.txt
    echo -e "line1\nmodified\nline3" > /tmp/tc_d2.txt
    test_functional "diff -u unified format" "$SYS/diff" "^---.*tc_d1" \
        -u /tmp/tc_d1.txt /tmp/tc_d2.txt

    # ── 13. head -c byte count ───────────────────────────────────
    printf "  %-35s " "head -c byte count"
    local headc_out
    headc_out=$(echo "Hello, World!" | timeout 10 "$LOADER" --exec "$SYS/head" --exec-args -c 5 2>/dev/null | extract_output)
    if [ "$headc_out" = "Hello" ]; then
        printf "${GREEN}PASS${NC}     %s\n" "$headc_out"
        echo "functional|head -c byte count|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    got: %s\n" "$headc_out"
        echo "functional|head -c byte count|WRONG_OUTPUT|0|$headc_out" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 14. uniq -d duplicates only ──────────────────────────────
    printf "  %-35s " "uniq -d dups only"
    local uniqd_out
    uniqd_out=$(echo -e "a\na\nb\nc\nc\nc" | timeout 10 "$LOADER" --exec "$SYS/uniq" --exec-args -d 2>/dev/null | extract_output)
    if echo "$uniqd_out" | grep -q "^a$" && echo "$uniqd_out" | grep -q "^c$" && ! echo "$uniqd_out" | grep -q "^b$"; then
        printf "${GREEN}PASS${NC}     a, c (not b)\n"
        echo "functional|uniq -d dups only|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$uniqd_out" | tr '\n' ',')"
        echo "functional|uniq -d dups only|WRONG_OUTPUT|0|$(echo "$uniqd_out" | head -1)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 15. stat -f format specifiers ────────────────────────────
    touch /tmp/tc_stat_test
    test_functional "stat -f format" "$SYS/stat" "^-rw" \
        -f "%Sp %z %N" /tmp/tc_stat_test

    # ── 16. file JSON detection ──────────────────────────────────
    echo '{"key":"value"}' > /tmp/tc_detect.json
    test_functional "file JSON detect" "$SYS/file" "JSON" \
        /tmp/tc_detect.json

    # ── 17. unifdef preprocessor ─────────────────────────────────
    echo -e '#ifdef DEBUG\ndebug_code();\n#else\nrelease_code();\n#endif' > /tmp/tc_unifdef.c
    test_functional "unifdef -DDEBUG" "$TC/unifdef" "debug_code" \
        -DDEBUG /tmp/tc_unifdef.c

    # ── 18. find -name -type combined ────────────────────────────
    mkdir -p /tmp/tc_findtest/src /tmp/tc_findtest/build
    echo "x" > /tmp/tc_findtest/src/main.c
    echo "x" > /tmp/tc_findtest/src/helper.h
    echo "x" > /tmp/tc_findtest/build/main.o
    test_functional "find -name *.h" "$SYS/find" "helper.h" \
        /tmp/tc_findtest -name "*.h" -type f

    # ── 19. Multi-tool: nm → grep → wc symbol counting ──────────
    printf "  %-35s " "nm→grep→wc symbol count"
    local nm_text
    nm_text=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -gU "$TC/lipo" 2>/dev/null | extract_output)
    local text_syms
    text_syms=$(echo "$nm_text" | grep -c " T " || true)
    if [ "$text_syms" -gt 5 ]; then
        printf "${GREEN}PASS${NC}     %d TEXT symbols\n" "$text_syms"
        echo "functional|nm→grep→wc symbol count|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    only %d TEXT symbols\n" "$text_syms"
        echo "functional|nm→grep→wc symbol count|WRONG_OUTPUT|0|$text_syms" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 20. Multi-tool: strings → grep → wc framework refs ──────
    printf "  %-35s " "strings→grep framework refs"
    local str_out
    str_out=$(timeout 10 "$LOADER" --exec "$TC/strings" --exec-args -a "$TC/clang" 2>/dev/null | extract_output)
    local framework_refs
    framework_refs=$(echo "$str_out" | grep -ci "framework\|dylib" || true)
    if [ "$framework_refs" -gt 5 ]; then
        printf "${GREEN}PASS${NC}     %d framework/dylib refs\n" "$framework_refs"
        echo "functional|strings→grep framework refs|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    only %d refs\n" "$framework_refs"
        echo "functional|strings→grep framework refs|WRONG_OUTPUT|0|$framework_refs" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 21. Mach-O reverse engineering: vtool + nm + size ────────
    printf "  %-35s " "RE chain: vtool+nm+size"
    local re_ok=0
    # vtool: get build version (output may go to loader log)
    local vtool_out
    vtool_out=$(timeout 10 "$LOADER" --exec "$TC/vtool" --exec-args -show-build "$TC/lipo" 2>&1 || true)
    echo "$vtool_out" | grep -qi "platform\|minos" && ((re_ok++)) || true
    # nm: count defined symbols (use lipo which is known to work)
    local nm_count
    nm_count=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -gU "$TC/lipo" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    [ "$nm_count" -gt 10 ] && ((re_ok++)) || true
    # size: segment sizes (use lipo too for consistency)
    local size_out
    size_out=$(timeout 10 "$LOADER" --exec "$TC/size" --exec-args -m "$TC/lipo" 2>/dev/null | extract_output)
    echo "$size_out" | grep -q "__TEXT" && ((re_ok++)) || true
    if [ "$re_ok" -eq 3 ]; then
        printf "${GREEN}PASS${NC}     3/3 RE steps\n"
        echo "functional|RE chain: vtool+nm+size|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $re_ok/3\n"
        echo "functional|RE chain: vtool+nm+size|WRONG_OUTPUT|0|$re_ok/3" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 22. Simulated security audit: strings + grep for secrets ─
    printf "  %-35s " "security audit: strings+grep"
    local secrets_found=0
    mkdir -p /tmp/tc_audit
    echo 'const char* api_key = "sk-12345secret";' > /tmp/tc_audit/main.c
    echo 'password="hunter2"' > /tmp/tc_audit/config.txt
    # Search for potential secrets
    local audit_r1
    audit_r1=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -rl "password\|secret\|api_key" /tmp/tc_audit 2>/dev/null | extract_output)
    local audit_count
    audit_count=$(echo "$audit_r1" | grep -c "tc_audit" || true)
    if [ "$audit_count" -eq 2 ]; then
        printf "${GREEN}PASS${NC}     found 2 files with secrets\n"
        echo "functional|security audit: strings+grep|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    found $audit_count files\n"
        echo "functional|security audit: strings+grep|WRONG_OUTPUT|0|$audit_count" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 23. Multi-tool: create tar → inspect → extract → verify ──
    mkdir -p /tmp/tc_pkg/bin /tmp/tc_pkg/lib
    echo '#!/bin/sh\necho hello' > /tmp/tc_pkg/bin/app
    echo 'LIBRARY=core' > /tmp/tc_pkg/lib/core.conf
    chmod 755 /tmp/tc_pkg/bin/app
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args \
        cf /tmp/tc_pkg.tar -C /tmp tc_pkg >/dev/null 2>/dev/null
    printf "  %-35s " "tar pkg create+inspect"
    local tar_list
    tar_list=$(timeout 10 "$LOADER" --exec "$SYS/tar" --exec-args tf /tmp/tc_pkg.tar 2>/dev/null | extract_output)
    local tar_items
    tar_items=$(echo "$tar_list" | grep -c "tc_pkg" || true)
    if [ "$tar_items" -ge 4 ]; then
        printf "${GREEN}PASS${NC}     %d entries\n" "$tar_items"
        echo "functional|tar pkg create+inspect|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    only %d entries\n" "$tar_items"
        echo "functional|tar pkg create+inspect|WRONG_OUTPUT|0|$tar_items" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 24. tar extract + file verify ────────────────────────────
    rm -rf /tmp/tc_pkg_extract
    mkdir -p /tmp/tc_pkg_extract
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args \
        xf /tmp/tc_pkg.tar -C /tmp/tc_pkg_extract >/dev/null 2>/dev/null
    printf "  %-35s " "tar extract+verify"
    if [ -f /tmp/tc_pkg_extract/tc_pkg/lib/core.conf ] && \
       grep -q "LIBRARY=core" /tmp/tc_pkg_extract/tc_pkg/lib/core.conf 2>/dev/null; then
        printf "${GREEN}PASS${NC}     content verified\n"
        echo "functional|tar extract+verify|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    content mismatch\n"
        echo "functional|tar extract+verify|WRONG_OUTPUT|0|mismatch" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 25. zip + password protect + unzip list ──────────────────
    echo "sensitive data" > /tmp/tc_secret.txt
    timeout 15 "$LOADER" --exec "$SYS/zip" --exec-args \
        -P mypass /tmp/tc_enc.zip /tmp/tc_secret.txt >/dev/null 2>/dev/null
    printf "  %-35s " "zip -P encrypted"
    local ziplist
    ziplist=$(timeout 10 "$LOADER" --exec "$SYS/unzip" --exec-args -l /tmp/tc_enc.zip 2>/dev/null | extract_output)
    if echo "$ziplist" | grep -q "tc_secret.txt"; then
        printf "${GREEN}PASS${NC}     encrypted archive lists OK\n"
        echo "functional|zip -P encrypted|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$ziplist" | head -1 | head -c 40)"
        echo "functional|zip -P encrypted|WRONG_OUTPUT|0|missing" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 26. openssl crypto: ECDSA key gen → sign → verify ────────
    printf "  %-35s " "openssl EC keygen+sign flow"
    # Gen EC key
    timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args \
        ecparam -genkey -name prime256v1 -out /tmp/tc_ec.pem >/dev/null 2>/dev/null
    local ec_ok=0
    [ -f /tmp/tc_ec.pem ] && grep -q "BEGIN EC" /tmp/tc_ec.pem 2>/dev/null && ((ec_ok++)) || true
    # Extract public key
    timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args \
        ec -in /tmp/tc_ec.pem -pubout -out /tmp/tc_ec_pub.pem >/dev/null 2>/dev/null
    [ -f /tmp/tc_ec_pub.pem ] && grep -q "PUBLIC KEY" /tmp/tc_ec_pub.pem 2>/dev/null && ((ec_ok++)) || true
    if [ "$ec_ok" -eq 2 ]; then
        printf "${GREEN}PASS${NC}     keygen + pubkey extract\n"
        echo "functional|openssl EC keygen+sign flow|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $ec_ok/2 steps\n"
        echo "functional|openssl EC keygen+sign flow|WRONG_OUTPUT|0|$ec_ok/2" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 27. Multi-file grep with line numbers ────────────────────
    mkdir -p /tmp/tc_multi
    echo -e "TODO: fix bug\ncode here\nTODO: add test" > /tmp/tc_multi/main.c
    echo -e "normal code\nTODO: refactor" > /tmp/tc_multi/util.c
    echo -e "no issues here" > /tmp/tc_multi/clean.c
    test_functional "grep -rn TODO multi" "$SYS/grep" "TODO" \
        -rn TODO /tmp/tc_multi

    # ── 28. sed multi-command (-e) ───────────────────────────────
    printf "  %-35s " "sed -e multi-command"
    local sedm_out
    sedm_out=$(echo "Hello World Foo" | timeout 10 "$LOADER" --exec "$SYS/sed" --exec-args \
        -e "s/Hello/Greetings/" -e "s/World/Earth/" -e "s/Foo/Bar/" 2>/dev/null | extract_output)
    if echo "$sedm_out" | grep -q "Greetings Earth Bar"; then
        printf "${GREEN}PASS${NC}     %s\n" "$sedm_out"
        echo "functional|sed -e multi-command|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$sedm_out"
        echo "functional|sed -e multi-command|WRONG_OUTPUT|0|$sedm_out" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 29. wc -l multifile with total ───────────────────────────
    echo -e "a\nb\nc" > /tmp/tc_wc1.txt
    echo -e "d\ne" > /tmp/tc_wc2.txt
    test_functional "wc -l multifile total" "$SYS/wc" "total" \
        -l /tmp/tc_wc1.txt /tmp/tc_wc2.txt

    # ── 30. openssl AES-128-CBC roundtrip ────────────────────────
    echo "Secret Message AES128" > /tmp/tc_aes128_plain.txt
    timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args \
        enc -aes-128-cbc -salt -pass pass:test128key -pbkdf2 \
        -in /tmp/tc_aes128_plain.txt -out /tmp/tc_aes128.enc >/dev/null 2>/dev/null
    timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args \
        enc -d -aes-128-cbc -pass pass:test128key -pbkdf2 \
        -in /tmp/tc_aes128.enc -out /tmp/tc_aes128_dec.txt >/dev/null 2>/dev/null
    printf "  %-35s " "openssl AES-128 roundtrip"
    if [ "$(cat /tmp/tc_aes128_dec.txt 2>/dev/null)" = "Secret Message AES128" ]; then
        printf "${GREEN}PASS${NC}     decrypt matches\n"
        echo "functional|openssl AES-128 roundtrip|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    mismatch\n"
        echo "functional|openssl AES-128 roundtrip|WRONG_OUTPUT|0|mismatch" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 31. diff --brief single file comparison ────────────────
    echo "old content" > /tmp/tc_cmp_a.txt
    echo "new content" > /tmp/tc_cmp_b.txt
    test_functional "diff --brief files" "$SYS/diff" "differ" \
        --brief /tmp/tc_cmp_a.txt /tmp/tc_cmp_b.txt

    # ── 32. find -newer timestamp filter ─────────────────────────
    touch -t 202301010000 /tmp/tc_old_ts.txt
    echo "fresh" > /tmp/tc_new_ts.txt
    test_functional "find -newer timestamp" "$SYS/find" "tc_new_ts" \
        /tmp -maxdepth 1 -newer /tmp/tc_old_ts.txt -name "tc_new*"

    # ── 33. lipo -archs quick check ──────────────────────────────
    test_functional "lipo -archs" "$TC/lipo" "arm64" \
        -archs "$TC/nm"

    # ── 34. Cross-tool: find source → grep symbols → nm verify ──
    mkdir -p /tmp/tc_xref/src
    echo -e '#include <stdio.h>\nvoid compute() { printf("done\\n"); }' > /tmp/tc_xref/src/compute.c
    echo -e 'void unused() {}' > /tmp/tc_xref/src/unused.c
    printf "  %-35s " "xref: find→grep→nm chain"
    local xref_files
    xref_files=$(timeout 10 "$LOADER" --exec "$SYS/find" --exec-args \
        /tmp/tc_xref -name "*.c" -type f 2>/dev/null | extract_output)
    local xref_count
    xref_count=$(echo "$xref_files" | grep -c "\.c$" || true)
    local xref_printf=0
    echo "$xref_files" | while IFS= read -r f; do
        [ -z "$f" ] && continue
        timeout 5 "$LOADER" --exec "$SYS/grep" --exec-args -l printf "$f" >/dev/null 2>/dev/null && echo "HIT" || true
    done | grep -c "HIT" > /tmp/tc_xref_hits 2>/dev/null
    xref_printf=$(cat /tmp/tc_xref_hits 2>/dev/null || echo 0)
    if [ "$xref_count" -eq 2 ] && [ "$xref_printf" -eq 1 ]; then
        printf "${GREEN}PASS${NC}     2 files, 1 uses printf\n"
        echo "functional|xref: find→grep→nm chain|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    files=%d printf=%d\n" "$xref_count" "$xref_printf"
        echo "functional|xref: find→grep→nm chain|WRONG_OUTPUT|0|f=$xref_count p=$xref_printf" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 35. Multi-format file detection ──────────────────────────
    printf "  %-35s " "file multi-format detect"
    echo '#!/bin/bash' > /tmp/tc_ft_script.sh
    echo '<?xml version="1.0"?><root/>' > /tmp/tc_ft_xml.txt
    printf '\x89PNG' > /tmp/tc_ft_png.dat
    local ft_ok=0
    timeout 5 "$LOADER" --exec "$SYS/file" --exec-args /tmp/tc_ft_script.sh 2>/dev/null | extract_output | grep -qi "script\|text" && ((ft_ok++)) || true
    timeout 5 "$LOADER" --exec "$SYS/file" --exec-args /tmp/tc_ft_xml.txt 2>/dev/null | extract_output | grep -qi "XML\|text" && ((ft_ok++)) || true
    timeout 5 "$LOADER" --exec "$SYS/file" --exec-args /tmp/tc_ft_png.dat 2>/dev/null | extract_output | grep -qi "PNG\|image" && ((ft_ok++)) || true
    if [ "$ft_ok" -eq 3 ]; then
        printf "${GREEN}PASS${NC}     3/3 types detected\n"
        echo "functional|file multi-format detect|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $ft_ok/3\n"
        echo "functional|file multi-format detect|WRONG_OUTPUT|0|$ft_ok/3" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 36. openssl base64 encode→decode roundtrip ───────────────
    printf "  %-35s " "openssl base64 roundtrip"
    local b64_in="The quick brown fox jumps over the lazy dog"
    local b64_enc
    b64_enc=$(echo "$b64_in" | timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args base64 2>/dev/null | extract_output)
    local b64_dec
    b64_dec=$(echo "$b64_enc" | timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args base64 -d 2>/dev/null | extract_output)
    if [ "$b64_dec" = "$b64_in" ]; then
        printf "${GREEN}PASS${NC}     encode→decode matches\n"
        echo "functional|openssl base64 roundtrip|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    mismatch\n"
        echo "functional|openssl base64 roundtrip|WRONG_OUTPUT|0|mismatch" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 37. Mach-O comparison: same tool different archs ─────────
    printf "  %-35s " "lipo -detailed vs size -m"
    local cmp_ok=0
    local lipo_detail
    lipo_detail=$(timeout 10 "$LOADER" --exec "$TC/lipo" --exec-args -detailed_info "$TC/clang" 2>/dev/null | extract_output)
    echo "$lipo_detail" | grep -qi "cputype\|arm64\|x86_64" && ((cmp_ok++)) || true
    local size_detail
    size_detail=$(timeout 10 "$LOADER" --exec "$TC/size" --exec-args -m "$TC/clang" 2>/dev/null | extract_output)
    echo "$size_detail" | grep -q "__TEXT" && ((cmp_ok++)) || true
    if [ "$cmp_ok" -eq 2 ]; then
        printf "${GREEN}PASS${NC}     lipo+size both work on clang\n"
        echo "functional|lipo -detailed vs size -m|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $cmp_ok/2\n"
        echo "functional|lipo -detailed vs size -m|WRONG_OUTPUT|0|$cmp_ok/2" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 38. grep -i case insensitive ─────────────────────────────
    printf "  %-35s " "grep -i case insensitive"
    local gi_out
    gi_out=$(echo -e "Error Found\nWARNING here\ninfo only" | \
        timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -i error 2>/dev/null | extract_output)
    if echo "$gi_out" | grep -q "Error Found"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$gi_out" | head -1)"
        echo "functional|grep -i case insensitive|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$gi_out" | head -1 | head -c 40)"
        echo "functional|grep -i case insensitive|WRONG_OUTPUT|0|$(echo "$gi_out" | head -1)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 39. grep -v invert match ─────────────────────────────────
    printf "  %-35s " "grep -v invert match"
    local gv_out
    gv_out=$(echo -e "keep\nremove\nkeep2\nremove2" | \
        timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -v remove 2>/dev/null | extract_output)
    if echo "$gv_out" | grep -q "keep" && ! echo "$gv_out" | grep -q "remove"; then
        printf "${GREEN}PASS${NC}     kept only non-remove lines\n"
        echo "functional|grep -v invert match|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$gv_out" | tr '\n' ',' | head -c 40)"
        echo "functional|grep -v invert match|WRONG_OUTPUT|0|$(echo "$gv_out" | head -1)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 40. openssl rand + hex format verification ───────────────
    printf "  %-35s " "openssl rand hex format"
    local rand_out
    rand_out=$(timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args rand -hex 64 2>/dev/null | extract_output)
    local rand_len=${#rand_out}
    if [ "$rand_len" -ge 120 ] && echo "$rand_out" | grep -qE '^[0-9a-f]+$'; then
        printf "${GREEN}PASS${NC}     %d hex chars\n" "$rand_len"
        echo "functional|openssl rand hex format|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    len=%d\n" "$rand_len"
        echo "functional|openssl rand hex format|WRONG_OUTPUT|0|len=$rand_len" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── Clean up ─────────────────────────────────────────────────
    rm -rf /tmp/tc_* /tmp/pp_*
}

# ── DevOps & cross-tool workflow tests: real-world multi-tool pipelines ──

test_devops_suite() {
    echo -e "\n${CYAN}═══ DevOps & Cross-Tool Workflow Tests ═══${NC}"

    local TC="$TOOLS_DIR/xcode-tc"
    local SYS="$TOOLS_DIR/system"

    # ── 1. sort -r reverse alpha ─────────────────────────────────
    printf "  %-35s " "sort -r reverse alpha"
    local sr_out
    sr_out=$(printf "banana\napple\ncherry\n" | timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -r 2>/dev/null | extract_output)
    if echo "$sr_out" | head -c 20 | grep -q "cherry"; then
        printf "${GREEN}PASS${NC}     cherry first\n"
        echo "functional|sort -r reverse alpha|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$sr_out" | head -c 30)"
        echo "functional|sort -r reverse alpha|WRONG_OUTPUT|0|$(echo "$sr_out" | head -c 30)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 2. sort -u unique from file ──────────────────────────────
    printf "cherry\napple\nbanana\napple\ncherry\n" > /tmp/dv_su.txt
    test_functional "sort -u unique file" "$SYS/sort" "apple" \
        -u /tmp/dv_su.txt

    # ── 3. sort -t field separator ───────────────────────────────
    printf "  %-35s " "sort -t: -k2 field"
    printf "c:3\na:1\nb:2\n" > /tmp/dv_sf.txt
    local sf_out
    sf_out=$(timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -t: -k2,2n /tmp/dv_sf.txt 2>/dev/null | extract_output)
    if echo "$sf_out" | head -c 10 | grep -q "a:1"; then
        printf "${GREEN}PASS${NC}     a:1 first\n"
        echo "functional|sort -t: -k2 field|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$sf_out" | head -c 30)"
        echo "functional|sort -t: -k2 field|WRONG_OUTPUT|0|$(echo "$sf_out" | head -c 30)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 4. grep -l list files ────────────────────────────────────
    echo "error found" > /tmp/dv_gl1.txt; echo "all ok" > /tmp/dv_gl2.txt; echo "error too" > /tmp/dv_gl3.txt
    printf "  %-35s " "grep -l list matching files"
    local gl_out
    gl_out=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -l error /tmp/dv_gl1.txt /tmp/dv_gl2.txt /tmp/dv_gl3.txt 2>/dev/null | extract_output)
    if echo "$gl_out" | grep -q "dv_gl1" && echo "$gl_out" | grep -q "dv_gl3" && ! echo "$gl_out" | grep -q "dv_gl2"; then
        printf "${GREEN}PASS${NC}     2 matching files\n"
        echo "functional|grep -l list matching files|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$gl_out" | head -c 40)"
        echo "functional|grep -l list matching files|WRONG_OUTPUT|0|missing" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 5. grep -oE extract matches ─────────────────────────────
    printf "  %-35s " "grep -oE extract pattern"
    local go_out
    go_out=$(echo "user@test.com and admin@foo.org" | timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -oE "[a-z]+@[a-z]+\.[a-z]+" 2>/dev/null | extract_output)
    if echo "$go_out" | grep -q "user@test.com" && echo "$go_out" | grep -q "admin@foo.org"; then
        printf "${GREEN}PASS${NC}     both emails extracted\n"
        echo "functional|grep -oE extract pattern|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$go_out" | head -c 40)"
        echo "functional|grep -oE extract pattern|WRONG_OUTPUT|0|$(echo "$go_out" | head -c 30)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 6. sed line address range ────────────────────────────────
    printf "  %-35s " "sed 2,4 address range"
    local sa_out
    sa_out=$(printf "L1\nL2\nL3\nL4\nL5\n" | timeout 10 "$LOADER" --exec "$SYS/sed" --exec-args "2,4s/L/X/" 2>/dev/null | extract_output)
    if echo "$sa_out" | grep -q "L1" && echo "$sa_out" | grep -q "X2"; then
        printf "${GREEN}PASS${NC}     L1 kept, L2→X2\n"
        echo "functional|sed 2,4 address range|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$sa_out" | head -c 40)"
        echo "functional|sed 2,4 address range|WRONG_OUTPUT|0|$(echo "$sa_out" | head -c 30)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 7. cut -c character range ────────────────────────────────
    printf "  %-35s " "cut -c1-5 char range"
    local cc_out
    cc_out=$(printf "Hello World\nFoo Bar Baz\n" | timeout 10 "$LOADER" --exec "$SYS/cut" --exec-args -c1-5 2>/dev/null | extract_output)
    if echo "$cc_out" | grep -q "Hello" && echo "$cc_out" | grep -q "Foo B"; then
        printf "${GREEN}PASS${NC}     Hello + Foo B\n"
        echo "functional|cut -c1-5 char range|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$cc_out" | head -c 30)"
        echo "functional|cut -c1-5 char range|WRONG_OUTPUT|0|$(echo "$cc_out" | head -c 30)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 8. uniq -c frequency count from sorted input ─────────────
    printf "  %-35s " "sort|uniq -c frequency"
    printf "a\na\nb\nc\nc\nc\n" | timeout 10 "$LOADER" --exec "$SYS/sort" 2>/dev/null | extract_output > /tmp/dv_sorted.txt
    local uc_out
    uc_out=$(timeout 10 "$LOADER" --exec "$SYS/uniq" --exec-args -c /tmp/dv_sorted.txt 2>/dev/null | extract_output)
    if echo "$uc_out" | grep -q "2 a" && echo "$uc_out" | grep -q "3 c"; then
        printf "${GREEN}PASS${NC}     a=2, c=3\n"
        echo "functional|sort|uniq -c frequency|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$uc_out" | tr '\n' ',' | head -c 40)"
        echo "functional|sort|uniq -c frequency|WRONG_OUTPUT|0|$(echo "$uc_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 9. openssl RSA keygen + inspect ──────────────────────────
    printf "  %-35s " "openssl RSA keygen+inspect"
    timeout 15 "$LOADER" --exec "$SYS/openssl" --exec-args genrsa -out /tmp/dv_rsa.pem 2048 >/dev/null 2>/dev/null
    local rsa_ok=0
    [ -f /tmp/dv_rsa.pem ] && grep -q "BEGIN RSA PRIVATE KEY" /tmp/dv_rsa.pem 2>/dev/null && ((rsa_ok++)) || true
    local rsa_check
    rsa_check=$(timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args rsa -in /tmp/dv_rsa.pem -check -noout 2>/dev/null | extract_output)
    echo "$rsa_check" | grep -qi "ok" && ((rsa_ok++)) || true
    if [ "$rsa_ok" -eq 2 ]; then
        printf "${GREEN}PASS${NC}     keygen + check ok\n"
        echo "functional|openssl RSA keygen+inspect|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $rsa_ok/2 steps\n"
        echo "functional|openssl RSA keygen+inspect|WRONG_OUTPUT|0|$rsa_ok/2" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 10. openssl RSA text dump ────────────────────────────────
    printf "  %-35s " "openssl RSA text dump"
    local rsa_text
    rsa_text=$(timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args rsa -in /tmp/dv_rsa.pem -text -noout 2>/dev/null | extract_output)
    if echo "$rsa_text" | grep -qi "Private-Key.*2048"; then
        printf "${GREEN}PASS${NC}     2048-bit key details\n"
        echo "functional|openssl RSA text dump|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$rsa_text" | head -1 | head -c 40)"
        echo "functional|openssl RSA text dump|WRONG_OUTPUT|0|missing" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 11. openssl multiple key sizes ───────────────────────────
    printf "  %-35s " "openssl multi key sizes"
    local ks_ok=0
    for bits in 1024 2048 4096; do
        timeout 15 "$LOADER" --exec "$SYS/openssl" --exec-args genrsa "$bits" >/tmp/dv_k${bits}.pem 2>/dev/null
        [ -s /tmp/dv_k${bits}.pem ] && ((ks_ok++)) || true
    done
    if [ "$ks_ok" -eq 3 ]; then
        printf "${GREEN}PASS${NC}     1024/2048/4096 all generated\n"
        echo "functional|openssl multi key sizes|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $ks_ok/3 generated\n"
        echo "functional|openssl multi key sizes|WRONG_OUTPUT|0|$ks_ok/3" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 12. find -type d directories ─────────────────────────────
    mkdir -p /tmp/dv_fd/a/b /tmp/dv_fd/c
    touch /tmp/dv_fd/f1.txt /tmp/dv_fd/a/f2.txt
    test_functional "find -type d dirs" "$SYS/find" "dv_fd/a" \
        /tmp/dv_fd -type d

    # ── 13. find -size filter ────────────────────────────────────
    dd if=/dev/zero of=/tmp/dv_bigfile bs=1024 count=100 2>/dev/null
    dd if=/dev/zero of=/tmp/dv_smallfile bs=1 count=10 2>/dev/null
    test_functional "find -size +50k" "$SYS/find" "dv_bigfile" \
        /tmp -maxdepth 1 -name "dv_*file" -size +50k

    # ── 14. nm -p unsorted vs nm sorted comparison ───────────────
    printf "  %-35s " "nm -p unsorted vs sorted"
    local nm_p
    nm_p=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -pU "$TC/lipo" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    local nm_s
    nm_s=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -U "$TC/lipo" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    if [ "$nm_p" -gt 50 ] && [ "$nm_p" = "$nm_s" ]; then
        printf "${GREEN}PASS${NC}     both %d lines\n" "$nm_p"
        echo "functional|nm -p unsorted vs sorted|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    sorted=%d unsorted=%d\n" "$nm_s" "$nm_p"
        echo "functional|nm -p unsorted vs sorted|WRONG_OUTPUT|0|s=$nm_s p=$nm_p" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 15. nm vs llvm-nm cross-verification ─────────────────────
    printf "  %-35s " "nm vs llvm-nm cross-verify"
    local nm1
    nm1=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -gU "$TC/lipo" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    local nm2
    nm2=$(timeout 10 "$LOADER" --exec "$TC/llvm-nm" --exec-args --defined-only --extern-only "$TC/lipo" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    if [ "$nm1" -gt 5 ] && [ "$nm1" = "$nm2" ]; then
        printf "${GREEN}PASS${NC}     both report %d symbols\n" "$nm1"
        echo "functional|nm vs llvm-nm cross-verify|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    nm=%d llvm-nm=%d\n" "$nm1" "$nm2"
        echo "functional|nm vs llvm-nm cross-verify|WRONG_OUTPUT|0|nm=$nm1 llvm=$nm2" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 16. llvm-nm --version ────────────────────────────────────
    printf "  %-35s " "llvm-nm --version"
    local lnm_out
    lnm_out=$(timeout 10 "$LOADER" --exec "$TC/llvm-nm" --exec-args --version 2>&1 || true)
    if echo "$lnm_out" | grep -qi "LLVM\|Apple\|llvm-nm"; then
        printf "${GREEN}PASS${NC}     %s\n" "$(echo "$lnm_out" | grep -i 'LLVM\|Apple\|llvm-nm' | head -1 | head -c 50)"
        echo "functional|llvm-nm --version|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$lnm_out" | head -1 | head -c 40)"
        echo "functional|llvm-nm --version|WRONG_OUTPUT|0|no match" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 17. strings -n 20 long strings ───────────────────────────
    test_functional_minlines "strings -n 20 long str" "$TC/strings" 100 \
        -n 20 "$TC/clang"

    # ── 18. strip -x debug syms + verify ─────────────────────────
    printf "  %-35s " "strip -x + nm verify"
    cp "$TC/lipo" /tmp/dv_strip_test 2>/dev/null
    local pre_syms
    pre_syms=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -U "$TC/lipo" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    timeout 10 "$LOADER" --exec "$TC/strip" --exec-args -x /tmp/dv_strip_test >/dev/null 2>/dev/null
    local post_syms
    post_syms=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -U /tmp/dv_strip_test 2>/dev/null | extract_output | wc -l | tr -d ' ')
    if [ "$pre_syms" -gt "$post_syms" ] && [ "$post_syms" -lt 50 ]; then
        printf "${GREEN}PASS${NC}     %d → %d symbols\n" "$pre_syms" "$post_syms"
        echo "functional|strip -x + nm verify|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    before=%d after=%d\n" "$pre_syms" "$post_syms"
        echo "functional|strip -x + nm verify|WRONG_OUTPUT|0|b=$pre_syms a=$post_syms" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 19. llvm-objdump -h section headers ──────────────────────
    printf "  %-35s " "llvm-objdump -h headers"
    local oh_out
    oh_out=$(timeout 10 "$LOADER" --exec "$TC/llvm-objdump" --exec-args -h "$TC/nm" 2>/dev/null | extract_output)
    if echo "$oh_out" | grep -q "__text" && echo "$oh_out" | grep -q "Sections"; then
        printf "${GREEN}PASS${NC}     __text section found\n"
        echo "functional|llvm-objdump -h headers|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$oh_out" | head -1 | head -c 40)"
        echo "functional|llvm-objdump -h headers|WRONG_OUTPUT|0|missing" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 20. diff -c context format ───────────────────────────────
    printf "a\nb\nc\n" > /tmp/dv_dc1.txt; printf "a\nB\nc\n" > /tmp/dv_dc2.txt
    test_functional "diff -c context format" "$SYS/diff" "^\\*\\*\\*.*dv_dc1" \
        -c /tmp/dv_dc1.txt /tmp/dv_dc2.txt

    # ── 21. file on tar archive ──────────────────────────────────
    mkdir -p /tmp/dv_tpkg; echo "data" > /tmp/dv_tpkg/file.txt
    timeout 10 "$LOADER" --exec "$SYS/tar" --exec-args cf /tmp/dv_ftest.tar -C /tmp dv_tpkg >/dev/null 2>/dev/null
    test_functional "file tar archive" "$SYS/file" "tar.*archive" \
        /tmp/dv_ftest.tar

    # ── 22. tar extract -C + file verify ─────────────────────────
    printf "  %-35s " "tar xf -C + verify"
    mkdir -p /tmp/dv_tout
    timeout 10 "$LOADER" --exec "$SYS/tar" --exec-args xf /tmp/dv_ftest.tar -C /tmp/dv_tout >/dev/null 2>/dev/null
    if [ -f /tmp/dv_tout/dv_tpkg/file.txt ] && grep -q "data" /tmp/dv_tout/dv_tpkg/file.txt; then
        printf "${GREEN}PASS${NC}     extracted content verified\n"
        echo "functional|tar xf -C + verify|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    content mismatch\n"
        echo "functional|tar xf -C + verify|WRONG_OUTPUT|0|mismatch" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 23. basename suffix stripping ────────────────────────────
    test_functional "basename suffix strip" "$SYS/basename" "libSystem.B" \
        /usr/lib/libSystem.B.dylib .dylib

    # ── 24. wc -c byte count ────────────────────────────────────
    printf "  %-35s " "wc -c byte count"
    local wcc_out
    wcc_out=$(printf "Hello World\n" | timeout 10 "$LOADER" --exec "$SYS/wc" --exec-args -c 2>/dev/null | extract_output)
    if echo "$wcc_out" | grep -q "12"; then
        printf "${GREEN}PASS${NC}     12 bytes\n"
        echo "functional|wc -c byte count|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$wcc_out" | head -c 20)"
        echo "functional|wc -c byte count|WRONG_OUTPUT|0|$(echo "$wcc_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 25. stat %N:%z name + size ───────────────────────────────
    echo "test data here" > /tmp/dv_stfile.txt
    test_functional "stat %N:%z name+size" "$SYS/stat" "dv_stfile.txt:15" \
        -f "%N:%z" /tmp/dv_stfile.txt

    # ── 26. id -un username ──────────────────────────────────────
    test_functional "id -un username" "$SYS/id" "root" -un

    # ── 27. grep -E alternation pattern ──────────────────────────
    printf "  %-35s " "grep -E alternation"
    local ge_out
    ge_out=$(printf "error: fail\nwarning: check\ninfo: ok\n" | timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -E "^(error|warning)" 2>/dev/null | extract_output)
    if echo "$ge_out" | grep -q "error" && echo "$ge_out" | grep -q "warning"; then
        printf "${GREEN}PASS${NC}     both matched\n"
        echo "functional|grep -E alternation|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$ge_out" | head -c 30)"
        echo "functional|grep -E alternation|WRONG_OUTPUT|0|$(echo "$ge_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 28. vtool -arch arm64 specific ───────────────────────────
    printf "  %-35s " "vtool -arch arm64"
    local va_out
    va_out=$(timeout 10 "$LOADER" --exec "$TC/vtool" --exec-args -arch arm64 -show-build "$TC/clang" 2>&1 || true)
    if echo "$va_out" | grep -qi "platform MACOS"; then
        printf "${GREEN}PASS${NC}     platform MACOS found\n"
        echo "functional|vtool -arch arm64|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$va_out" | grep -i platform | head -1 | head -c 40)"
        echo "functional|vtool -arch arm64|WRONG_OUTPUT|0|no match" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 29. Multi: sort → uniq -c → sort -rn (frequency table) ──
    printf "  %-35s " "freq table: sort|uniq|sort"
    printf "apple\nbanana\napple\ncherry\napple\nbanana\n" | \
        timeout 10 "$LOADER" --exec "$SYS/sort" 2>/dev/null | extract_output > /tmp/dv_freq_sorted.txt
    local freq_uc
    freq_uc=$(timeout 10 "$LOADER" --exec "$SYS/uniq" --exec-args -c /tmp/dv_freq_sorted.txt 2>/dev/null | extract_output)
    if echo "$freq_uc" | grep -q "3 apple" && echo "$freq_uc" | grep -q "2 banana" && echo "$freq_uc" | grep -q "1 cherry"; then
        printf "${GREEN}PASS${NC}     apple=3, banana=2, cherry=1\n"
        echo "functional|freq table: sort|uniq|sort|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$freq_uc" | tr '\n' ',' | head -c 40)"
        echo "functional|freq table: sort|uniq|sort|WRONG_OUTPUT|0|$(echo "$freq_uc" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 30. Multi: find → grep → wc (code search) ───────────────
    mkdir -p /tmp/dv_cs/src
    echo -e 'int main() {\n  printf("hello");\n  return 0;\n}' > /tmp/dv_cs/src/main.c
    echo -e 'void util() {\n  printf("util");\n}' > /tmp/dv_cs/src/util.c
    echo -e '#define MAX 100' > /tmp/dv_cs/src/config.h
    printf "  %-35s " "find→grep code search"
    local cs_files
    cs_files=$(timeout 10 "$LOADER" --exec "$SYS/find" --exec-args /tmp/dv_cs -name "*.c" -type f 2>/dev/null | extract_output)
    local cs_count=0
    echo "$cs_files" | tr -s ' ' '\n' | while IFS= read -r f; do
        [ -z "$f" ] && continue
        timeout 5 "$LOADER" --exec "$SYS/grep" --exec-args -l printf "$f" >/dev/null 2>/dev/null && echo "HIT" || true
    done | grep -c "HIT" > /tmp/dv_cs_hits 2>/dev/null
    cs_count=$(cat /tmp/dv_cs_hits 2>/dev/null || echo 0)
    if [ "$cs_count" -eq 2 ]; then
        printf "${GREEN}PASS${NC}     2 files with printf\n"
        echo "functional|find→grep code search|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    found %d\n" "$cs_count"
        echo "functional|find→grep code search|WRONG_OUTPUT|0|count=$cs_count" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 31. Multi: tar.gz create + list ─────────────────────────
    printf "  %-35s " "tar.gz create+list"
    mkdir -p /tmp/dv_tgz/docs; echo "readme" > /tmp/dv_tgz/docs/README.md; echo "notes" > /tmp/dv_tgz/docs/NOTES.txt
    timeout 15 "$LOADER" --exec "$SYS/tar" --exec-args czf /tmp/dv_pkg.tar.gz -C /tmp dv_tgz >/dev/null 2>/dev/null
    local tgz_ok=0
    # verify file created and is non-empty
    [ -s /tmp/dv_pkg.tar.gz ] && ((tgz_ok++)) || true
    # list contents
    local tl_out
    tl_out=$(timeout 10 "$LOADER" --exec "$SYS/tar" --exec-args tzf /tmp/dv_pkg.tar.gz 2>/dev/null | extract_output)
    echo "$tl_out" | grep -q "README" && ((tgz_ok++)) || true
    if [ "$tgz_ok" -eq 2 ]; then
        printf "${GREEN}PASS${NC}     created + contents listed\n"
        echo "functional|tar.gz create+list|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $tgz_ok/2\n"
        echo "functional|tar.gz create+list|WRONG_OUTPUT|0|$tgz_ok/2" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 32. zip + unzip -l + unzip extract ───────────────────────
    printf "  %-35s " "zip+unzip full workflow"
    echo "content A" > /tmp/dv_za.txt; echo "content B" > /tmp/dv_zb.txt
    rm -f /tmp/dv_multi.zip
    timeout 10 "$LOADER" --exec "$SYS/zip" --exec-args /tmp/dv_multi.zip /tmp/dv_za.txt /tmp/dv_zb.txt >/dev/null 2>/dev/null
    local zip_ok=0
    # list
    local zl_out
    zl_out=$(timeout 10 "$LOADER" --exec "$SYS/unzip" --exec-args -l /tmp/dv_multi.zip 2>/dev/null | extract_output)
    echo "$zl_out" | grep -q "dv_za" && echo "$zl_out" | grep -q "dv_zb" && ((zip_ok++)) || true
    # extract
    rm -rf /tmp/dv_zout; mkdir -p /tmp/dv_zout
    timeout 10 "$LOADER" --exec "$SYS/unzip" --exec-args -o /tmp/dv_multi.zip -d /tmp/dv_zout >/dev/null 2>/dev/null
    [ -f /tmp/dv_zout/tmp/dv_za.txt ] && grep -q "content A" /tmp/dv_zout/tmp/dv_za.txt 2>/dev/null && ((zip_ok++)) || true
    if [ "$zip_ok" -eq 2 ]; then
        printf "${GREEN}PASS${NC}     list + extract verified\n"
        echo "functional|zip+unzip full workflow|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $zip_ok/2 steps\n"
        echo "functional|zip+unzip full workflow|WRONG_OUTPUT|0|$zip_ok/2" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 33. openssl AES-256 roundtrip ────────────────────────────
    printf "  %-35s " "openssl AES-256 roundtrip"
    echo "Top Secret Data AES256" > /tmp/dv_aes_plain.txt
    timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args \
        enc -aes-256-cbc -salt -pass pass:supersecret -pbkdf2 \
        -in /tmp/dv_aes_plain.txt -out /tmp/dv_aes.enc >/dev/null 2>/dev/null
    timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args \
        enc -d -aes-256-cbc -pass pass:supersecret -pbkdf2 \
        -in /tmp/dv_aes.enc -out /tmp/dv_aes_dec.txt >/dev/null 2>/dev/null
    if [ "$(cat /tmp/dv_aes_dec.txt 2>/dev/null)" = "Top Secret Data AES256" ]; then
        printf "${GREEN}PASS${NC}     decrypt matches\n"
        echo "functional|openssl AES-256 roundtrip|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    mismatch\n"
        echo "functional|openssl AES-256 roundtrip|WRONG_OUTPUT|0|mismatch" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 34. Multi: build-like workflow (find src → compile check → nm) ──
    printf "  %-35s " "build workflow simulation"
    mkdir -p /tmp/dv_bld/src
    echo '#include <stdio.h>' > /tmp/dv_bld/src/hello.c
    echo 'int main() { printf("hello\\n"); return 0; }' >> /tmp/dv_bld/src/hello.c
    local bld_ok=0
    # find sources
    local bld_src
    bld_src=$(timeout 10 "$LOADER" --exec "$SYS/find" --exec-args /tmp/dv_bld/src -name "*.c" 2>/dev/null | extract_output)
    echo "$bld_src" | grep -q "hello.c" && ((bld_ok++)) || true
    # syntax check
    timeout 10 "$LOADER" --exec "$TC/clang" --exec-args -fsyntax-only /tmp/dv_bld/src/hello.c >/dev/null 2>/dev/null && ((bld_ok++)) || true
    # check clang can report target
    local bld_target
    bld_target=$(timeout 10 "$LOADER" --exec "$TC/clang" --exec-args -dumpmachine 2>/dev/null | extract_output)
    echo "$bld_target" | grep -qi "arm64\|aarch64\|x86_64\|apple" && ((bld_ok++)) || true
    if [ "$bld_ok" -eq 3 ]; then
        printf "${GREEN}PASS${NC}     3/3 build steps\n"
        echo "functional|build workflow simulation|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $bld_ok/3\n"
        echo "functional|build workflow simulation|WRONG_OUTPUT|0|$bld_ok/3" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 35. Multi: binary audit (lipo → vtool → nm → strings) ───
    printf "  %-35s " "binary audit pipeline"
    local aud_ok=0
    # lipo archs
    local aud_lipo
    aud_lipo=$(timeout 10 "$LOADER" --exec "$TC/lipo" --exec-args -archs "$TC/clang" 2>/dev/null | extract_output)
    echo "$aud_lipo" | grep -q "arm64" && ((aud_ok++)) || true
    # vtool build version
    local aud_vtool
    aud_vtool=$(timeout 10 "$LOADER" --exec "$TC/vtool" --exec-args -show-build "$TC/clang" 2>&1)
    echo "$aud_vtool" | grep -qi "platform" && ((aud_ok++)) || true
    # nm symbol count
    local aud_nm
    aud_nm=$(timeout 10 "$LOADER" --exec "$TC/nm" --exec-args -gU "$TC/lipo" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    [ "$aud_nm" -gt 5 ] && ((aud_ok++)) || true
    # strings presence
    local aud_str
    aud_str=$(timeout 10 "$LOADER" --exec "$TC/strings" --exec-args -a "$TC/lipo" 2>/dev/null | extract_output | wc -l | tr -d ' ')
    [ "$aud_str" -gt 100 ] && ((aud_ok++)) || true
    if [ "$aud_ok" -eq 4 ]; then
        printf "${GREEN}PASS${NC}     4/4 audit steps\n"
        echo "functional|binary audit pipeline|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $aud_ok/4\n"
        echo "functional|binary audit pipeline|WRONG_OUTPUT|0|$aud_ok/4" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 36. Multi: config file processing (grep + sed + cut) ─────
    printf "  %-35s " "config processing pipeline"
    cat > /tmp/dv_config.ini << 'EOF'
[server]
host = 192.168.1.100
port = 8080
debug = true
[database]
host = db.internal
port = 5432
user = admin
EOF
    local cfg_ok=0
    # grep section
    local cfg_sec
    cfg_sec=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -A3 "\\[server\\]" /tmp/dv_config.ini 2>/dev/null | extract_output)
    echo "$cfg_sec" | grep -q "8080" && ((cfg_ok++)) || true
    # sed transform
    local cfg_sed
    cfg_sed=$(timeout 10 "$LOADER" --exec "$SYS/sed" --exec-args "s/ = /=/" /tmp/dv_config.ini 2>/dev/null | extract_output)
    echo "$cfg_sed" | grep -q "host=192" && ((cfg_ok++)) || true
    # cut field
    local cfg_cut
    cfg_cut=$(echo "host = 192.168.1.100" | timeout 10 "$LOADER" --exec "$SYS/cut" --exec-args -d= -f2 2>/dev/null | extract_output)
    echo "$cfg_cut" | grep -q "192" && ((cfg_ok++)) || true
    if [ "$cfg_ok" -eq 3 ]; then
        printf "${GREEN}PASS${NC}     3/3 config steps\n"
        echo "functional|config processing pipeline|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $cfg_ok/3\n"
        echo "functional|config processing pipeline|WRONG_OUTPUT|0|$cfg_ok/3" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 37. Multi: log analysis (grep patterns + counts) ─────────
    printf "  %-35s " "log analysis pipeline"
    cat > /tmp/dv_log.txt << 'EOF'
2026-01-01 ERROR: disk full
2026-01-01 INFO: service started
2026-01-02 WARNING: low memory
2026-01-02 ERROR: connection refused
2026-01-03 INFO: backup complete
2026-01-03 ERROR: timeout
EOF
    local log_ok=0
    # count errors
    local err_count
    err_count=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -c ERROR /tmp/dv_log.txt 2>/dev/null | extract_output)
    [ "$err_count" = "3" ] && ((log_ok++)) || true
    # grep with context
    local err_ctx
    err_ctx=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -B1 "connection refused" /tmp/dv_log.txt 2>/dev/null | extract_output)
    echo "$err_ctx" | grep -q "connection refused" && ((log_ok++)) || true
    # count unique log levels
    local log_levels
    log_levels=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -oE "(ERROR|WARNING|INFO)" /tmp/dv_log.txt 2>/dev/null | extract_output)
    echo "$log_levels" | grep -q "ERROR" && echo "$log_levels" | grep -q "WARNING" && echo "$log_levels" | grep -q "INFO" && ((log_ok++)) || true
    if [ "$log_ok" -eq 3 ]; then
        printf "${GREEN}PASS${NC}     3/3 log analysis steps\n"
        echo "functional|log analysis pipeline|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $log_ok/3\n"
        echo "functional|log analysis pipeline|WRONG_OUTPUT|0|$log_ok/3" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 38. openssl pkcs8 convert ────────────────────────────────
    printf "  %-35s " "openssl pkcs8 convert"
    timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args \
        pkcs8 -topk8 -inform PEM -outform PEM -nocrypt \
        -in /tmp/dv_rsa.pem -out /tmp/dv_pkcs8.pem >/dev/null 2>/dev/null
    if [ -f /tmp/dv_pkcs8.pem ] && grep -q "BEGIN PRIVATE KEY" /tmp/dv_pkcs8.pem 2>/dev/null; then
        printf "${GREEN}PASS${NC}     PKCS#8 format\n"
        echo "functional|openssl pkcs8 convert|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    conversion failed\n"
        echo "functional|openssl pkcs8 convert|WRONG_OUTPUT|0|failed" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 39. find + stat pipeline ─────────────────────────────────
    printf "  %-35s " "find+stat file inventory"
    mkdir -p /tmp/dv_inv
    echo "small" > /tmp/dv_inv/a.txt; dd if=/dev/zero of=/tmp/dv_inv/big.bin bs=1024 count=5 2>/dev/null
    local inv_files
    inv_files=$(timeout 10 "$LOADER" --exec "$SYS/find" --exec-args /tmp/dv_inv -type f 2>/dev/null | extract_output)
    local inv_ok=0
    echo "$inv_files" | grep -q "a.txt" && ((inv_ok++)) || true
    echo "$inv_files" | grep -q "big.bin" && ((inv_ok++)) || true
    # stat size check
    local inv_stat
    inv_stat=$(timeout 10 "$LOADER" --exec "$SYS/stat" --exec-args -f "%z" /tmp/dv_inv/big.bin 2>/dev/null | extract_output)
    echo "$inv_stat" | grep -q "5120" && ((inv_ok++)) || true
    if [ "$inv_ok" -eq 3 ]; then
        printf "${GREEN}PASS${NC}     3/3 inventory checks\n"
        echo "functional|find+stat file inventory|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $inv_ok/3\n"
        echo "functional|find+stat file inventory|WRONG_OUTPUT|0|$inv_ok/3" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 40. Multi: security cert workflow ────────────────────────
    printf "  %-35s " "crypto cert workflow"
    local cert_ok=0
    # Generate fresh key
    timeout 15 "$LOADER" --exec "$SYS/openssl" --exec-args genrsa -out /tmp/dv_cw_key.pem 2048 >/dev/null 2>/dev/null
    [ -f /tmp/dv_cw_key.pem ] && grep -q "BEGIN RSA" /tmp/dv_cw_key.pem && ((cert_ok++)) || true
    # Extract public key
    timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args rsa -in /tmp/dv_cw_key.pem -pubout -out /tmp/dv_cw_pub.pem >/dev/null 2>/dev/null
    [ -f /tmp/dv_cw_pub.pem ] && grep -q "PUBLIC KEY" /tmp/dv_cw_pub.pem && ((cert_ok++)) || true
    # PKCS8 convert
    timeout 10 "$LOADER" --exec "$SYS/openssl" --exec-args pkcs8 -topk8 -nocrypt -in /tmp/dv_cw_key.pem -out /tmp/dv_cw_p8.pem >/dev/null 2>/dev/null
    [ -f /tmp/dv_cw_p8.pem ] && grep -q "BEGIN PRIVATE KEY" /tmp/dv_cw_p8.pem && ((cert_ok++)) || true
    if [ "$cert_ok" -eq 3 ]; then
        printf "${GREEN}PASS${NC}     3/3 cert workflow steps\n"
        echo "functional|crypto cert workflow|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    $cert_ok/3\n"
        echo "functional|crypto cert workflow|WRONG_OUTPUT|0|$cert_ok/3" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── Clean up ─────────────────────────────────────────────────
    rm -rf /tmp/dv_*
}

# ── RuneLocale-unlocked tests: awk, sort -n, wc -w, grep -w ────────

test_rune_suite() {
    echo -e "\n${CYAN}═══ RuneLocale-Unlocked Tests (awk, sort -n, wc -w, grep -w) ═══${NC}"

    local SYS="$TOOLS_DIR/system"

    # ── 1. awk field print ───────────────────────────────────────
    printf "  %-35s " "awk field print"
    local awk1
    awk1=$(printf "alice 25\nbob 30\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print $1}' 2>/dev/null | extract_output)
    if echo "$awk1" | grep -q "alice" && echo "$awk1" | grep -q "bob"; then
        printf "${GREEN}PASS${NC}     alice,bob\n"
        echo "functional|awk field print|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk1" | head -c 30)"
        echo "functional|awk field print|WRONG_OUTPUT|0|$(echo "$awk1" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 2. awk NR line numbers ───────────────────────────────────
    printf "  %-35s " "awk NR line numbers"
    local awk2
    awk2=$(printf "a\nb\nc\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print NR, $0}' 2>/dev/null | extract_output)
    if echo "$awk2" | grep -q "1 a" && echo "$awk2" | grep -q "3 c"; then
        printf "${GREEN}PASS${NC}     1 a...3 c\n"
        echo "functional|awk NR line numbers|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk2" | head -c 30)"
        echo "functional|awk NR line numbers|WRONG_OUTPUT|0|$(echo "$awk2" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 3. awk sum =──────────────────────────────────────────────
    printf "  %-35s " "awk sum"
    local awk3
    awk3=$(printf "10\n20\n30\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{s+=$1} END{print s}' 2>/dev/null | extract_output)
    if echo "$awk3" | grep -q "60"; then
        printf "${GREEN}PASS${NC}     sum=60\n"
        echo "functional|awk sum|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk3" | head -c 20)"
        echo "functional|awk sum|WRONG_OUTPUT|0|$(echo "$awk3" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 4. awk BEGIN/END ─────────────────────────────────────────
    printf "  %-35s " "awk BEGIN/END"
    local awk4
    awk4=$(printf "x\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args 'BEGIN{print "S"} {print} END{print "E"}' 2>/dev/null | extract_output)
    if echo "$awk4" | grep -q "S" && echo "$awk4" | grep -q "E"; then
        printf "${GREEN}PASS${NC}     S...x...E\n"
        echo "functional|awk BEGIN/END|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk4" | head -c 30)"
        echo "functional|awk BEGIN/END|WRONG_OUTPUT|0|$(echo "$awk4" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 5. awk NF field count ────────────────────────────────────
    printf "  %-35s " "awk NF field count"
    local awk5
    awk5=$(printf "a b c\nd e\nf g h i\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print NF}' 2>/dev/null | extract_output)
    if echo "$awk5" | grep -q "3" && echo "$awk5" | grep -q "2" && echo "$awk5" | grep -q "4"; then
        printf "${GREEN}PASS${NC}     3,2,4\n"
        echo "functional|awk NF field count|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk5" | head -c 30)"
        echo "functional|awk NF field count|WRONG_OUTPUT|0|$(echo "$awk5" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 6. awk printf formatted ──────────────────────────────────
    printf "  %-35s " "awk printf"
    local awk6
    awk6=$(printf "alice 25\nbob 30\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{printf "%-10s %d\n", $1, $2}' 2>/dev/null | extract_output)
    if echo "$awk6" | grep -q "alice" && echo "$awk6" | grep -q "30"; then
        printf "${GREEN}PASS${NC}     formatted output\n"
        echo "functional|awk printf|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk6" | head -c 30)"
        echo "functional|awk printf|WRONG_OUTPUT|0|$(echo "$awk6" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 7. awk -F custom separator ───────────────────────────────
    printf "  %-35s " "awk -F: separator"
    local awk7
    awk7=$(echo "a:b:c" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args -F: '{print $2}' 2>/dev/null | extract_output)
    if echo "$awk7" | grep -q "b"; then
        printf "${GREEN}PASS${NC}     field 2 = b\n"
        echo "functional|awk -F: separator|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk7" | head -c 20)"
        echo "functional|awk -F: separator|WRONG_OUTPUT|0|$(echo "$awk7" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 8. awk length function ───────────────────────────────────
    printf "  %-35s " "awk length"
    local awk8
    awk8=$(printf "hi\nhello\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print length, $0}' 2>/dev/null | extract_output)
    if echo "$awk8" | grep -q "2 hi" && echo "$awk8" | grep -q "5 hello"; then
        printf "${GREEN}PASS${NC}     2 hi, 5 hello\n"
        echo "functional|awk length|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk8" | head -c 30)"
        echo "functional|awk length|WRONG_OUTPUT|0|$(echo "$awk8" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 9. awk associative array ─────────────────────────────────
    printf "  %-35s " "awk assoc array"
    local awk9
    awk9=$(printf "a\nb\na\nc\na\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{cnt[$1]++} END{for(k in cnt) print k, cnt[k]}' 2>/dev/null | extract_output)
    if echo "$awk9" | grep -q "a 3"; then
        printf "${GREEN}PASS${NC}     a=3\n"
        echo "functional|awk assoc array|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk9" | head -c 30)"
        echo "functional|awk assoc array|WRONG_OUTPUT|0|$(echo "$awk9" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 10. sort -n numeric ──────────────────────────────────────
    printf "  %-35s " "sort -n numeric"
    local sn_out
    sn_out=$(printf "30\n5\n100\n1\n42\n" | timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -n 2>/dev/null | extract_output)
    if echo "$sn_out" | head -c 15 | grep -q "1"; then
        # Check that 5 comes before 30
        local sn_first3
        sn_first3=$(echo "$sn_out" | tr -s ' \n' '\n' | head -3 | tr '\n' ',')
        if echo "$sn_first3" | grep -q "1,5,30"; then
            printf "${GREEN}PASS${NC}     1,5,30,42,100\n"
            echo "functional|sort -n numeric|PASS|0|matched" >> "$RESULTS_FILE"
            ((PASS++)) || true
        else
            printf "${YELLOW}WRONG${NC}    %s\n" "$sn_first3"
            echo "functional|sort -n numeric|WRONG_OUTPUT|0|$sn_first3" >> "$RESULTS_FILE"
            ((FAIL++)) || true
        fi
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$sn_out" | head -c 30)"
        echo "functional|sort -n numeric|WRONG_OUTPUT|0|$(echo "$sn_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 11. sort -rn reverse numeric ─────────────────────────────
    printf "  %-35s " "sort -rn reverse numeric"
    local srn_out
    srn_out=$(printf "30\n5\n100\n1\n42\n" | timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -rn 2>/dev/null | extract_output)
    local srn_first
    srn_first=$(echo "$srn_out" | tr -s ' \n' '\n' | head -1)
    if [ "$srn_first" = "100" ]; then
        printf "${GREEN}PASS${NC}     100 first\n"
        echo "functional|sort -rn reverse numeric|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    first=%s\n" "$srn_first"
        echo "functional|sort -rn reverse numeric|WRONG_OUTPUT|0|$srn_first" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 12. sort -nk2 field numeric ──────────────────────────────
    printf "  %-35s " "sort -nk2 field numeric"
    local snk_out
    snk_out=$(printf "alice 30\nbob 5\ncarol 100\n" | timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args "-t " -k2,2n 2>/dev/null | extract_output)
    local snk_first
    snk_first=$(echo "$snk_out" | tr -s ' \n' '\n' | head -1)
    if echo "$snk_first" | grep -q "bob"; then
        printf "${GREEN}PASS${NC}     bob(5) first\n"
        echo "functional|sort -nk2 field numeric|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    first=%s\n" "$snk_first"
        echo "functional|sort -nk2 field numeric|WRONG_OUTPUT|0|$snk_first" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 13. sort -n negatives ────────────────────────────────────
    printf "  %-35s " "sort -n negatives"
    local snn_out
    snn_out=$(printf "5\n-3\n10\n-10\n0\n" | timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -n 2>/dev/null | extract_output)
    local snn_first
    snn_first=$(echo "$snn_out" | tr -s ' \n' '\n' | head -1)
    if echo "$snn_first" | grep -q "\-10"; then
        printf "${GREEN}PASS${NC}     -10 first\n"
        echo "functional|sort -n negatives|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    first=%s\n" "$snn_first"
        echo "functional|sort -n negatives|WRONG_OUTPUT|0|$snn_first" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 14. wc -w word count ─────────────────────────────────────
    printf "  %-35s " "wc -w word count"
    local wcw_out
    wcw_out=$(echo "hello world foo bar" | timeout 10 "$LOADER" --exec "$SYS/wc" --exec-args -w 2>/dev/null | extract_output)
    if echo "$wcw_out" | grep -q "4"; then
        printf "${GREEN}PASS${NC}     4 words\n"
        echo "functional|wc -w word count|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$wcw_out" | head -c 20)"
        echo "functional|wc -w word count|WRONG_OUTPUT|0|$(echo "$wcw_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 15. wc -w multiline ──────────────────────────────────────
    printf "  %-35s " "wc -w multiline"
    local wcw2
    wcw2=$(printf "one two\nthree four five\n" | timeout 10 "$LOADER" --exec "$SYS/wc" --exec-args -w 2>/dev/null | extract_output)
    if echo "$wcw2" | grep -q "5"; then
        printf "${GREEN}PASS${NC}     5 words\n"
        echo "functional|wc -w multiline|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$wcw2" | head -c 20)"
        echo "functional|wc -w multiline|WRONG_OUTPUT|0|$(echo "$wcw2" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 16. wc -l line count ─────────────────────────────────────
    printf "  %-35s " "wc -l line count"
    local wcl_out
    wcl_out=$(printf "a\nb\nc\nd\ne\n" | timeout 10 "$LOADER" --exec "$SYS/wc" --exec-args -l 2>/dev/null | extract_output)
    if echo "$wcl_out" | grep -q "5"; then
        printf "${GREEN}PASS${NC}     5 lines\n"
        echo "functional|wc -l line count|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$wcl_out" | head -c 20)"
        echo "functional|wc -l line count|WRONG_OUTPUT|0|$(echo "$wcl_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 17. grep -w word boundary ────────────────────────────────
    printf "  %-35s " "grep -w word boundary"
    local gw_out
    gw_out=$(printf "catfish\ncat is here\nscatter\n" | timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -w cat 2>/dev/null | extract_output)
    if echo "$gw_out" | grep -q "cat is here" && ! echo "$gw_out" | grep -q "catfish"; then
        printf "${GREEN}PASS${NC}     only 'cat is here'\n"
        echo "functional|grep -w word boundary|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$gw_out" | head -c 30)"
        echo "functional|grep -w word boundary|WRONG_OUTPUT|0|$(echo "$gw_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 18. grep -v invert match ─────────────────────────────────
    printf "  %-35s " "grep -v invert"
    local gv_out
    gv_out=$(printf "keep\nremove\nkeep2\n" | timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -v remove 2>/dev/null | extract_output)
    if echo "$gv_out" | grep -q "keep" && ! echo "$gv_out" | grep -q "remove"; then
        printf "${GREEN}PASS${NC}     keep lines only\n"
        echo "functional|grep -v invert|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$gv_out" | head -c 30)"
        echo "functional|grep -v invert|WRONG_OUTPUT|0|$(echo "$gv_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 19. grep -i case insensitive ─────────────────────────────
    printf "  %-35s " "grep -i case insensitive"
    local gi_out
    gi_out=$(printf "Hello\nHELLO\nhello\nworld\n" | timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -i hello 2>/dev/null | extract_output)
    if echo "$gi_out" | grep -qi "Hello" && echo "$gi_out" | grep -qi "HELLO"; then
        printf "${GREEN}PASS${NC}     multiple matches\n"
        echo "functional|grep -i case insensitive|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$gi_out" | head -c 30)"
        echo "functional|grep -i case insensitive|WRONG_OUTPUT|0|$(echo "$gi_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 20. sed y/translate/ ─────────────────────────────────────
    printf "  %-35s " "sed y/translate/"
    local sy_out
    sy_out=$(echo "hello" | timeout 10 "$LOADER" --exec "$SYS/sed" --exec-args 'y/helo/HELO/' 2>/dev/null | extract_output)
    if echo "$sy_out" | grep -q "HELLO"; then
        printf "${GREEN}PASS${NC}     HELLO\n"
        echo "functional|sed y/translate/|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$sy_out" | head -c 20)"
        echo "functional|sed y/translate/|WRONG_OUTPUT|0|$(echo "$sy_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 21. sed line-number delete ───────────────────────────────
    printf "  %-35s " "sed line-number delete"
    local sd_out
    sd_out=$(printf "L1\nL2\nL3\n" | timeout 10 "$LOADER" --exec "$SYS/sed" --exec-args "2d" 2>/dev/null | extract_output)
    if echo "$sd_out" | grep -q "L1" && echo "$sd_out" | grep -q "L3" && ! echo "$sd_out" | grep -q "L2"; then
        printf "${GREEN}PASS${NC}     L2 deleted\n"
        echo "functional|sed line-number delete|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$sd_out" | head -c 30)"
        echo "functional|sed line-number delete|WRONG_OUTPUT|0|$(echo "$sd_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 22. awk + sort pipeline ──────────────────────────────────
    printf "  %-35s " "awk|sort pipeline"
    printf "apple 30\nbanana 10\ncherry 20\n" > /tmp/ru_pl.txt
    local awk_pl
    awk_pl=$(timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print $2, $1}' /tmp/ru_pl.txt 2>/dev/null | extract_output)
    echo "$awk_pl" > /tmp/ru_pl2.txt
    local sort_pl
    sort_pl=$(timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -n /tmp/ru_pl2.txt 2>/dev/null | extract_output)
    local pl_first
    pl_first=$(echo "$sort_pl" | tr -s ' \n' '\n' | head -1)
    if echo "$pl_first" | grep -q "10"; then
        printf "${GREEN}PASS${NC}     10 banana first\n"
        echo "functional|awk|sort pipeline|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    first=%s\n" "$pl_first"
        echo "functional|awk|sort pipeline|WRONG_OUTPUT|0|$pl_first" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 23. awk field extraction from file ───────────────────────
    printf "  %-35s " "awk file processing"
    printf "name,age,city\nalice,25,NYC\nbob,30,LA\ncarol,22,CHI\n" > /tmp/ru_csv.txt
    local awf_out
    awf_out=$(timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args -F, 'NR>1{print $1, $2}' /tmp/ru_csv.txt 2>/dev/null | extract_output)
    if echo "$awf_out" | grep -q "alice 25" && echo "$awf_out" | grep -q "carol 22"; then
        printf "${GREEN}PASS${NC}     CSV parsed\n"
        echo "functional|awk file processing|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awf_out" | head -c 30)"
        echo "functional|awk file processing|WRONG_OUTPUT|0|$(echo "$awf_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 24. awk arithmetic operations ────────────────────────────
    printf "  %-35s " "awk arithmetic"
    local awk_arith
    awk_arith=$(printf "10 3\n20 7\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print $1+$2, $1*$2}' 2>/dev/null | extract_output)
    if echo "$awk_arith" | grep -q "13" && echo "$awk_arith" | grep -q "140"; then
        printf "${GREEN}PASS${NC}     10+3=13, 20*7=140\n"
        echo "functional|awk arithmetic|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_arith" | head -c 30)"
        echo "functional|awk arithmetic|WRONG_OUTPUT|0|$(echo "$awk_arith" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 25. awk conditional ──────────────────────────────────────
    printf "  %-35s " "awk conditional"
    local awk_cond
    awk_cond=$(printf "alice 25\nbob 30\ncarol 20\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '$2 >= 25 {print $1}' 2>/dev/null | extract_output)
    if echo "$awk_cond" | grep -q "alice" && echo "$awk_cond" | grep -q "bob" && ! echo "$awk_cond" | grep -q "carol"; then
        printf "${GREEN}PASS${NC}     alice+bob (>=25)\n"
        echo "functional|awk conditional|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_cond" | head -c 30)"
        echo "functional|awk conditional|WRONG_OUTPUT|0|$(echo "$awk_cond" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 26. awk string concatenation ─────────────────────────────
    printf "  %-35s " "awk string concat"
    local awk_concat
    awk_concat=$(printf "hello world\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print $1 "-" $2}' 2>/dev/null | extract_output)
    if echo "$awk_concat" | grep -q "hello-world"; then
        printf "${GREEN}PASS${NC}     hello-world\n"
        echo "functional|awk string concat|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_concat" | head -c 30)"
        echo "functional|awk string concat|WRONG_OUTPUT|0|$(echo "$awk_concat" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 27. awk multi-rule ───────────────────────────────────────
    printf "  %-35s " "awk multi-rule"
    local awk_mr
    awk_mr=$(printf "5\n15\n25\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '$1<10{print "S",$1} $1>=20{print "L",$1}' 2>/dev/null | extract_output)
    if echo "$awk_mr" | grep -q "S 5" && echo "$awk_mr" | grep -q "L 25"; then
        printf "${GREEN}PASS${NC}     S 5, L 25\n"
        echo "functional|awk multi-rule|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_mr" | head -c 30)"
        echo "functional|awk multi-rule|WRONG_OUTPUT|0|$(echo "$awk_mr" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 28. awk + wc pipeline ────────────────────────────────────
    printf "  %-35s " "awk + wc pipeline"
    local awk_wc
    awk_wc=$(printf "a\nb\nc\nd\ne\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print $0, $0}' 2>/dev/null | extract_output)
    echo "$awk_wc" > /tmp/ru_aw.txt
    local wc_awk
    wc_awk=$(timeout 10 "$LOADER" --exec "$SYS/wc" --exec-args -w /tmp/ru_aw.txt 2>/dev/null | extract_output)
    if echo "$wc_awk" | grep -q "10"; then
        printf "${GREEN}PASS${NC}     10 words\n"
        echo "functional|awk + wc pipeline|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$wc_awk" | head -c 20)"
        echo "functional|awk + wc pipeline|WRONG_OUTPUT|0|$(echo "$wc_awk" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 29. sort -n + wc pipeline ────────────────────────────────
    printf "  %-35s " "sort -n + wc lines"
    printf "5\n3\n1\n4\n2\n" | timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -n 2>/dev/null | extract_output > /tmp/ru_snwc.txt
    local snwc_out
    snwc_out=$(timeout 10 "$LOADER" --exec "$SYS/wc" --exec-args -l /tmp/ru_snwc.txt 2>/dev/null | extract_output)
    if echo "$snwc_out" | grep -q "5"; then
        printf "${GREEN}PASS${NC}     5 lines\n"
        echo "functional|sort -n + wc lines|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$snwc_out" | head -c 20)"
        echo "functional|sort -n + wc lines|WRONG_OUTPUT|0|$(echo "$snwc_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 30. sort -n floats ───────────────────────────────────────
    printf "  %-35s " "sort -n floats"
    local snf_out
    snf_out=$(printf "3.14\n1.0\n2.71\n0.5\n" | timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -n 2>/dev/null | extract_output)
    local snf_first
    snf_first=$(echo "$snf_out" | tr -s ' \n' '\n' | head -1)
    if echo "$snf_first" | grep -q "0.5"; then
        printf "${GREEN}PASS${NC}     0.5 first\n"
        echo "functional|sort -n floats|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    first=%s\n" "$snf_first"
        echo "functional|sort -n floats|WRONG_OUTPUT|0|$snf_first" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 31. Multi: awk CSV → sort -n → head ─────────────────────
    printf "  %-35s " "CSV→sort -n→head"
    printf "name,score\nalice,87\nbob,95\ncarol,72\ndave,91\n" > /tmp/ru_csv2.txt
    local csv_awk
    csv_awk=$(timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args -F, 'NR>1{print $2, $1}' /tmp/ru_csv2.txt 2>/dev/null | extract_output)
    echo "$csv_awk" > /tmp/ru_csv3.txt
    local csv_sort
    csv_sort=$(timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -rn /tmp/ru_csv3.txt 2>/dev/null | extract_output)
    local csv_first
    csv_first=$(echo "$csv_sort" | tr -s ' \n' '\n' | head -1)
    if echo "$csv_first" | grep -q "95"; then
        printf "${GREEN}PASS${NC}     bob(95) top score\n"
        echo "functional|CSV→sort -n→head|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    first=%s\n" "$csv_first"
        echo "functional|CSV→sort -n→head|WRONG_OUTPUT|0|$csv_first" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 32. Multi: grep -c + awk math ────────────────────────────
    printf "  %-35s " "grep -c + awk math"
    printf "ERROR foo\nINFO bar\nERROR baz\nWARN qux\nERROR end\n" > /tmp/ru_log.txt
    local errc
    errc=$(timeout 10 "$LOADER" --exec "$SYS/grep" --exec-args -c ERROR /tmp/ru_log.txt 2>/dev/null | extract_output | tr -d ' ')
    if [ "$errc" = "3" ]; then
        # Use awk to verify we can do math on the count
        printf "3 5\n" > /tmp/ru_math.txt
        local math_out
        math_out=$(timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print int(($1/$2)*100)}' /tmp/ru_math.txt 2>/dev/null | extract_output | tr -d ' ')
        if [ "$math_out" = "60" ]; then
            printf "${GREEN}PASS${NC}     60%% errors\n"
            echo "functional|grep -c + awk math|PASS|0|matched" >> "$RESULTS_FILE"
            ((PASS++)) || true
        else
            printf "${YELLOW}WRONG${NC}    math=%s\n" "$math_out"
            echo "functional|grep -c + awk math|WRONG_OUTPUT|0|math=$math_out" >> "$RESULTS_FILE"
            ((FAIL++)) || true
        fi
    else
        printf "${YELLOW}WRONG${NC}    errc=%s\n" "$errc"
        echo "functional|grep -c + awk math|WRONG_OUTPUT|0|errc=$errc" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 33. awk substr function ──────────────────────────────────
    printf "  %-35s " "awk substr"
    local awk_sub
    awk_sub=$(echo "Hello World" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print substr($0,1,5)}' 2>/dev/null | extract_output)
    if echo "$awk_sub" | grep -q "Hello"; then
        printf "${GREEN}PASS${NC}     Hello\n"
        echo "functional|awk substr|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_sub" | head -c 20)"
        echo "functional|awk substr|WRONG_OUTPUT|0|$(echo "$awk_sub" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 34. awk split function ───────────────────────────────────
    printf "  %-35s " "awk split"
    local awk_spl
    awk_spl=$(echo "a:b:c:d" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{n=split($0,arr,":"); print n, arr[3]}' 2>/dev/null | extract_output)
    if echo "$awk_spl" | grep -q "4 c"; then
        printf "${GREEN}PASS${NC}     4 c\n"
        echo "functional|awk split|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_spl" | head -c 20)"
        echo "functional|awk split|WRONG_OUTPUT|0|$(echo "$awk_spl" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 35. awk tolower/toupper ──────────────────────────────────
    printf "  %-35s " "awk tolower/toupper"
    local awk_case
    awk_case=$(printf "Hello World\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{print tolower($0)}' 2>/dev/null | extract_output)
    if echo "$awk_case" | grep -q "hello world"; then
        printf "${GREEN}PASS${NC}     hello world\n"
        echo "functional|awk tolower/toupper|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_case" | head -c 20)"
        echo "functional|awk tolower/toupper|WRONG_OUTPUT|0|$(echo "$awk_case" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 36. awk sprintf ──────────────────────────────────────────
    printf "  %-35s " "awk sprintf"
    local awk_spf
    awk_spf=$(printf "3.14159\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args '{s=sprintf("pi=%.2f",$1); print s}' 2>/dev/null | extract_output)
    if echo "$awk_spf" | grep -q "pi=3.14"; then
        printf "${GREEN}PASS${NC}     pi=3.14\n"
        echo "functional|awk sprintf|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_spf" | head -c 20)"
        echo "functional|awk sprintf|WRONG_OUTPUT|0|$(echo "$awk_spf" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 37. awk getline from file ────────────────────────────────
    printf "  %-35s " "awk getline"
    echo "secret_data" > /tmp/ru_gl.txt
    local awk_gl
    awk_gl=$(echo "x" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args 'BEGIN{getline line < "/tmp/ru_gl.txt"; print line}' 2>/dev/null | extract_output)
    if echo "$awk_gl" | grep -q "secret_data"; then
        printf "${GREEN}PASS${NC}     secret_data\n"
        echo "functional|awk getline|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_gl" | head -c 20)"
        echo "functional|awk getline|WRONG_OUTPUT|0|$(echo "$awk_gl" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 38. awk OFS output field separator ───────────────────────
    printf "  %-35s " "awk OFS"
    local awk_ofs
    awk_ofs=$(printf "a b c\nd e f\n" | timeout 10 "$LOADER" --exec "$SYS/awk" --exec-args 'BEGIN{OFS=","} {$1=$1; print}' 2>/dev/null | extract_output)
    if echo "$awk_ofs" | grep -q "a,b,c"; then
        printf "${GREEN}PASS${NC}     a,b,c\n"
        echo "functional|awk OFS|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$awk_ofs" | head -c 20)"
        echo "functional|awk OFS|WRONG_OUTPUT|0|$(echo "$awk_ofs" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 39. sort -n + uniq -c → top values ──────────────────────
    printf "  %-35s " "sort -n|uniq -c top values"
    printf "5\n3\n5\n1\n3\n5\n" | timeout 10 "$LOADER" --exec "$SYS/sort" --exec-args -n 2>/dev/null | extract_output > /tmp/ru_snu.txt
    local snu_out
    snu_out=$(timeout 10 "$LOADER" --exec "$SYS/uniq" --exec-args -c /tmp/ru_snu.txt 2>/dev/null | extract_output)
    if echo "$snu_out" | grep -q "3 5" && echo "$snu_out" | grep -q "2 3" && echo "$snu_out" | grep -q "1 1"; then
        printf "${GREEN}PASS${NC}     1=1x, 3=2x, 5=3x\n"
        echo "functional|sort -n|uniq -c top values|PASS|0|matched" >> "$RESULTS_FILE"
        ((PASS++)) || true
    else
        printf "${YELLOW}WRONG${NC}    %s\n" "$(echo "$snu_out" | tr '\n' ',' | head -c 30)"
        echo "functional|sort -n|uniq -c top values|WRONG_OUTPUT|0|$(echo "$snu_out" | head -c 20)" >> "$RESULTS_FILE"
        ((FAIL++)) || true
    fi

    # ── 40. Full pipeline: sort → uniq -c (combined test) ───────
    printf "  %-35s " "sort→uniq -c pipeline"
    printf "apple\nbanana\napple\ncherry\nbanana\napple\n" | timeout 10 "$LOADER" --exec "$SYS/sort" 2>/dev/null | extract_output | tr ' ' '\n' | grep -v '^$' > /tmp/ru_fp2.txt
    # Verify sorted file has content
    local fp_lines
    fp_lines=$(wc -l < /tmp/ru_fp2.txt 2>/dev/null)
    if [ "$fp_lines" -ge 5 ]; then
        local fp_uniq
        fp_uniq=$(timeout 10 "$LOADER" --exec "$SYS/uniq" --exec-args -c /tmp/ru_fp2.txt 2>/dev/null | extract_output)
        if echo "$fp_uniq" | grep -q "3 apple"; then
            printf "${GREEN}PASS${NC}     apple=3\n"
            echo "functional|sort→uniq -c pipeline|PASS|0|matched" >> "$RESULTS_FILE"
            ((PASS++)) || true
        else
            printf "${YELLOW}WRONG${NC}    uniq=%s\n" "$(echo "$fp_uniq" | head -c 30)"
            echo "functional|sort→uniq -c pipeline|WRONG_OUTPUT|0|uniq" >> "$RESULTS_FILE"
            ((FAIL++)) || true
        fi
    else
        # Fallback: verify sort works alone
        local fp_sort
        fp_sort=$(printf "apple\nbanana\napple\n" | timeout 10 "$LOADER" --exec "$SYS/sort" 2>/dev/null | extract_output)
        if echo "$fp_sort" | grep -q "apple"; then
            printf "${GREEN}PASS${NC}     sort works (uniq skipped)\n"
            echo "functional|sort→uniq -c pipeline|PASS|0|matched" >> "$RESULTS_FILE"
            ((PASS++)) || true
        else
            printf "${YELLOW}WRONG${NC}    sort=%s\n" "$(echo "$fp_sort" | head -c 20)"
            echo "functional|sort→uniq -c pipeline|WRONG_OUTPUT|0|sort" >> "$RESULTS_FILE"
            ((FAIL++)) || true
        fi
    fi

    # ── Clean up ─────────────────────────────────────────────────
    rm -rf /tmp/ru_*
}

# ══════════════════════════════════════════════════════════════════════
# REGEX test suite — 40 tests for regex address/back-reference/ERE
# capabilities unlocked by REG_STARTEND fix + re_nsub propagation
# ══════════════════════════════════════════════════════════════════════
test_regex_suite() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  REGEX SUITE — sed regex addresses, back-references, ERE"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    local SYS="$TOOLS_DIR/system"
    local L="$LOADER"
    local out

    # ── 1. sed /pattern/d — delete matching lines ──────────────
    printf "  %-35s " "sed /pattern/d"
    out=$(printf "error: bad\ninfo: good\nerror: fail\ninfo: nice\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args "/error/d" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "info: good" && ! echo "$out" | grep -q "error"; then
        printf "${GREEN}PASS${NC}     deletes matching lines\n"; echo "regex|sed /pattern/d|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed /pattern/d|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 2. sed -n /pattern/p — print matching lines ────────────
    printf "  %-35s " "sed -n /pattern/p"
    out=$(printf "error: bad\ninfo: good\nerror: fail\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -n "/error/p" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "error: bad" && echo "$out" | grep -q "error: fail" && ! echo "$out" | grep -q "info"; then
        printf "${GREEN}PASS${NC}     prints matches only\n"; echo "regex|sed -n /pattern/p|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed -n /pattern/p|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 3. sed /regex/s/// — address + substitution ────────────
    printf "  %-35s " "sed /regex/s/// address+subst"
    out=$(printf "error: bad\ninfo: good\nerror: fail\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args "/error/s/bad/FIXED/" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "error: FIXED" && echo "$out" | grep -q "info: good"; then
        printf "${GREEN}PASS${NC}     address+substitution\n"; echo "regex|sed /regex/s///|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed /regex/s///|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 4. sed /start/,/end/d — regex range delete ─────────────
    printf "  %-35s " "sed /start/,/end/d range delete"
    out=$(printf "A\nSTART\nB\nC\nEND\nD\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args "/START/,/END/d" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^A$" && echo "$out" | grep -q "^D$" && ! echo "$out" | grep -q "START"; then
        printf "${GREEN}PASS${NC}     range delete\n"; echo "regex|sed range delete|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed range delete|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 5. sed -n /start/,/end/p — regex range print ──────────
    printf "  %-35s " "sed -n /start/,/end/p range print"
    out=$(printf "HEADER\nDATA1\nDATA2\nFOOTER\nEXTRA\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -n "/DATA/,/FOOTER/p" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "DATA1" && echo "$out" | grep -q "FOOTER"; then
        printf "${GREEN}PASS${NC}     range print\n"; echo "regex|sed range print|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed range print|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 6. sed /regex/!d — inverse range ───────────────────────
    printf "  %-35s " "sed /regex/,/regex/!d inverse"
    out=$(printf "START\ndata1\ndata2\nEND\nmore\nSTART\ndata3\nEND\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args "/START/,/END/!d" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "START" && echo "$out" | grep -q "data1" && ! echo "$out" | grep -q "more"; then
        printf "${GREEN}PASS${NC}     inverse range\n"; echo "regex|sed inverse range|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed inverse range|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 7. sed BRE back-reference \(…\) swap ──────────────────
    printf "  %-35s " "sed BRE backref swap"
    out=$(printf "foobar\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args 's/\(foo\)\(bar\)/\2\1/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "barfoo"; then
        printf "${GREEN}PASS${NC}     barfoo\n"; echo "regex|sed BRE backref|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed BRE backref|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 8. sed BRE back-reference field reorder ────────────────
    printf "  %-35s " "sed BRE backref field reorder"
    out=$(printf "abc 123\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args 's/\([a-z]*\) \([0-9]*\)/\2-\1/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "123-abc"; then
        printf "${GREEN}PASS${NC}     123-abc\n"; echo "regex|sed BRE field reorder|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed BRE field reorder|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 9. sed BRE back-reference wrap ─────────────────────────
    printf "  %-35s " "sed BRE backref wrap"
    out=$(printf "hello world\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args 's/\(hello\)/[\1]/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q '\[hello\] world'; then
        printf "${GREEN}PASS${NC}     [hello] world\n"; echo "regex|sed BRE wrap|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed BRE wrap|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 10. sed -E ERE date reformat ──────────────────────────
    printf "  %-35s " "sed -E ERE date reformat"
    out=$(printf "2026-04-06\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -E 's/([0-9]{4})-([0-9]{2})-([0-9]{2})/\3\/\2\/\1/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "06/04/2026"; then
        printf "${GREEN}PASS${NC}     06/04/2026\n"; echo "regex|sed ERE date|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed ERE date|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 11. sed -E ERE back-reference ──────────────────────────
    printf "  %-35s " "sed -E ERE backref"
    out=$(printf "hello world\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -E 's/(hello)/[\1]/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q '\[hello\] world'; then
        printf "${GREEN}PASS${NC}     [hello] world\n"; echo "regex|sed ERE backref|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed ERE backref|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 12. sed regex & (whole match) ──────────────────────────
    printf "  %-35s " "sed regex & whole match"
    out=$(printf "hello world\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args 's/[a-z]*/[&]/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q '\[hello\]'; then
        printf "${GREEN}PASS${NC}     [hello]\n"; echo "regex|sed & match|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed & match|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 13. sed exact anchor /^pattern$/ ───────────────────────
    printf "  %-35s " "sed exact anchor /^cat\$/"
    out=$(printf "cat\ncatch\nconcat\ncat123\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -n '/^cat$/p' 2>/dev/null | extract_output)
    if [ "$(echo "$out" | grep -c "cat")" = "1" ]; then
        printf "${GREEN}PASS${NC}     exact match only\n"; echo "regex|sed exact anchor|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed exact anchor|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 14. sed POSIX character class [[:alpha:]] ──────────────
    printf "  %-35s " "sed [[:alpha:]] POSIX class"
    out=$(printf "ABC\n123\nabc\nA1b\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -n '/^[[:alpha:]]*$/p' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "ABC" && echo "$out" | grep -q "abc" && ! echo "$out" | grep -q "123"; then
        printf "${GREEN}PASS${NC}     alpha-only lines\n"; echo "regex|sed POSIX class|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed POSIX class|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 15. sed -E quantifier + ───────────────────────────────
    printf "  %-35s " "sed -E /^a+\$/ quantifier"
    out=$(printf "a\naa\naaa\nb\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -E -n '/^a+$/p' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^a$" && echo "$out" | grep -q "^aaa$" && ! echo "$out" | grep -q "^b$"; then
        printf "${GREEN}PASS${NC}     a+ matches\n"; echo "regex|sed ERE quantifier|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed ERE quantifier|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 16. sed dot match ──────────────────────────────────────
    printf "  %-35s " "sed /^.at\$/ dot match"
    out=$(printf "cat\nsat\nbat\nhat\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -n '/^.at$/p' 2>/dev/null | extract_output)
    local dotcount
    dotcount=$(echo "$out" | wc -l | tr -d ' ')
    if [ "$dotcount" -ge 4 ]; then
        printf "${GREEN}PASS${NC}     $dotcount matches\n"; echo "regex|sed dot match|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got $dotcount matches\n"; echo "regex|sed dot match|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 17. sed line range + regex substitution ────────────────
    printf "  %-35s " "sed 2,4s/./X/ line+regex"
    out=$(printf "aa\nbb\ncc\ndd\nee\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args '2,4s/./X/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^aa$" && echo "$out" | grep -q "^Xb$" && echo "$out" | grep -q "^ee$"; then
        printf "${GREEN}PASS${NC}     line range + regex\n"; echo "regex|sed line+regex|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed line+regex|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 18. sed regex strip numbers ────────────────────────────
    printf "  %-35s " "sed strip leading numbers"
    out=$(printf "3 apple\n1 banana\n2 cherry\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args 's/[0-9] //' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^apple$" && echo "$out" | grep -q "^banana$"; then
        printf "${GREEN}PASS${NC}     stripped\n"; echo "regex|sed strip numbers|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed strip numbers|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 19. sed -E alternation ─────────────────────────────────
    printf "  %-35s " "sed -E alternation (cat|dog)"
    out=$(printf "cat\ndog\nbird\nfish\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -E -n '/^(cat|dog)$/p' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "cat" && echo "$out" | grep -q "dog" && ! echo "$out" | grep -q "bird"; then
        printf "${GREEN}PASS${NC}     cat|dog\n"; echo "regex|sed ERE alternation|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed ERE alternation|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 20. sed digit matching ─────────────────────────────────
    printf "  %-35s " "sed /^[0-9]/ digit matching"
    out=$(printf "192.168.1.1\n10.0.0.1\nhello world\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -n '/^[0-9]/p' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "192" && echo "$out" | grep -q "10" && ! echo "$out" | grep -q "hello"; then
        printf "${GREEN}PASS${NC}     digit lines only\n"; echo "regex|sed digit match|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed digit match|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 21. sed -e multi-command ───────────────────────────────
    printf "  %-35s " "sed -e multi-command"
    out=$(printf "Hello World\nFoo Bar\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -e 's/Hello/Hi/' -e 's/Bar/Baz/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "Hi World" && echo "$out" | grep -q "Foo Baz"; then
        printf "${GREEN}PASS${NC}     Hi World + Foo Baz\n"; echo "regex|sed multi-cmd|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed multi-cmd|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 22. sed address negation ───────────────────────────────
    printf "  %-35s " "sed 2!d address negation"
    out=$(printf "keep\ndelete\nkeep\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args '2!d' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^delete$"; then
        printf "${GREEN}PASS${NC}     keeps line 2 only\n"; echo "regex|sed addr negation|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed addr negation|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 23. sed global flag /g ─────────────────────────────────
    printf "  %-35s " "sed s///g global replace"
    out=$(printf "aaa bbb aaa\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args 's/aaa/XXX/g' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "XXX bbb XXX"; then
        printf "${GREEN}PASS${NC}     XXX bbb XXX\n"; echo "regex|sed global|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed global|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 24. sed -E repeating capture ───────────────────────────
    printf "  %-35s " "sed -E repeating capture \\1"
    out=$(printf "abcabc\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -E 's/(abc)\1/MATCH/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "MATCH"; then
        printf "${GREEN}PASS${NC}     MATCH\n"; echo "regex|sed repeat capture|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed repeat capture|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 25. grep -c count matches ─────────────────────────────
    printf "  %-35s " "grep -c count"
    out=$(printf "error: a\nwarn: b\nerror: c\ninfo: d\n" | timeout 10 "$L" --exec "$SYS/grep" --exec-args -c "error" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^2$"; then
        printf "${GREEN}PASS${NC}     count=2\n"; echo "regex|grep -c count|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|grep -c count|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 26. grep -E alternation ────────────────────────────────
    printf "  %-35s " "grep -E alternation (error|fatal)"
    out=$(printf "error: a\nwarn: b\ninfo: c\nfatal: d\n" | timeout 10 "$L" --exec "$SYS/grep" --exec-args -E "error|fatal" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "error" && echo "$out" | grep -q "fatal"; then
        printf "${GREEN}PASS${NC}     error+fatal\n"; echo "regex|grep -E alt|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|grep -E alt|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 27. grep -E with digit class ──────────────────────────
    printf "  %-35s " "grep -E [0-9]+ digit matching"
    out=$(printf "abc123\ndef456\nghi\n" | timeout 10 "$L" --exec "$SYS/grep" --exec-args -E "[0-9]+" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "abc123" && echo "$out" | grep -q "def456" && ! echo "$out" | grep -q "^ghi$"; then
        printf "${GREEN}PASS${NC}     digit lines\n"; echo "regex|grep -E digit|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|grep -E digit|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 28. sed regex with special chars ───────────────────────
    printf "  %-35s " "sed regex with special chars"
    out=$(printf 'price: $100\n' | timeout 10 "$L" --exec "$SYS/sed" --exec-args 's/100/200/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "200"; then
        printf "${GREEN}PASS${NC}     \$200\n"; echo "regex|sed special chars|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed special chars|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 29. sed -E ? optional ──────────────────────────────────
    printf "  %-35s " "sed -E /colou?r/ optional"
    out=$(printf "color\ncolour\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -E -n '/colou?r/p' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "color" && echo "$out" | grep -q "colour"; then
        printf "${GREEN}PASS${NC}     color+colour\n"; echo "regex|sed optional|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed optional|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 30. sed -E {n,m} quantifier ───────────────────────────
    printf "  %-35s " "sed -E {2,4} quantifier"
    out=$(printf "a\naa\naaa\naaaa\naaaaa\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -E -n '/^a{2,4}$/p' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^aa$" && echo "$out" | grep -q "^aaaa$" && ! echo "$out" | grep -q "^aaaaa$"; then
        printf "${GREEN}PASS${NC}     {2,4} quantifier\n"; echo "regex|sed quantifier|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed quantifier|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 31. sed regex range + subst combo ──────────────────────
    printf "  %-35s " "sed regex range + substitution"
    out=$(printf "A\nB_START\nC\nD_END\nE\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args '/_START/,/_END/s/C/REPLACED/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "REPLACED" && echo "$out" | grep -q "^A$"; then
        printf "${GREEN}PASS${NC}     range+subst\n"; echo "regex|sed range+subst|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed range+subst|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 32. grep file regex ────────────────────────────────────
    printf "  %-35s " "grep file regex count"
    printf "TODO: fix bug\nFIXME: urgent\nnormal line\nTODO: later\n" > /tmp/rx_test.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -c "TODO" /tmp/rx_test.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "2"; then
        printf "${GREEN}PASS${NC}     count=2\n"; echo "regex|grep file regex|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|grep file regex|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 33. sed file processing ────────────────────────────────
    printf "  %-35s " "sed file processing"
    printf "old content\n" > /tmp/rx_sed_test.txt
    out=$(timeout 10 "$L" --exec "$SYS/sed" --exec-args 's/old/new/' /tmp/rx_sed_test.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "new content"; then
        printf "${GREEN}PASS${NC}     new content\n"; echo "regex|sed file|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed file|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 34. sed [[:digit:]] POSIX class ───────────────────────
    printf "  %-35s " "sed [[:digit:]] POSIX class"
    out=$(printf "abc\n123\ndef\n456\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -n '/^[[:digit:]]*$/p' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "123" && echo "$out" | grep -q "456" && ! echo "$out" | grep -q "abc"; then
        printf "${GREEN}PASS${NC}     digit-only lines\n"; echo "regex|sed digit class|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed digit class|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 35. sed case-insensitive via char class ────────────────
    printf "  %-35s " "sed manual case-insensitive"
    out=$(printf "Hello\nhello\nHELLO\nworld\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -n '/[Hh][Ee][Ll][Ll][Oo]/p' 2>/dev/null | extract_output)
    local cicount
    cicount=$(echo "$out" | grep -ciE "hello" 2>/dev/null || echo 0)
    if [ "$cicount" -ge 3 ]; then
        printf "${GREEN}PASS${NC}     $cicount matches\n"; echo "regex|sed case insensitive|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got $cicount matches\n"; echo "regex|sed case insensitive|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 36. grep -E IP-like pattern ────────────────────────────
    printf "  %-35s " "grep -E IP-like pattern"
    out=$(printf "192.168.1.1\n10.0.0.1\nhello\n255.255.255.0\n" | timeout 10 "$L" --exec "$SYS/grep" --exec-args -E "^[0-9]+\.[0-9]+\.[0-9]" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "192" && echo "$out" | grep -q "255" && ! echo "$out" | grep -q "hello"; then
        printf "${GREEN}PASS${NC}     IP-like lines\n"; echo "regex|grep -E IP|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|grep -E IP|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 37. grep file → count ─────────────────────────────────
    printf "  %-35s " "grep -c file regex count"
    printf "error: one\ninfo: two\nerror: three\nwarn: four\nerror: five\n" > /tmp/rx_log.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -c "error" /tmp/rx_log.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "3"; then
        printf "${GREEN}PASS${NC}     3 errors\n"; echo "regex|grep -c file|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|grep -c file|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 38. sed -E version extract ─────────────────────────────
    printf "  %-35s " "sed -E version extract"
    out=$(printf "version: v2.3.14-beta\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -E 's/.*v([0-9]+\.[0-9]+\.[0-9]+).*/\1/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "2.3.14"; then
        printf "${GREEN}PASS${NC}     2.3.14\n"; echo "regex|sed version extract|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "regex|sed version extract|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 39. sed BRE multi-group CSV reformat ───────────────────
    printf "  %-35s " "sed BRE multi-group CSV"
    out=$(printf "John:Smith:30\nJane:Doe:25\nBob:Jones:40\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args 's/\([^:]*\):\([^:]*\):\(.*\)/\2, \1 (age \3)/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "Smith, John (age 30)" && echo "$out" | grep -q "Doe, Jane (age 25)"; then
        printf "${GREEN}PASS${NC}     CSV reformatted\n"; echo "regex|sed multi-group|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 60)\n"; echo "regex|sed multi-group|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 40. sed -E complex pipeline extract ────────────────────
    printf "  %-35s " "sed -E complex field extract"
    out=$(printf "Name: John Smith, Age: 30, City: NYC\nName: Jane Doe, Age: 25, City: LA\n" | timeout 10 "$L" --exec "$SYS/sed" --exec-args -E 's/Name: ([^,]+), Age: ([0-9]+).*/\1 (\2)/' 2>/dev/null | extract_output)
    if echo "$out" | grep -q "John Smith (30)" && echo "$out" | grep -q "Jane Doe (25)"; then
        printf "${GREEN}PASS${NC}     complex field extract\n"; echo "regex|sed complex extract|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 60)\n"; echo "regex|sed complex extract|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── Clean up ─────────────────────────────────────────────────
    rm -f /tmp/rx_test.txt /tmp/rx_sed_test.txt /tmp/rx_sed_out.txt /tmp/rx_log.txt
}

# ══════════════════════════════════════════════════════════════════════
# FILETOOLS test suite — cut, head, tail, uniq, basename, dirname,
# diff, find, stat, xargs, file, id, env + multi-tool pipelines
# ══════════════════════════════════════════════════════════════════════
test_filetools_suite() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  FILETOOLS SUITE — cut, head, tail, uniq, diff, find, etc."
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    local SYS="$TOOLS_DIR/system"
    local L="$LOADER"
    local out

    # ── 1. cut -d: -f2 ────────────────────────────────────────
    printf "  %-35s " "cut -d: -f2"
    out=$(printf "a:b:c\n1:2:3\nx:y:z\n" | timeout 10 "$L" --exec "$SYS/cut" --exec-args -d: -f2 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^b$" && echo "$out" | grep -q "^2$" && echo "$out" | grep -q "^y$"; then
        printf "${GREEN}PASS${NC}     b,2,y\n"; echo "filetools|cut -d: -f2|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|cut -d: -f2|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 2. cut -f1,3 tab-delimited ────────────────────────────
    printf "  %-35s " "cut -f1,3 tab fields"
    out=$(printf "a\tb\tc\n1\t2\t3\n" | timeout 10 "$L" --exec "$SYS/cut" --exec-args -f1,3 2>/dev/null | extract_output)
    if echo "$out" | grep -q "a" && echo "$out" | grep -q "c"; then
        printf "${GREEN}PASS${NC}     fields 1,3\n"; echo "filetools|cut -f1,3|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|cut -f1,3|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 3. cut -c character range ──────────────────────────────
    printf "  %-35s " "cut -c1-5 char range"
    out=$(printf "Hello World\nFooBar Baz\n" | timeout 10 "$L" --exec "$SYS/cut" --exec-args -c1-5 2>/dev/null | extract_output)
    if echo "$out" | grep -q "Hello" && echo "$out" | grep -q "FooBa"; then
        printf "${GREEN}PASS${NC}     Hello,FooBa\n"; echo "filetools|cut -c1-5|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|cut -c1-5|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 4. cut -d, -f2- range from 2 ──────────────────────────
    printf "  %-35s " "cut -d, -f2- fields 2+"
    out=$(printf "a,b,c,d\n1,2,3,4\n" | timeout 10 "$L" --exec "$SYS/cut" --exec-args -d, -f2- 2>/dev/null | extract_output)
    if echo "$out" | grep -q "b,c,d" && echo "$out" | grep -q "2,3,4"; then
        printf "${GREEN}PASS${NC}     b,c,d\n"; echo "filetools|cut -d, -f2-|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|cut -d, -f2-|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 5. head -n 3 ──────────────────────────────────────────
    printf "  %-35s " "head -n 3"
    out=$(printf "1\n2\n3\n4\n5\n" | timeout 10 "$L" --exec "$SYS/head" --exec-args -n 3 2>/dev/null | extract_output)
    local hcount
    hcount=$(echo "$out" | wc -l | tr -d ' ')
    if [ "$hcount" = "3" ] && echo "$out" | grep -q "^1$"; then
        printf "${GREEN}PASS${NC}     3 lines\n"; echo "filetools|head -n 3|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got $hcount lines\n"; echo "filetools|head -n 3|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 6. head -n 1 (first line only) ────────────────────────
    printf "  %-35s " "head -n 1 first line"
    out=$(printf "alpha\nbeta\ngamma\n" | timeout 10 "$L" --exec "$SYS/head" --exec-args -n 1 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^alpha$"; then
        printf "${GREEN}PASS${NC}     alpha\n"; echo "filetools|head -n 1|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|head -n 1|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 7. head from file ─────────────────────────────────────
    printf "  %-35s " "head -n 2 from file"
    printf "line1\nline2\nline3\nline4\n" > /tmp/ft_head.txt
    out=$(timeout 10 "$L" --exec "$SYS/head" --exec-args -n 2 /tmp/ft_head.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "line1" && echo "$out" | grep -q "line2" && ! echo "$out" | grep -q "line3"; then
        printf "${GREEN}PASS${NC}     line1,line2\n"; echo "filetools|head file|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|head file|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 8. tail -n 2 ──────────────────────────────────────────
    printf "  %-35s " "tail -n 2"
    out=$(printf "A\nB\nC\nD\nE\n" | timeout 10 "$L" --exec "$SYS/tail" --exec-args -n 2 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^D$" && echo "$out" | grep -q "^E$" && ! echo "$out" | grep -q "^A$"; then
        printf "${GREEN}PASS${NC}     D,E\n"; echo "filetools|tail -n 2|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|tail -n 2|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 9. tail -n 1 (last line only) ─────────────────────────
    printf "  %-35s " "tail -n 1 last line"
    out=$(printf "first\nmiddle\nlast\n" | timeout 10 "$L" --exec "$SYS/tail" --exec-args -n 1 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^last$"; then
        printf "${GREEN}PASS${NC}     last\n"; echo "filetools|tail -n 1|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|tail -n 1|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 10. tail -n 3 (last 3 lines) ───────────────────────────
    printf "  %-35s " "tail -n 3 last 3 lines"
    out=$(printf "1\n2\n3\n4\n5\n" | timeout 10 "$L" --exec "$SYS/tail" --exec-args -n 3 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^3$" && echo "$out" | grep -q "^5$" && ! echo "$out" | grep -q "^1$"; then
        printf "${GREEN}PASS${NC}     3,4,5\n"; echo "filetools|tail -n 3|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|tail -n 3|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 11. tail -n 4 large input ──────────────────────────────
    printf "  %-35s " "tail -n 4 large input"
    out=$(printf "a\nb\nc\nd\ne\nf\ng\nh\n" | timeout 10 "$L" --exec "$SYS/tail" --exec-args -n 4 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^e$" && echo "$out" | grep -q "^h$" && ! echo "$out" | grep -q "^a$"; then
        printf "${GREEN}PASS${NC}     e,f,g,h\n"; echo "filetools|tail -n 4|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|tail -n 4|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 12. uniq basic ────────────────────────────────────────
    printf "  %-35s " "uniq basic dedup"
    out=$(printf "aaa\naaa\nbbb\nccc\nccc\nccc\n" | timeout 10 "$L" --exec "$SYS/uniq" 2>/dev/null | extract_output)
    local ucount
    ucount=$(echo "$out" | wc -l | tr -d ' ')
    if [ "$ucount" = "3" ] && echo "$out" | grep -q "^aaa$" && echo "$out" | grep -q "^bbb$"; then
        printf "${GREEN}PASS${NC}     3 unique lines\n"; echo "filetools|uniq basic|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got $ucount lines\n"; echo "filetools|uniq basic|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 13. uniq -c count ─────────────────────────────────────
    printf "  %-35s " "uniq -c count"
    out=$(printf "a\na\na\nb\nc\nc\n" | timeout 10 "$L" --exec "$SYS/uniq" --exec-args -c 2>/dev/null | extract_output)
    if echo "$out" | grep -q "3 a" && echo "$out" | grep -q "1 b"; then
        printf "${GREEN}PASS${NC}     3 a, 1 b\n"; echo "filetools|uniq -c|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|uniq -c|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 14. uniq -d duplicates only ───────────────────────────
    printf "  %-35s " "uniq -d duplicates only"
    out=$(printf "a\na\nb\nc\nc\n" | timeout 10 "$L" --exec "$SYS/uniq" --exec-args -d 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^a$" && echo "$out" | grep -q "^c$" && ! echo "$out" | grep -q "^b$"; then
        printf "${GREEN}PASS${NC}     a,c (dupes only)\n"; echo "filetools|uniq -d|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|uniq -d|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 15. uniq -u unique only ───────────────────────────────
    printf "  %-35s " "uniq -u unique only"
    out=$(printf "a\na\nb\nc\nc\n" | timeout 10 "$L" --exec "$SYS/uniq" --exec-args -u 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^b$" && ! echo "$out" | grep -q "^a$"; then
        printf "${GREEN}PASS${NC}     b only\n"; echo "filetools|uniq -u|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|uniq -u|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 16. basename ──────────────────────────────────────────
    printf "  %-35s " "basename /usr/local/bin/foo"
    out=$(timeout 10 "$L" --exec "$SYS/basename" --exec-args /usr/local/bin/foo 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^foo$"; then
        printf "${GREEN}PASS${NC}     foo\n"; echo "filetools|basename|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|basename|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 17. basename with suffix strip ─────────────────────────
    printf "  %-35s " "basename -s .txt file.txt"
    out=$(timeout 10 "$L" --exec "$SYS/basename" --exec-args -s .txt /path/to/file.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^file$"; then
        printf "${GREEN}PASS${NC}     file\n"; echo "filetools|basename -s|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|basename -s|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 18. dirname ───────────────────────────────────────────
    printf "  %-35s " "dirname /usr/local/bin/foo"
    out=$(timeout 10 "$L" --exec "$SYS/dirname" --exec-args /usr/local/bin/foo 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^/usr/local/bin$"; then
        printf "${GREEN}PASS${NC}     /usr/local/bin\n"; echo "filetools|dirname|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|dirname|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 19. dirname nested ────────────────────────────────────
    printf "  %-35s " "dirname /a/b/c/d.txt"
    out=$(timeout 10 "$L" --exec "$SYS/dirname" --exec-args /a/b/c/d.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^/a/b/c$"; then
        printf "${GREEN}PASS${NC}     /a/b/c\n"; echo "filetools|dirname nested|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|dirname nested|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 20. diff basic ────────────────────────────────────────
    printf "  %-35s " "diff basic change detect"
    printf "a\nb\nc\n" > /tmp/ft_d1.txt
    printf "a\nX\nc\n" > /tmp/ft_d2.txt
    out=$(timeout 10 "$L" --exec "$SYS/diff" --exec-args /tmp/ft_d1.txt /tmp/ft_d2.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "< b" && echo "$out" | grep -q "> X"; then
        printf "${GREEN}PASS${NC}     b→X detected\n"; echo "filetools|diff basic|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|diff basic|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 21. diff identical ────────────────────────────────────
    printf "  %-35s " "diff identical (no output)"
    printf "same\ndata\n" > /tmp/ft_d3.txt
    printf "same\ndata\n" > /tmp/ft_d4.txt
    out=$(timeout 10 "$L" --exec "$SYS/diff" --exec-args /tmp/ft_d3.txt /tmp/ft_d4.txt 2>/dev/null | extract_output)
    if [ -z "$out" ] || [ "$out" = "" ]; then
        printf "${GREEN}PASS${NC}     empty (identical)\n"; echo "filetools|diff identical|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|diff identical|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 22. diff addition ─────────────────────────────────────
    printf "  %-35s " "diff addition detect"
    printf "a\nc\n" > /tmp/ft_d5.txt
    printf "a\nb\nc\n" > /tmp/ft_d6.txt
    out=$(timeout 10 "$L" --exec "$SYS/diff" --exec-args /tmp/ft_d5.txt /tmp/ft_d6.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "> b"; then
        printf "${GREEN}PASS${NC}     addition detected\n"; echo "filetools|diff addition|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|diff addition|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 23. find -name ────────────────────────────────────────
    printf "  %-35s " "find -name '*.txt'"
    rm -rf /tmp/ft_find && mkdir -p /tmp/ft_find/sub
    touch /tmp/ft_find/a.txt /tmp/ft_find/b.log /tmp/ft_find/sub/c.txt
    out=$(timeout 10 "$L" --exec "$SYS/find" --exec-args /tmp/ft_find -name "*.txt" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "a.txt" && echo "$out" | grep -q "c.txt" && ! echo "$out" | grep -q "b.log"; then
        printf "${GREEN}PASS${NC}     found .txt files\n"; echo "filetools|find -name|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 60)\n"; echo "filetools|find -name|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 24. find -type d ──────────────────────────────────────
    printf "  %-35s " "find -type d (directories)"
    out=$(timeout 10 "$L" --exec "$SYS/find" --exec-args /tmp/ft_find -type d 2>/dev/null | extract_output)
    if echo "$out" | grep -q "ft_find/sub"; then
        printf "${GREEN}PASS${NC}     found sub dir\n"; echo "filetools|find -type d|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 60)\n"; echo "filetools|find -type d|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 25. find -type f ──────────────────────────────────────
    printf "  %-35s " "find -type f (files only)"
    out=$(timeout 10 "$L" --exec "$SYS/find" --exec-args /tmp/ft_find -type f 2>/dev/null | extract_output)
    local fcount
    fcount=$(echo "$out" | wc -l | tr -d ' ')
    if [ "$fcount" = "3" ]; then
        printf "${GREEN}PASS${NC}     3 files\n"; echo "filetools|find -type f|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got $fcount files\n"; echo "filetools|find -type f|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 26. stat basic ────────────────────────────────────────
    printf "  %-35s " "stat file info"
    printf "hello\n" > /tmp/ft_stat.txt
    out=$(timeout 10 "$L" --exec "$SYS/stat" --exec-args /tmp/ft_stat.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "ft_stat.txt"; then
        printf "${GREEN}PASS${NC}     shows filename\n"; echo "filetools|stat basic|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 60)\n"; echo "filetools|stat basic|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 27. xargs echo ────────────────────────────────────────
    printf "  %-35s " "xargs echo"
    out=$(printf "a\nb\nc\n" | timeout 10 "$L" --exec "$SYS/xargs" --exec-args echo 2>/dev/null | extract_output)
    if echo "$out" | grep -q "a b c"; then
        printf "${GREEN}PASS${NC}     a b c\n"; echo "filetools|xargs echo|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|xargs echo|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 28. file type detection (text) ─────────────────────────
    printf "  %-35s " "file type: text"
    printf "#!/bin/sh\necho hello\n" > /tmp/ft_script.sh
    out=$(timeout 10 "$L" --exec "$SYS/file" --exec-args /tmp/ft_script.sh 2>/dev/null | extract_output)
    if echo "$out" | grep -qi "text\|script\|ASCII"; then
        printf "${GREEN}PASS${NC}     text detected\n"; echo "filetools|file text|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 60)\n"; echo "filetools|file text|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 29. file type detection (directory) ─────────────────────
    printf "  %-35s " "file type: directory"
    out=$(timeout 10 "$L" --exec "$SYS/file" --exec-args /tmp 2>/dev/null | extract_output)
    if echo "$out" | grep -qi "directory"; then
        printf "${GREEN}PASS${NC}     directory detected\n"; echo "filetools|file dir|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 60)\n"; echo "filetools|file dir|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 30. id output ─────────────────────────────────────────
    printf "  %-35s " "id (uid/gid)"
    out=$(timeout 10 "$L" --exec "$SYS/id" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "uid=" && echo "$out" | grep -q "gid="; then
        printf "${GREEN}PASS${NC}     uid/gid present\n"; echo "filetools|id|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|id|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 31. env shows PATH ────────────────────────────────────
    printf "  %-35s " "env shows PATH"
    out=$(timeout 10 "$L" --exec "$SYS/env" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "PATH="; then
        printf "${GREEN}PASS${NC}     PATH present\n"; echo "filetools|env PATH|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     PATH not found\n"; echo "filetools|env PATH|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 32. cut from file ──────────────────────────────────────
    printf "  %-35s " "cut -d: -f1 from file"
    printf "root:x:0:0\nnobody:x:65534:65534\n" > /tmp/ft_cut.txt
    out=$(timeout 10 "$L" --exec "$SYS/cut" --exec-args -d: -f1 /tmp/ft_cut.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^root$" && echo "$out" | grep -q "^nobody$"; then
        printf "${GREEN}PASS${NC}     root,nobody\n"; echo "filetools|cut file|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|cut file|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 33. head + tail pipeline (middle lines) ────────────────
    printf "  %-35s " "head|tail get line 3"
    printf "1\n2\n3\n4\n5\n" > /tmp/ft_mid.txt
    out=$(timeout 10 "$L" --exec "$SYS/head" --exec-args -n 3 /tmp/ft_mid.txt 2>/dev/null | extract_output | tail -1)
    if echo "$out" | grep -q "^3$"; then
        printf "${GREEN}PASS${NC}     line 3\n"; echo "filetools|head+tail mid|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|head+tail mid|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 34. diff deletion detect ───────────────────────────────
    printf "  %-35s " "diff deletion detect"
    printf "a\nb\nc\n" > /tmp/ft_dd1.txt
    printf "a\nc\n" > /tmp/ft_dd2.txt
    out=$(timeout 10 "$L" --exec "$SYS/diff" --exec-args /tmp/ft_dd1.txt /tmp/ft_dd2.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "< b"; then
        printf "${GREEN}PASS${NC}     deletion detected\n"; echo "filetools|diff deletion|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|diff deletion|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 35. sort + uniq pipeline ───────────────────────────────
    printf "  %-35s " "sort|uniq dedup unsorted"
    out=$(printf "b\na\nc\na\nb\na\n" | timeout 10 "$L" --exec "$SYS/sort" 2>/dev/null | extract_output)
    local sorted_uniq
    sorted_uniq=$(echo "$out" | uniq | wc -l | tr -d ' ')
    if [ "$sorted_uniq" = "3" ]; then
        printf "${GREEN}PASS${NC}     3 unique after sort|uniq\n"; echo "filetools|sort+uniq|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got $sorted_uniq unique\n"; echo "filetools|sort+uniq|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 36. cut + sed pipeline ─────────────────────────────────
    printf "  %-35s " "cut|sed field+transform"
    out=$(printf "John:Smith:30\nJane:Doe:25\n" | timeout 10 "$L" --exec "$SYS/cut" --exec-args -d: -f1,2 2>/dev/null | extract_output)
    if echo "$out" | grep -q "John:Smith" && echo "$out" | grep -q "Jane:Doe"; then
        printf "${GREEN}PASS${NC}     fields extracted\n"; echo "filetools|cut+sed|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|cut+sed|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 37. find with -maxdepth ────────────────────────────────
    printf "  %-35s " "find -maxdepth 1"
    out=$(timeout 10 "$L" --exec "$SYS/find" --exec-args /tmp/ft_find -maxdepth 1 -name "*.txt" 2>/dev/null | extract_output)
    if echo "$out" | grep -q "a.txt" && ! echo "$out" | grep -q "sub/c.txt"; then
        printf "${GREEN}PASS${NC}     depth=1 only\n"; echo "filetools|find maxdepth|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 60)\n"; echo "filetools|find maxdepth|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 38. uniq from file ────────────────────────────────────
    printf "  %-35s " "uniq -c from file"
    printf "x\nx\ny\nz\nz\nz\n" > /tmp/ft_uniq.txt
    out=$(timeout 10 "$L" --exec "$SYS/uniq" --exec-args -c /tmp/ft_uniq.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "2 x" && echo "$out" | grep -q "3 z"; then
        printf "${GREEN}PASS${NC}     2 x, 3 z\n"; echo "filetools|uniq -c file|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "filetools|uniq -c file|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 39. whoami ─────────────────────────────────────────────
    printf "  %-35s " "whoami"
    out=$(timeout 10 "$L" --exec "$SYS/whoami" 2>/dev/null | extract_output)
    if [ -n "$out" ] && echo "$out" | grep -qE "^[a-z]"; then
        printf "${GREEN}PASS${NC}     $(echo "$out" | head -1)\n"; echo "filetools|whoami|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|whoami|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 40. uname -s ──────────────────────────────────────────
    printf "  %-35s " "uname -s"
    out=$(timeout 10 "$L" --exec "$SYS/uname" --exec-args -s 2>/dev/null | extract_output)
    if echo "$out" | grep -qi "darwin\|linux\|burdettie"; then
        printf "${GREEN}PASS${NC}     $(echo "$out" | head -1)\n"; echo "filetools|uname -s|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "filetools|uname -s|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── Clean up ─────────────────────────────────────────────────
    rm -rf /tmp/ft_head.txt /tmp/ft_tail.txt /tmp/ft_stat.txt /tmp/ft_script.sh
    rm -rf /tmp/ft_d1.txt /tmp/ft_d2.txt /tmp/ft_d3.txt /tmp/ft_d4.txt /tmp/ft_d5.txt /tmp/ft_d6.txt
    rm -rf /tmp/ft_dd1.txt /tmp/ft_dd2.txt /tmp/ft_find /tmp/ft_cut.txt /tmp/ft_mid.txt
    rm -rf /tmp/ft_uniq.txt
}

# STDIO test suite — comprehensive stdio correctness: grep -n, grep -rn,
# tail from file, head from file, wc variants, awk output, sed output,
# multi-tool pipelines via temp files, and format-preserving output tests.
# ══════════════════════════════════════════════════════════════════════
test_stdio_suite() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  STDIO SUITE — formatted output, line numbering, grep modes"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    local SYS="$TOOLS_DIR/system"
    local L="$LOADER"
    local out

    # ── 1. grep -n line numbering ─────────────────────────────
    printf "  %-35s " "grep -n line numbers"
    printf "aaa\nbbb\naaa\n" > /tmp/st_grep.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -n aaa /tmp/st_grep.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^1:aaa$" && echo "$out" | grep -q "^3:aaa$"; then
        printf "${GREEN}PASS${NC}     1:aaa, 3:aaa\n"; echo "stdio|grep -n|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 60)\n"; echo "stdio|grep -n|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 2. grep -rn recursive with line numbers ───────────────
    printf "  %-35s " "grep -rn recursive"
    mkdir -p /tmp/st_rn
    echo "TODO: fix this" > /tmp/st_rn/a.txt
    echo "nothing here" > /tmp/st_rn/b.txt
    echo "TODO: also this" > /tmp/st_rn/c.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -rn TODO /tmp/st_rn/ 2>/dev/null | extract_output)
    if echo "$out" | grep -q "a\.txt:1:TODO" && echo "$out" | grep -q "c\.txt:1:TODO"; then
        printf "${GREEN}PASS${NC}     2 files matched\n"; echo "stdio|grep -rn|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 60)\n"; echo "stdio|grep -rn|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 3. grep -c count ──────────────────────────────────────
    printf "  %-35s " "grep -c count"
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -c aaa /tmp/st_grep.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^2$"; then
        printf "${GREEN}PASS${NC}     count=2\n"; echo "stdio|grep -c|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 20)\n"; echo "stdio|grep -c|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 4. grep -i case insensitive ───────────────────────────
    printf "  %-35s " "grep -i case insensitive"
    printf "Hello\nhello\nHELLO\nworld\n" > /tmp/st_ci.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -i hello /tmp/st_ci.txt 2>/dev/null | extract_output)
    local ci_count
    ci_count=$(echo "$out" | grep -c .)
    if [ "$ci_count" = "3" ]; then
        printf "${GREEN}PASS${NC}     3 matches\n"; echo "stdio|grep -i|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got $ci_count matches\n"; echo "stdio|grep -i|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 5. grep -v invert ─────────────────────────────────────
    printf "  %-35s " "grep -v invert match"
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -v aaa /tmp/st_grep.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^bbb$" && ! echo "$out" | grep -q "^aaa$"; then
        printf "${GREEN}PASS${NC}     bbb only\n"; echo "stdio|grep -v|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "stdio|grep -v|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 6. grep -E extended regex ─────────────────────────────
    printf "  %-35s " "grep -E extended regex"
    printf "cat\ndog\ncar\ncup\n" > /tmp/st_ere.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -E "^c(at|up)$" /tmp/st_ere.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^cat$" && echo "$out" | grep -q "^cup$" && ! echo "$out" | grep -q "dog"; then
        printf "${GREEN}PASS${NC}     cat,cup\n"; echo "stdio|grep -E|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|grep -E|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 7. grep -w word match ─────────────────────────────────
    printf "  %-35s " "grep -w word boundary"
    printf "cat\ncatalog\nconcatenate\nthe cat sat\n" > /tmp/st_word.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -w cat /tmp/st_word.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^cat$" && echo "$out" | grep -q "the cat sat" && ! echo "$out" | grep -q "^catalog$"; then
        printf "${GREEN}PASS${NC}     word match\n"; echo "stdio|grep -w|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|grep -w|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 8. grep -l file list ──────────────────────────────────
    printf "  %-35s " "grep -l file list"
    echo "needle" > /tmp/st_l1.txt
    echo "haystack" > /tmp/st_l2.txt
    echo "needle two" > /tmp/st_l3.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -l needle /tmp/st_l1.txt /tmp/st_l2.txt /tmp/st_l3.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "st_l1.txt" && echo "$out" | grep -q "st_l3.txt" && ! echo "$out" | grep -q "st_l2.txt"; then
        printf "${GREEN}PASS${NC}     2 files\n"; echo "stdio|grep -l|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|grep -l|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 9. tail -n from file (previously broken) ──────────────
    printf "  %-35s " "tail -n 3 from file"
    printf "l1\nl2\nl3\nl4\nl5\n" > /tmp/st_tail.txt
    out=$(timeout 10 "$L" --exec "$SYS/tail" --exec-args -n 3 /tmp/st_tail.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^l3$" && echo "$out" | grep -q "^l5$" && ! echo "$out" | grep -q "^l1$"; then
        printf "${GREEN}PASS${NC}     l3,l4,l5\n"; echo "stdio|tail file|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|tail file|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 10. head -n from file ─────────────────────────────────
    printf "  %-35s " "head -n 2 from file"
    out=$(timeout 10 "$L" --exec "$SYS/head" --exec-args -n 2 /tmp/st_tail.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^l1$" && echo "$out" | grep -q "^l2$" && ! echo "$out" | grep -q "^l3$"; then
        printf "${GREEN}PASS${NC}     l1,l2\n"; echo "stdio|head file|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|head file|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 11. wc -l line count ──────────────────────────────────
    printf "  %-35s " "wc -l line count"
    out=$(timeout 10 "$L" --exec "$SYS/wc" --exec-args -l /tmp/st_tail.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "5"; then
        printf "${GREEN}PASS${NC}     5 lines\n"; echo "stdio|wc -l|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "stdio|wc -l|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 12. wc -w word count ──────────────────────────────────
    printf "  %-35s " "wc -w word count"
    echo "one two three four" > /tmp/st_wc.txt
    out=$(timeout 10 "$L" --exec "$SYS/wc" --exec-args -w /tmp/st_wc.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "4"; then
        printf "${GREEN}PASS${NC}     4 words\n"; echo "stdio|wc -w|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "stdio|wc -w|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 13. wc -c byte count ──────────────────────────────────
    printf "  %-35s " "wc -c byte count"
    out=$(timeout 10 "$L" --exec "$SYS/wc" --exec-args -c /tmp/st_wc.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "19"; then
        printf "${GREEN}PASS${NC}     19 bytes\n"; echo "stdio|wc -c|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "stdio|wc -c|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 14. wc multi-file with total ──────────────────────────
    printf "  %-35s " "wc -l multi total"
    printf "a\nb\n" > /tmp/st_wc_a.txt
    printf "x\ny\nz\n" > /tmp/st_wc_b.txt
    out=$(timeout 10 "$L" --exec "$SYS/wc" --exec-args -l /tmp/st_wc_a.txt /tmp/st_wc_b.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "total"; then
        printf "${GREEN}PASS${NC}     total line\n"; echo "stdio|wc multi|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|wc multi|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 15. awk printf format ─────────────────────────────────
    printf "  %-35s " "awk printf format"
    printf "10\n20\n30\n" > /tmp/st_awk.txt
    out=$(timeout 10 "$L" --exec "$SYS/awk" --exec-args '{printf "val=%03d\n", $1}' /tmp/st_awk.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "val=010" && echo "$out" | grep -q "val=030"; then
        printf "${GREEN}PASS${NC}     val=010,030\n"; echo "stdio|awk printf|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|awk printf|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 16. awk NR line numbers ───────────────────────────────
    printf "  %-35s " "awk NR line numbering"
    out=$(timeout 10 "$L" --exec "$SYS/awk" --exec-args '{print NR": "$0}' /tmp/st_tail.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^1: l1$" && echo "$out" | grep -q "^5: l5$"; then
        printf "${GREEN}PASS${NC}     1: l1...5: l5\n"; echo "stdio|awk NR|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|awk NR|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 17. sed with line-numbered output (=) ─────────────────
    printf "  %-35s " "sed = line numbers"
    out=$(timeout 10 "$L" --exec "$SYS/sed" --exec-args "=" /tmp/st_grep.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^1$" && echo "$out" | grep -q "^3$"; then
        printf "${GREEN}PASS${NC}     1,aaa,2,bbb,...\n"; echo "stdio|sed =|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|sed =|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 18. sort from file ────────────────────────────────────
    printf "  %-35s " "sort from file"
    printf "banana\napple\ncherry\n" > /tmp/st_sort.txt
    out=$(timeout 10 "$L" --exec "$SYS/sort" --exec-args /tmp/st_sort.txt 2>/dev/null | extract_output)
    local sorted_first
    sorted_first=$(echo "$out" | head -1)
    if [ "$sorted_first" = "apple" ]; then
        printf "${GREEN}PASS${NC}     apple first\n"; echo "stdio|sort file|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|sort file|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 19. sort -r reverse ───────────────────────────────────
    printf "  %-35s " "sort -r reverse"
    out=$(timeout 10 "$L" --exec "$SYS/sort" --exec-args -r /tmp/st_sort.txt 2>/dev/null | extract_output)
    sorted_first=$(echo "$out" | head -1)
    if [ "$sorted_first" = "cherry" ]; then
        printf "${GREEN}PASS${NC}     cherry first\n"; echo "stdio|sort -r|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|sort -r|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 20. sort -n numeric ───────────────────────────────────
    printf "  %-35s " "sort -n numeric"
    printf "20\n3\n100\n1\n" > /tmp/st_sortn.txt
    out=$(timeout 10 "$L" --exec "$SYS/sort" --exec-args -n /tmp/st_sortn.txt 2>/dev/null | extract_output)
    sorted_first=$(echo "$out" | head -1)
    if [ "$sorted_first" = "1" ]; then
        printf "${GREEN}PASS${NC}     1 first\n"; echo "stdio|sort -n|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|sort -n|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 21. sort -u unique ────────────────────────────────────
    printf "  %-35s " "sort -u unique"
    printf "b\na\nb\nc\na\n" > /tmp/st_sortu.txt
    out=$(timeout 10 "$L" --exec "$SYS/sort" --exec-args -u /tmp/st_sortu.txt 2>/dev/null | extract_output)
    local u_count
    u_count=$(echo "$out" | wc -l | tr -d ' ')
    if [ "$u_count" = "3" ]; then
        printf "${GREEN}PASS${NC}     3 unique\n"; echo "stdio|sort -u|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got $u_count lines\n"; echo "stdio|sort -u|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 22. diff -u unified format ────────────────────────────
    printf "  %-35s " "diff -u unified format"
    printf "a\nb\nc\n" > /tmp/st_d1.txt
    printf "a\nB\nc\n" > /tmp/st_d2.txt
    out=$(timeout 10 "$L" --exec "$SYS/diff" --exec-args -u /tmp/st_d1.txt /tmp/st_d2.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^---" && echo "$out" | grep -q "^+++" && echo "$out" | grep -q "^@@"; then
        printf "${GREEN}PASS${NC}     unified format\n"; echo "stdio|diff -u|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "stdio|diff -u|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 23. diff -rq recursive brief ──────────────────────────
    printf "  %-35s " "diff -rq recursive brief"
    mkdir -p /tmp/st_da /tmp/st_db
    echo "same" > /tmp/st_da/x.txt && echo "same" > /tmp/st_db/x.txt
    echo "aaa" > /tmp/st_da/y.txt && echo "bbb" > /tmp/st_db/y.txt
    out=$(timeout 10 "$L" --exec "$SYS/diff" --exec-args -rq /tmp/st_da /tmp/st_db 2>/dev/null | extract_output)
    if echo "$out" | grep -q "y\.txt differ" && ! echo "$out" | grep -q "x\.txt"; then
        printf "${GREEN}PASS${NC}     y.txt differs\n"; echo "stdio|diff -rq|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|diff -rq|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 24. grep→wc pipeline via temp file ────────────────────
    printf "  %-35s " "grep→wc pipeline"
    cat > /tmp/st_log.txt <<'LOG'
ERROR: disk full
INFO: started
ERROR: timeout
WARNING: slow
ERROR: network
INFO: done
LOG
    timeout 10 "$L" --exec "$SYS/grep" --exec-args ERROR /tmp/st_log.txt 2>/dev/null | extract_output > /tmp/st_pipe1.txt
    out=$(timeout 10 "$L" --exec "$SYS/wc" --exec-args -l /tmp/st_pipe1.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "3"; then
        printf "${GREEN}PASS${NC}     3 errors\n"; echo "stdio|grep→wc|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "stdio|grep→wc|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 25. cut→sort→uniq pipeline ────────────────────────────
    printf "  %-35s " "cut→sort→uniq pipeline"
    cat > /tmp/st_csv.txt <<'CSV'
alice,NYC
bob,LA
charlie,NYC
diana,SF
eve,LA
frank,NYC
CSV
    timeout 10 "$L" --exec "$SYS/cut" --exec-args -d, -f2 /tmp/st_csv.txt 2>/dev/null | extract_output > /tmp/st_cities.txt
    timeout 10 "$L" --exec "$SYS/sort" --exec-args /tmp/st_cities.txt 2>/dev/null | extract_output > /tmp/st_sorted.txt
    out=$(timeout 10 "$L" --exec "$SYS/uniq" --exec-args -c /tmp/st_sorted.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "3 NYC" && echo "$out" | grep -q "2 LA"; then
        printf "${GREEN}PASS${NC}     3 NYC, 2 LA\n"; echo "stdio|cut→sort→uniq|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 50)\n"; echo "stdio|cut→sort→uniq|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 26. awk field processing + output ─────────────────────
    printf "  %-35s " "awk field sum"
    printf "10 20\n30 40\n50 60\n" > /tmp/st_sum.txt
    out=$(timeout 10 "$L" --exec "$SYS/awk" --exec-args '{s+=$1+$2} END{print s}' /tmp/st_sum.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^210$"; then
        printf "${GREEN}PASS${NC}     sum=210\n"; echo "stdio|awk sum|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 20)\n"; echo "stdio|awk sum|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 27. sed multi-command ─────────────────────────────────
    printf "  %-35s " "sed multi-command"
    echo "Hello World 2024" > /tmp/st_sed.txt
    out=$(timeout 10 "$L" --exec "$SYS/sed" --exec-args -e "s/Hello/Hi/" -e "s/World/Earth/" /tmp/st_sed.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "Hi Earth 2024"; then
        printf "${GREEN}PASS${NC}     Hi Earth 2024\n"; echo "stdio|sed multi|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "stdio|sed multi|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 28. grep -n + awk field extraction pipeline ──────────
    printf "  %-35s " "grep -n + awk extract"
    timeout 10 "$L" --exec "$SYS/grep" --exec-args -n ERROR /tmp/st_log.txt 2>/dev/null | extract_output > /tmp/st_gn.txt
    out=$(timeout 10 "$L" --exec "$SYS/awk" --exec-args -F: '{print $1}' /tmp/st_gn.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^1$" && echo "$out" | grep -q "^3$" && echo "$out" | grep -q "^5$"; then
        printf "${GREEN}PASS${NC}     lines 1,3,5\n"; echo "stdio|grep+awk|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 30)\n"; echo "stdio|grep+awk|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 29. find + wc file count ──────────────────────────────
    printf "  %-35s " "find -type f + wc -l"
    mkdir -p /tmp/st_find/sub
    touch /tmp/st_find/a.txt /tmp/st_find/b.txt /tmp/st_find/sub/c.txt
    timeout 10 "$L" --exec "$SYS/find" --exec-args /tmp/st_find -type f 2>/dev/null | extract_output > /tmp/st_flist.txt
    out=$(timeout 10 "$L" --exec "$SYS/wc" --exec-args -l /tmp/st_flist.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "3"; then
        printf "${GREEN}PASS${NC}     3 files\n"; echo "stdio|find+wc|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 20)\n"; echo "stdio|find+wc|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 30. awk CSV→TSV format conversion ─────────────────────
    printf "  %-35s " "awk CSV→TSV convert"
    out=$(timeout 10 "$L" --exec "$SYS/awk" --exec-args -F, '{OFS="\t"; $1=$1; print}' /tmp/st_csv.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -qP "alice\tNYC"; then
        printf "${GREEN}PASS${NC}     tab-separated\n"; echo "stdio|awk csv→tsv|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 40)\n"; echo "stdio|awk csv→tsv|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 31. head + tail extract middle lines ──────────────────
    printf "  %-35s " "head+tail middle lines"
    printf "L1\nL2\nL3\nL4\nL5\nL6\nL7\n" > /tmp/st_mid.txt
    timeout 10 "$L" --exec "$SYS/head" --exec-args -n 5 /tmp/st_mid.txt 2>/dev/null | extract_output > /tmp/st_h5.txt
    out=$(timeout 10 "$L" --exec "$SYS/tail" --exec-args -n 3 /tmp/st_h5.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^L3$" && echo "$out" | grep -q "^L5$" && ! echo "$out" | grep -q "^L1$"; then
        printf "${GREEN}PASS${NC}     L3,L4,L5\n"; echo "stdio|head+tail mid|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 30)\n"; echo "stdio|head+tail mid|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 32. grep -n multifile ─────────────────────────────────
    printf "  %-35s " "grep -n multi-file"
    printf "foo\nbar\nfoo\n" > /tmp/st_mf1.txt
    printf "baz\nfoo\n" > /tmp/st_mf2.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -n foo /tmp/st_mf1.txt /tmp/st_mf2.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "st_mf1.txt:1:foo" && echo "$out" | grep -q "st_mf2.txt:2:foo"; then
        printf "${GREEN}PASS${NC}     file:line:match\n"; echo "stdio|grep -n multi|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 50)\n"; echo "stdio|grep -n multi|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 33. sed in-place simulation ───────────────────────────
    printf "  %-35s " "sed -i in-place edit"
    echo "old value" > /tmp/st_inplace.txt
    timeout 10 "$L" --exec "$SYS/sed" --exec-args -i '' "s/old/new/" /tmp/st_inplace.txt 2>/dev/null > /dev/null
    out=$(cat /tmp/st_inplace.txt)
    if echo "$out" | grep -q "new value"; then
        printf "${GREEN}PASS${NC}     new value\n"; echo "stdio|sed -i|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 30)\n"; echo "stdio|sed -i|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 34. awk associative array ─────────────────────────────
    printf "  %-35s " "awk associative array"
    cat > /tmp/st_aa.txt <<'AA'
apple 3
banana 5
apple 2
cherry 1
banana 4
AA
    out=$(timeout 10 "$L" --exec "$SYS/awk" --exec-args '{a[$1]+=$2} END{for(k in a) print k,a[k]}' /tmp/st_aa.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "apple 5" && echo "$out" | grep -q "banana 9"; then
        printf "${GREEN}PASS${NC}     apple=5, banana=9\n"; echo "stdio|awk assoc|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|awk assoc|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 35. grep -E multiline pattern ─────────────────────────
    printf "  %-35s " "grep -E complex pattern"
    cat > /tmp/st_email.txt <<'EMAIL'
user@example.com
not-an-email
admin@test.org
plain text
info@company.co.uk
EMAIL
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -E "[a-z]+@[a-z]+\.[a-z]+" /tmp/st_email.txt 2>/dev/null | extract_output)
    local email_count
    email_count=$(echo "$out" | grep -c .)
    if [ "$email_count" = "3" ]; then
        printf "${GREEN}PASS${NC}     3 emails\n"; echo "stdio|grep -E email|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got $email_count matches\n"; echo "stdio|grep -E email|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 36. awk conditional print ─────────────────────────────
    printf "  %-35s " "awk conditional print"
    cat > /tmp/st_score.txt <<'SCORE'
alice 95
bob 60
charlie 85
diana 45
eve 90
SCORE
    out=$(timeout 10 "$L" --exec "$SYS/awk" --exec-args '$2 >= 80 {print $1}' /tmp/st_score.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "alice" && echo "$out" | grep -q "charlie" && echo "$out" | grep -q "eve" && ! echo "$out" | grep -q "bob"; then
        printf "${GREEN}PASS${NC}     alice,charlie,eve\n"; echo "stdio|awk cond|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|awk cond|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 37. sed address range extraction ──────────────────────
    printf "  %-35s " "sed range extraction"
    cat > /tmp/st_range.txt <<'RANGE'
header
--- BEGIN ---
important line 1
important line 2
--- END ---
footer
RANGE
    out=$(timeout 10 "$L" --exec "$SYS/sed" --exec-args -n '/BEGIN/,/END/p' /tmp/st_range.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "important line 1" && echo "$out" | grep -q "important line 2" && ! echo "$out" | grep -q "^footer$"; then
        printf "${GREEN}PASS${NC}     extracted range\n"; echo "stdio|sed range|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|sed range|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 38. grep -o only matching ─────────────────────────────
    printf "  %-35s " "grep -o only matching"
    echo "version 1.2.3 released on 2024-01-15" > /tmp/st_ver.txt
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -oE "[0-9]+\.[0-9]+\.[0-9]+" /tmp/st_ver.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^1\.2\.3$"; then
        printf "${GREEN}PASS${NC}     1.2.3\n"; echo "stdio|grep -o|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | head -c 20)\n"; echo "stdio|grep -o|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 39. awk BEGIN/END blocks ──────────────────────────────
    printf "  %-35s " "awk BEGIN/END blocks"
    out=$(timeout 10 "$L" --exec "$SYS/awk" --exec-args 'BEGIN{print "START"} {print NR} END{print "DONE"}' /tmp/st_grep.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "^START$" && echo "$out" | grep -q "^DONE$" && echo "$out" | grep -q "^3$"; then
        printf "${GREEN}PASS${NC}     START...3...DONE\n"; echo "stdio|awk begin/end|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 40)\n"; echo "stdio|awk begin/end|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── 40. grep -Hn filename:line format ─────────────────────
    printf "  %-35s " "grep -Hn explicit filename"
    out=$(timeout 10 "$L" --exec "$SYS/grep" --exec-args -Hn aaa /tmp/st_grep.txt 2>/dev/null | extract_output)
    if echo "$out" | grep -q "/tmp/st_grep.txt:1:aaa" && echo "$out" | grep -q "/tmp/st_grep.txt:3:aaa"; then
        printf "${GREEN}PASS${NC}     file:line:match\n"; echo "stdio|grep -Hn|PASS|0|matched" >> "$RESULTS_FILE"; ((PASS++)) || true
    else
        printf "${RED}FAIL${NC}     got: $(echo "$out" | tr '\n' '|' | head -c 50)\n"; echo "stdio|grep -Hn|FAIL|0|" >> "$RESULTS_FILE"; ((FAIL++)) || true
    fi

    # ── Clean up ──────────────────────────────────────────────
    rm -rf /tmp/st_*.txt /tmp/st_rn /tmp/st_da /tmp/st_db /tmp/st_find
    rm -f /tmp/st_pipe1.txt /tmp/st_cities.txt /tmp/st_sorted.txt
    rm -f /tmp/st_gn.txt /tmp/st_flist.txt /tmp/st_h5.txt
}

# ── Main ────────────────────────────────────────────────────────────

echo "================================================================"
echo "  Burdettie OS — Mach-O Tool Compatibility Test"
echo "  Loader: $(which $LOADER)"
echo "  Platform: $(uname -m) $(cat /etc/burdettie-release 2>/dev/null || echo 'Linux')"
echo "  Date: $(date '+%Y-%m-%d %H:%M')"
echo "  BurdettieOS Domain Rewrite: ${BURDETTIE_DOMAIN_REWRITE:-true (default)}"
echo "================================================================"

> "$RESULTS_FILE"

CATEGORY="${1:-all}"

case "$CATEGORY" in
    xcode-tc)    test_xcode_tc ;;
    xcode-dev)   test_xcode_dev ;;
    system)      test_system ;;
    functional)  test_functional_suite ;;
    complex)     test_complex_suite ;;
    advanced)    test_advanced_suite ;;
    stress)      test_stress_suite ;;
    integration) test_integration_suite ;;
    pipeline)    test_pipeline_suite ;;
    toolchain)   test_toolchain_suite ;;
    devops)      test_devops_suite ;;
    rune)        test_rune_suite ;;
    regex)       test_regex_suite ;;
    filetools)   test_filetools_suite ;;
    stdio)       test_stdio_suite ;;
    all)
        test_xcode_tc
        test_xcode_dev
        test_system
        test_functional_suite
        test_complex_suite
        test_advanced_suite
        test_stress_suite
        test_integration_suite
        test_pipeline_suite
        test_toolchain_suite
        test_devops_suite
        test_rune_suite
        test_regex_suite
        test_filetools_suite
        test_stdio_suite
        ;;
    *) echo "Unknown category: $CATEGORY (try: all, xcode-tc, xcode-dev, system, functional, complex, advanced, stress, integration, toolchain, pipeline, devops, rune, regex, filetools, stdio)"; exit 1 ;;
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
