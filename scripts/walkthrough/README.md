# Walkthrough recording

Records the walkthrough video and the screenshots of the swarmy README on a headless display, so
that the desktop is not used. Linux only, it needs `kwin_wayland`, `Xwayland`, `xdotool`,
ImageMagick (`import`, `magick`) and `ffmpeg`.

```bash
# A swarmy release build with its sidecars next to it.
NO_STRIP=true cargo tauri build --no-bundle
# The replays to show, a directory with replays and nothing else (the snapshot is written in it),
# the map caches, and the duplicate replays folder outside of the replays: the duplicates moved there are
# moved back to the replays at the start of the next recording.
export WALKTHROUGH_REPLAYS=~/SC2Replays/Demo
export WALKTHROUGH_MAP_CACHES=~/SC2Replays/swarmy
export WALKTHROUGH_DUPLICATES=~/SC2Replays/Demo-duplicates
scripts/walkthrough/headless.sh start
scripts/walkthrough/record.sh
scripts/walkthrough/headless.sh stop
```

`headless.sh start` runs a virtual KWin session with a rootful Xwayland on `:7` (2000x1600),
`record.sh` starts swarmy with a new profile at 2x, the window is 1000x800, deletes the snapshot of
the replays and records every step with `ffmpeg`, the output is in `target/walkthrough`:
`raw.mp4`, `walkthrough.mp4` (1200x960), the screenshots and `chapters.txt` with the second each step
starts at.

The clicks are at fixed coordinates of the 1000x800 window, a change of the layout of a view needs
the coordinates of `record.sh` to be updated. The new replays banner is only shown until Optimize,
the views after it are 32 pixels higher.

## The tour of the browser version

`tour.py` records `docs/tour.mp4` and the screenshots of the views in Firefox, on the same headless
display, from a `swarmy --serve` backend. It opens each view by its address and finds the buttons by
their text, so it needs no coordinates. The administrator key comes from `SWARMY_ADMIN_KEY`, to
show Scan; it is never typed on screen.

```sh
scripts/walkthrough/headless.sh start
SWARMY_ADMIN_KEY=... python3 scripts/walkthrough/tour.py http://corvid-zt:7878 target/walkthrough-web
scripts/walkthrough/headless.sh stop
ffmpeg -i target/walkthrough-web/raw.mp4 -vf "scale=1200:960:flags=lanczos,fps=15" -c:v libx264 \
  -preset slow -crf 30 -pix_fmt yuv420p -movflags +faststart docs/tour.mp4
```
