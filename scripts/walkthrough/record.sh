#!/usr/bin/env bash
# Records the swarmy walkthrough on the headless display :7 of headless.sh (2000x1600, the app at
# 2x) from a new profile: Config, Scan, Optimize, Map List, Map Details, Replay List, Replay Detail,
# Player, Map Scout, Patch and Unit Overview, Rerun and swarmy-bevy. Saves the screenshots on the
# way, see README.md.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/target/walkthrough"
PROFILE="$OUT/profile"
: "${WALKTHROUGH_REPLAYS:?the directory of the replays to show}"
: "${WALKTHROUGH_MAP_CACHES:?the directory of the map caches}"
: "${WALKTHROUGH_BALANCE_DATA:?the directory of the balance data}"
export DISPLAY=:7

# The snapshot is generated from scratch with the new profile.
rm -rf "$OUT" "$WALKTHROUGH_REPLAYS/ipcs"
mkdir -p "$PROFILE"/{data,config,cache}

# The coordinates are the ones of the 1000x800 window, drawn at 2x.
click() { xdotool mousemove $(($1 * 2)) $(($2 * 2)); sleep 0.4; xdotool click 1; sleep "${3:-1}"; }
move() { xdotool mousemove $(($1 * 2)) $(($2 * 2)); sleep "${3:-0.5}"; }
type_text() { xdotool type --delay 20 "$1"; }
scroll() { move 500 "${3:-500}" 0.2; for _ in $(seq "$2"); do xdotool click "$1"; sleep 0.15; done; sleep 1; }
snap() { import -window root "$OUT/$1.png"; }
START=$(date +%s)
chapter() { echo "$(($(date +%s) - START)) $1" >> "$OUT/chapters.txt"; }

ffmpeg -loglevel error -y -f x11grab -video_size 2000x1600 -framerate 20 -draw_mouse 1 -i :7 \
  -c:v libx264 -preset veryfast -crf 18 -pix_fmt yuv420p "$OUT/raw.mp4" &
FFMPEG=$!
sleep 1

chapter "Home"
setsid env -u WAYLAND_DISPLAY DISPLAY=:7 GDK_BACKEND=x11 GDK_SCALE=2 WEBKIT_DISABLE_DMABUF_RENDERER=1 \
  XDG_DATA_HOME="$PROFILE/data" XDG_CONFIG_HOME="$PROFILE/config" \
  XDG_CACHE_HOME="$PROFILE/cache" "$ROOT/target/release/swarmy" > "$OUT/swarmy.log" 2>&1 < /dev/null &
sleep 6
snap home
sleep 2

chapter "Config: the map caches, the replays and the balance data"
click 22 498 2
click 308 202 1
click 530 281 0.5
type_text "$WALKTHROUGH_MAP_CACHES"
xdotool key Tab; sleep 0.3
type_text "$WALKTHROUGH_REPLAYS"
xdotool key Tab; sleep 0.3
type_text "$WALKTHROUGH_BALANCE_DATA"
sleep 1
snap config
scroll 5 5
click 530 756 3
snap new_replays_banner
sleep 2

chapter "Scan"
click 22 60 2
click 583 81 5
snap scan
sleep 2

chapter "Optimize"
click 668 81 4
snap optimize_progress
# Optimize writes metadata.json at the end, 12 to 40 seconds for 834 replays.
for _ in $(seq 120); do
  [ -f "$WALKTHROUGH_REPLAYS/ipcs/metadata.json" ] && break
  sleep 1
done
sleep 4
snap optimize
scroll 5 6
sleep 2
snap snapshot_statistics
scroll 4 6

# Without new replays the banner is gone, the views are 32 pixels higher from here.
chapter "Map List, sorted by games"
click 22 104 3
click 450 119 1
click 450 119 2
snap map_list

chapter "Map Details"
click 90 163 6
snap map_details
sleep 2

chapter "Replay List of the PvT games of the map"
click 600 221 5
click 645 121 3
snap replay_list
sleep 1

chapter "Replay Detail"
click 64 172 9
snap replay_detail
scroll 5 8
snap replay_detail_charts
scroll 5 8
sleep 1
scroll 4 16

chapter "Player"
click 22 256 3
click 190 57 1
type_text "Classic"
xdotool key Return
sleep 9
snap player
scroll 5 6
scroll 4 6

chapter "Map Scout"
click 22 294 7
snap map_scout

chapter "Patch Overview"
click 22 339 4
snap patch_overview

chapter "Unit Overview"
click 22 377 5
snap unit_overview
sleep 1

# The Rerun preview and swarmy-bevy of the last replay opened.
chapter "Rerun preview"
click 22 218 6
scroll 4 16
click 713 41 1
for _ in $(seq 60); do
  R=$(xdotool search --name '^Rerun$' | head -1)
  [ -n "$R" ] && break
  sleep 1
done
if [ -n "$R" ]; then
  xdotool windowsize $R 2000 1600
  sleep 14
  snap rerun
fi
# The viewer covers swarmy, it is closed before opening swarmy-bevy.
kill $(pgrep -x rerun-cli) 2>/dev/null
sleep 2

chapter "swarmy-bevy"
click 888 41 1
for _ in $(seq 30); do
  B=$(xdotool search --name '^swarmy-bevy$' | head -1)
  [ -n "$B" ] && break
  sleep 1
done
sleep 8
if [ -n "$B" ]; then
  xdotool windowsize $B 2000 1600
  xdotool windowfocus $B
  sleep 2
  for _ in $(seq 16); do xdotool key Right; sleep 0.5; done
  xdotool key End
  sleep 3
  snap swarmy_bevy
fi
chapter "End"
sleep 1
kill -INT $FFMPEG
wait $FFMPEG

pkill -x swarmy-bevy
pkill -x swarmy

# The README video and screenshots.
ffmpeg -loglevel error -y -i "$OUT/raw.mp4" -vf "scale=1200:960:flags=lanczos,fps=15" \
  -c:v libx264 -preset slow -crf 30 -pix_fmt yuv420p -movflags +faststart "$OUT/walkthrough.mp4"
mkdir -p "$OUT/screenshots"
for png in "$OUT"/*.png; do
  magick "$png" -resize 1200x -strip "$OUT/screenshots/$(basename "$png")"
done
cat "$OUT/chapters.txt"
