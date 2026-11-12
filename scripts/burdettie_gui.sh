#!/usr/bin/env bash
# burdettie_gui.sh — Start a virtual display plus optional VNC/noVNC viewers,
# then run a GUI-capable Burdettie command inside the container.

set -euo pipefail

if [ $# -eq 0 ]; then
    echo "usage: burdettie-gui <command> [args...]" >&2
    echo "example: burdettie-gui macho-inspect --exec --gui /workspace/fixtures/cocoa_draw/app" >&2
    exit 64
fi

display_value="${BURDETTIE_GUI_DISPLAY:-:99}"
screen_value="${BURDETTIE_GUI_SCREEN:-1440x900x24}"
enable_vnc="${BURDETTIE_GUI_ENABLE_VNC:-1}"
enable_novnc="${BURDETTIE_GUI_ENABLE_NOVNC:-1}"
vnc_port="${BURDETTIE_GUI_VNC_PORT:-5900}"
novnc_port="${BURDETTIE_GUI_NOVNC_PORT:-6080}"
wait_secs="${BURDETTIE_GUI_WAIT_SECS:-10}"
hold_open="${BURDETTIE_GUI_HOLD_OPEN:-0}"
novnc_dir="${BURDETTIE_GUI_NOVNC_DIR:-/usr/share/novnc}"

xvfb_log="${BURDETTIE_GUI_XVFB_LOG:-/tmp/burdettie-xvfb.log}"
x11vnc_log="${BURDETTIE_GUI_VNC_LOG:-/tmp/burdettie-x11vnc.log}"
novnc_log="${BURDETTIE_GUI_NOVNC_LOG:-/tmp/burdettie-novnc.log}"

xvfb_pid=""
x11vnc_pid=""
novnc_pid=""

cleanup() {
    local status=$?

    if [ -n "$novnc_pid" ] && kill -0 "$novnc_pid" 2>/dev/null; then
        kill "$novnc_pid" 2>/dev/null || true
        wait "$novnc_pid" 2>/dev/null || true
    fi
    if [ -n "$x11vnc_pid" ] && kill -0 "$x11vnc_pid" 2>/dev/null; then
        kill "$x11vnc_pid" 2>/dev/null || true
        wait "$x11vnc_pid" 2>/dev/null || true
    fi
    if [ -n "$xvfb_pid" ] && kill -0 "$xvfb_pid" 2>/dev/null; then
        kill "$xvfb_pid" 2>/dev/null || true
        wait "$xvfb_pid" 2>/dev/null || true
    fi

    exit "$status"
}

trap cleanup EXIT INT TERM

if [ "$enable_novnc" != "0" ] && [ "$enable_vnc" = "0" ]; then
    enable_vnc=1
fi

export DISPLAY="$display_value"

Xvfb "$DISPLAY" -screen 0 "$screen_value" -ac -nolisten tcp >"$xvfb_log" 2>&1 &
xvfb_pid=$!

ready=0
for _ in $(seq 1 $((wait_secs * 10))); do
    if xdpyinfo -display "$DISPLAY" >/dev/null 2>&1; then
        ready=1
        break
    fi
    if ! kill -0 "$xvfb_pid" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

if [ "$ready" != "1" ]; then
    echo "burdettie-gui: failed to start Xvfb on $DISPLAY" >&2
    if [ -f "$xvfb_log" ]; then
        cat "$xvfb_log" >&2
    fi
    exit 1
fi

echo "burdettie-gui: display ready on $DISPLAY ($screen_value)"

if [ "$enable_vnc" != "0" ]; then
    x11vnc -display "$DISPLAY" -rfbport "$vnc_port" -forever -shared -nopw -quiet >"$x11vnc_log" 2>&1 &
    x11vnc_pid=$!
    echo "burdettie-gui: VNC available on 127.0.0.1:${vnc_port}"

    if [ "$enable_novnc" != "0" ]; then
        if [ ! -d "$novnc_dir" ]; then
            echo "burdettie-gui: noVNC assets not found at $novnc_dir" >&2
            exit 1
        fi
        websockify --web "$novnc_dir" "$novnc_port" "127.0.0.1:${vnc_port}" >"$novnc_log" 2>&1 &
        novnc_pid=$!
        echo "burdettie-gui: noVNC available at http://127.0.0.1:${novnc_port}/vnc.html?autoconnect=1&resize=remote"
    fi
fi

set +e
"$@"
command_status=$?
set -e

if [ "$hold_open" = "1" ]; then
    echo "burdettie-gui: command exited with status ${command_status}; GUI services remain up until Ctrl+C."
    wait "$xvfb_pid"
fi

exit "$command_status"