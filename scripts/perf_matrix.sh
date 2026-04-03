#!/usr/bin/env bash
# scripts/perf_matrix.sh — Run the performance matrix benchmark suite.
#
# Usage:
#   ./scripts/perf_matrix.sh                  # default (3 iterations, all tools)
#   ./scripts/perf_matrix.sh --json           # JSON output for CI
#   ./scripts/perf_matrix.sh --filter cache   # benchmark only tools matching "cache"
#   ./scripts/perf_matrix.sh --iterations 10  # more iterations for stable results
#
# Prerequisites:
#   - corpus/build_corpus.sh has been run (corpus/<tool>/app binaries exist)
#   - release build recommended for accurate timings

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

# Check corpus exists
CORPUS_COUNT=$(find corpus -name app -type f 2>/dev/null | wc -l | tr -d ' ')
if [ "$CORPUS_COUNT" -eq 0 ]; then
    echo "No corpus binaries found. Building corpus..."
    bash corpus/build_corpus.sh
    CORPUS_COUNT=$(find corpus -name app -type f 2>/dev/null | wc -l | tr -d ' ')
    if [ "$CORPUS_COUNT" -eq 0 ]; then
        echo "ERROR: corpus/build_corpus.sh did not produce binaries."
        exit 1
    fi
fi

echo "Found $CORPUS_COUNT corpus tools."
echo "Building perf-bench (release)..."

cargo build -p perf-bench --release 2>&1 | tail -3

echo ""
echo "Running performance matrix..."
echo "════════════════════════════════════════════════════════════════"

# Pass all CLI args through
cargo run -p perf-bench --release -- "$@"

EXIT=$?

# Save results if JSON mode was used
if [[ "$*" == *"--json"* ]]; then
    OUTPUT_FILE="$ROOT_DIR/target/perf-matrix-$(date +%Y%m%d-%H%M%S).json"
    cargo run -p perf-bench --release -- "$@" > "$OUTPUT_FILE" 2>/dev/null
    echo ""
    echo "JSON results saved to: $OUTPUT_FILE"
fi

exit $EXIT
