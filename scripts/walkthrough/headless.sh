#!/usr/bin/env bash
# Starts or stops the headless display :7 of the walkthrough: a virtual KWin session and a rootful
# Xwayland in it, xdotool can move the pointer of a rootful Xwayland.
set -euo pipefail

STATE_DIR="${XDG_RUNTIME_DIR:-/tmp}/swarmy-walkthrough"
mkdir -p "$STATE_DIR"

case "${1:-}" in
  start)
    env -u DISPLAY WAYLAND_DISPLAY= setsid kwin_wayland --virtual --xwayland --width 1600 \
      --height 1000 --socket wayland-swarmy-walkthrough --no-lockscreen \
      > "$STATE_DIR/kwin.log" 2>&1 < /dev/null &
    echo $! > "$STATE_DIR/kwin.pid"
    sleep 5
    env -u DISPLAY WAYLAND_DISPLAY=wayland-swarmy-walkthrough setsid Xwayland :7 \
      -geometry 2000x1600 -noreset > "$STATE_DIR/xwayland.log" 2>&1 < /dev/null &
    echo $! > "$STATE_DIR/xwayland.pid"
    sleep 3
    DISPLAY=:7 xdotool getdisplaygeometry
    ;;
  stop)
    for name in xwayland kwin; do
      if [[ -f "$STATE_DIR/$name.pid" ]]; then
        kill "$(cat "$STATE_DIR/$name.pid")" 2>/dev/null || true
        rm "$STATE_DIR/$name.pid"
      fi
    done
    ;;
  *)
    echo "Usage: $0 start|stop" >&2
    exit 1
    ;;
esac
