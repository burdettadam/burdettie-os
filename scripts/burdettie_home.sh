#!/usr/bin/env bash
# burdettie_home.sh — Start the native Burdettie home screen.

set -euo pipefail

export DARWIN_LOADER_ROOT="${DARWIN_LOADER_ROOT:-/workspace}"

exec /usr/local/bin/launchpad "$@"