#!/usr/bin/env bash
# burdettie_gui_session.sh — Pick the default GUI session payload.

set -euo pipefail

mode="${BURDETTIE_GUI_MODE:-home}"

case "$mode" in
    home)
        exec /usr/local/bin/burdettie-home
        ;;
    guest)
        guest_app="${BURDETTIE_GUEST_APP:-/workspace/fixtures/cocoa_draw/app}"
        exec /usr/local/bin/macho-inspect --exec --gui "$guest_app"
        ;;
    *)
        echo "burdettie-gui-session: unknown BURDETTIE_GUI_MODE '$mode'" >&2
        echo "expected 'home' or 'guest'" >&2
        exit 64
        ;;
esac