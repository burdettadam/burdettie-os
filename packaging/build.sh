#!/usr/bin/env bash
# build.sh — Full build + test + package pipeline for burdetty-os.
#
# Usage:
#   ./packaging/build.sh              # default: debug build + tests
#   ./packaging/build.sh --release    # release build + tests
#   ./packaging/build.sh --skip-tests # build only, no tests
#
# Exit codes:
#   0 — all steps passed
#   1 — build failed
#   2 — clippy/fmt check failed
#   3 — tests failed
#   4 — fixture build failed
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$SCRIPT_DIR/.."
cd "$ROOT"

# ── Parse arguments ─────────────────────────────────────────────────
PROFILE="dev"
CARGO_PROFILE_FLAG=""
SKIP_TESTS=false

for arg in "$@"; do
    case "$arg" in
        --release)
            PROFILE="release"
            CARGO_PROFILE_FLAG="--release"
            ;;
        --skip-tests)
            SKIP_TESTS=true
            ;;
        --help|-h)
            head -14 "$0" | tail -13
            exit 0
            ;;
    esac
done

echo "╔══════════════════════════════════════════════════════╗"
echo "║  burdetty-os — build pipeline                 ║"
echo "║  profile: $PROFILE                                      ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# ── Step 1: Format check ───────────────────────────────────────────
echo "▶ Step 1/6: Checking formatting..."
if ! cargo fmt --all -- --check 2>/dev/null; then
    echo "✗ Formatting check failed. Run 'cargo fmt' to fix."
    exit 2
fi
echo "✓ Formatting OK"
echo ""

# ── Step 2: Clippy ─────────────────────────────────────────────────
echo "▶ Step 2/6: Running clippy..."
if ! cargo clippy --workspace --all-targets $CARGO_PROFILE_FLAG -- -D warnings 2>&1; then
    echo "✗ Clippy found warnings/errors."
    exit 2
fi
echo "✓ Clippy OK (zero warnings)"
echo ""

# ── Step 3: Build ──────────────────────────────────────────────────
echo "▶ Step 3/6: Building workspace..."
if ! cargo build --workspace --all-targets $CARGO_PROFILE_FLAG 2>&1; then
    echo "✗ Build failed."
    exit 1
fi
echo "✓ Build OK"
echo ""

# ── Step 4: Build fixtures ─────────────────────────────────────────
echo "▶ Step 4/6: Building test fixtures..."
if [ -x "$ROOT/scripts/build_fixtures.sh" ]; then
    if ! "$ROOT/scripts/build_fixtures.sh" 2>&1; then
        echo "✗ Fixture build failed."
        exit 4
    fi
    echo "✓ Fixtures OK"
else
    echo "⚠ scripts/build_fixtures.sh not found or not executable, skipping"
fi
echo ""

# ── Step 5: Tests ──────────────────────────────────────────────────
if [ "$SKIP_TESTS" = true ]; then
    echo "▶ Step 5/6: Tests SKIPPED (--skip-tests)"
else
    echo "▶ Step 5/6: Running tests..."
    if ! cargo test --workspace $CARGO_PROFILE_FLAG 2>&1; then
        echo "✗ Tests failed."
        exit 3
    fi
    echo "✓ Tests OK"
fi
echo ""

# ── Step 6: Summary ───────────────────────────────────────────────
echo "▶ Step 6/6: Collecting summary..."
CRATE_COUNT=$(cargo metadata --no-deps --format-version=1 2>/dev/null | python3 -c "import sys,json; print(len(json.load(sys.stdin)['packages']))" 2>/dev/null || echo "?")
FIXTURE_COUNT=$(ls -d "$ROOT/fixtures"/*/app 2>/dev/null | wc -l | tr -d ' ')
TEST_COUNT=$(cargo test --workspace $CARGO_PROFILE_FLAG 2>&1 | grep "^test result:" | awk '{sum += $4} END {print sum}' 2>/dev/null || echo "?")

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║  BUILD COMPLETE                                     ║"
echo "║                                                     ║"
printf "║  Crates:   %-40s║\n" "$CRATE_COUNT"
printf "║  Fixtures: %-40s║\n" "$FIXTURE_COUNT"
printf "║  Tests:    %-40s║\n" "$TEST_COUNT passed"
printf "║  Profile:  %-40s║\n" "$PROFILE"
echo "║                                                     ║"
echo "╚══════════════════════════════════════════════════════╝"
