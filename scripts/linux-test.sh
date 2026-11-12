#!/usr/bin/env bash
# scripts/linux-test.sh — Fast cross-compile + test loop for Linux ARM64.
#
# Instead of rebuilding the Docker image on every change (minutes),
# this cross-compiles macho-inspect on the host (seconds) and
# volume-mounts it into the existing burdettie-os runtime container.
#
# Prerequisites (one-time setup):
#   rustup target add aarch64-unknown-linux-gnu
#   brew install zig
#   cargo install cargo-zigbuild
#   docker build --platform linux/arm64 -f docker/Dockerfile.burdettie -t burdettie-os .
#
# Usage:
#   ./scripts/linux-test.sh              # build + run full test suite
#   ./scripts/linux-test.sh stat         # build + test a single tool
#   ./scripts/linux-test.sh stat file    # build + test multiple tools
#   ./scripts/linux-test.sh --skip-build # skip build, just run tests
#   ./scripts/linux-test.sh --shell      # build + drop into interactive shell

set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

TARGET="aarch64-unknown-linux-gnu"
# Use glibc 2.36 to match Debian bookworm
GLIBC="2.36"
CARGO_ARTIFACT_DIR=$(cargo metadata --locked --no-deps --format-version=1 \
  | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
BINARY="${CARGO_ARTIFACT_DIR}/${TARGET}/release/macho-inspect"
IMAGE="burdettie-os"
SKIP_BUILD=false
SHELL_MODE=false
TOOLS=()

for arg in "$@"; do
  case "$arg" in
    --skip-build) SKIP_BUILD=true ;;
    --shell)      SHELL_MODE=true ;;
    *)            TOOLS+=("$arg") ;;
  esac
done

# ── Cross-compile ──────────────────────────────────────────────────
if [ "$SKIP_BUILD" = false ]; then
  echo "▸ Cross-compiling macho-inspect for ${TARGET}..."
  time cargo zigbuild --locked --release --target "${TARGET}.${GLIBC}" -p macho-inspect 2>&1 \
    | grep -v "^$" | tail -3
  echo "▸ Binary: $(ls -lh "$BINARY" | awk '{print $5}')"
fi

if [ ! -f "$BINARY" ]; then
  echo "ERROR: Binary not found at $BINARY"
  echo "Run without --skip-build first."
  exit 1
fi

# ── Docker run with volume mount ───────────────────────────────────
DOCKER_ARGS=(
  --rm
  --platform linux/arm64
  -v "${BINARY}:/usr/local/bin/macho-inspect:ro"
  "$IMAGE"
)

if [ "$SHELL_MODE" = true ]; then
  echo "▸ Launching interactive shell..."
  docker run -it "${DOCKER_ARGS[@]}" bash
  exit 0
fi

if [ ${#TOOLS[@]} -eq 0 ]; then
  # Full test suite
  echo "▸ Running full test suite..."
  docker run "${DOCKER_ARGS[@]}" bash -c '
    OUTPUT=$(test-xcode-tools 2>&1)
    echo "$OUTPUT" | grep -E "PASS|CRASH|FAIL|MISSING" | grep -v "exit="
    echo
    echo "$OUTPUT" | grep "RESULTS SUMMARY" -A10
  '
else
  # Test specific tools
  for tool in "${TOOLS[@]}"; do
    echo "▸ Testing: $tool"
    docker run "${DOCKER_ARGS[@]}" bash -c "
      BIN=\$(find /opt/xcode-tools -name '$tool' 2>/dev/null | head -1)
      if [ -z \"\$BIN\" ]; then
        echo '  NOT FOUND'
        exit 1
      fi
      OUTPUT=\$(macho-inspect --exec \"\$BIN\" 2>&1)
      EXIT=\$?
      echo \"\$OUTPUT\" | grep -E 'output|end|Exit code|SIGSEG|fault|Shims' | head -10
      if echo \"\$OUTPUT\" | grep -q 'SIGSEGV'; then
        echo \"  CRASH (exit=\$EXIT)\"
      else
        echo \"  OK (exit=\$EXIT)\"
      fi
    "
    echo
  done
fi
