#!/usr/bin/env bash
# run_comparison.sh — Build fixtures, run Rust tools, collect host data, and compare.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$SCRIPT_DIR/.."

echo "=== Step 1: Build fixtures ==="
"$SCRIPT_DIR/build_fixtures.sh"

echo ""
echo "=== Step 2: Build Rust workspace ==="
cd "$ROOT"
cargo build --workspace

echo ""
echo "=== Step 3: Inspect fixtures ==="
mkdir -p observed

for fixture in simple_cli dylib_chain constructor_test; do
    binary="fixtures/${fixture}/app"
    if [ -f "$binary" ]; then
        echo ""
        echo "--- ${fixture} ---"
        cargo run -q -p macho-inspect -- "$binary"
        cargo run -q -p macho-inspect -- --json "$binary" > "observed/${fixture}_inspect.json" 2>/dev/null || true
    fi
done

# Inspect dylibs too
for dylib in fixtures/dylib_chain/libchaina.dylib fixtures/dylib_chain/libchainb.dylib; do
    if [ -f "$dylib" ]; then
        echo ""
        echo "--- $(basename "$dylib") ---"
        cargo run -q -p macho-inspect -- "$dylib"
    fi
done

echo ""
echo "=== Step 4: Collect host observations ==="
for fixture in simple_cli dylib_chain constructor_test objc_foundation; do
    binary="fixtures/${fixture}/app"
    if [ -f "$binary" ]; then
        "$SCRIPT_DIR/collect_host_info.sh" "$binary" observed
    fi
done

echo ""
echo "=== Step 5: Validate against host ==="
echo "Comparing our parser output against otool..."

ERRORS=0
for fixture in simple_cli dylib_chain constructor_test; do
    binary="fixtures/${fixture}/app"
    if [ ! -f "$binary" ]; then continue; fi

    echo ""
    echo "--- Validating: ${fixture} ---"

    # Compare segment count
    OUR_SEGS=$(cargo run -q -p macho-inspect -- --json "$binary" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['segments']))" 2>/dev/null || echo "?")
    HOST_SEGS=$(otool -l "$binary" 2>/dev/null | grep -c "cmd LC_SEGMENT_64" || echo "0")
    if [ "$OUR_SEGS" = "$HOST_SEGS" ]; then
        echo "  ✓ Segment count matches: $OUR_SEGS"
    else
        echo "  ✗ Segment count mismatch: ours=$OUR_SEGS host=$HOST_SEGS"
        ERRORS=$((ERRORS + 1))
    fi

    # Compare dylib count
    OUR_DYLIBS=$(cargo run -q -p macho-inspect -- --json "$binary" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['dylibs']))" 2>/dev/null || echo "?")
    HOST_DYLIBS=$(otool -L "$binary" 2>/dev/null | tail -n +2 | wc -l | tr -d ' ')
    if [ "$OUR_DYLIBS" = "$HOST_DYLIBS" ]; then
        echo "  ✓ Dylib count matches: $OUR_DYLIBS"
    else
        echo "  ✗ Dylib count mismatch: ours=$OUR_DYLIBS host=$HOST_DYLIBS"
        ERRORS=$((ERRORS + 1))
    fi

    # Compare LC count
    OUR_LC=$(cargo run -q -p macho-inspect -- --json "$binary" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['commands']))" 2>/dev/null || echo "?")
    HOST_LC=$(otool -l "$binary" 2>/dev/null | grep -c "^Load command" || echo "0")
    if [ "$OUR_LC" = "$HOST_LC" ]; then
        echo "  ✓ Load command count matches: $OUR_LC"
    else
        echo "  ✗ Load command count mismatch: ours=$OUR_LC host=$HOST_LC"
        ERRORS=$((ERRORS + 1))
    fi
done

echo ""
if [ "$ERRORS" -eq 0 ]; then
    echo "=== All validations passed ✓ ==="
else
    echo "=== $ERRORS validation(s) failed ✗ ==="
fi
