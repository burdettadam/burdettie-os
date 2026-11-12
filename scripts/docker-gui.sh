#!/usr/bin/env bash
# docker-gui.sh — Run a GUI-enabled Burdettie session with noVNC/VNC port maps.

set -euo pipefail

ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
cd "$ROOT"

IMAGE="${BURDETTIE_DOCKER_IMAGE:-burdettie-os}"
PLATFORM="${BURDETTIE_DOCKER_PLATFORM:-}"
HOST_VNC_PORT="${BURDETTIE_GUI_HOST_VNC_PORT:-5900}"
HOST_NOVNC_PORT="${BURDETTIE_GUI_HOST_NOVNC_PORT:-6080}"

TTY_ARGS=()
if [ -t 0 ] && [ -t 1 ]; then
    TTY_ARGS=(-it)
fi

ENV_ARGS=()
for name in RUST_LOG DARWIN_LOADER_ROOT BURDETTIE_GUI_DISPLAY BURDETTIE_GUI_ENABLE_NOVNC BURDETTIE_GUI_ENABLE_VNC BURDETTIE_GUI_HOLD_OPEN BURDETTIE_GUI_MODE BURDETTIE_GUI_SCREEN BURDETTIE_GUI_WAIT_SECS BURDETTIE_GUEST_APP; do
    if [ -n "${!name:-}" ]; then
        ENV_ARGS+=(-e "$name=${!name}")
    fi
done

if [ $# -eq 0 ]; then
    echo "Starting the Burdettie home screen in ${IMAGE}."
    echo "Viewer: http://127.0.0.1:${HOST_NOVNC_PORT}/vnc.html?autoconnect=1&resize=remote"
    echo "Guest smoke example: BURDETTIE_GUI_MODE=guest BURDETTIE_GUEST_APP=/workspace/fixtures/cocoa_draw/app ./scripts/docker-gui.sh"
    echo "Imported app example: BURDETTIE_GUI_MODE=guest BURDETTIE_GUEST_APP=/opt/imported-artifacts/apps/Stickies.app/Contents/MacOS/Stickies ./scripts/docker-gui.sh"
    echo "Open-source terminal example: BURDETTIE_GUI_MODE=guest BURDETTIE_GUEST_APP=/opt/imported-artifacts/apps/WezTerm.app/Contents/MacOS/wezterm-gui ./scripts/docker-gui.sh"
    set -- burdettie-gui-session
elif [ "$1" = "--shell" ]; then
    echo "Starting a GUI-enabled shell in ${IMAGE}."
    echo "Viewer: http://127.0.0.1:${HOST_NOVNC_PORT}/vnc.html?autoconnect=1&resize=remote"
    shift
    set -- bash
else
    echo "Viewer: http://127.0.0.1:${HOST_NOVNC_PORT}/vnc.html?autoconnect=1&resize=remote"
fi

DOCKER_ARGS=(docker run --rm "${TTY_ARGS[@]}")

if [ -n "$PLATFORM" ]; then
    DOCKER_ARGS+=(--platform "$PLATFORM")
fi

DOCKER_ARGS+=(
    -p "${HOST_VNC_PORT}:5900"
    -p "${HOST_NOVNC_PORT}:6080"
    -v "${ROOT}:/workspace"
    -w /workspace
    "${ENV_ARGS[@]}"
    "$IMAGE"
    burdettie-gui "$@"
)

"${DOCKER_ARGS[@]}"