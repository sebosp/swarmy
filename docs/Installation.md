# Installation

## Download

The bundles of every release are in https://github.com/sebosp/swarmy/releases:

- Linux (x86_64 and aarch64): `.deb`, `.rpm` or `.AppImage`.
- Windows (x86_64): the `.msi` or the `-setup.exe` installer.
- macOS (Apple Silicon): the `.dmg`.

They include the sidecars, rerun-cli and swarmy-bevy. The builds are not signed by Apple or
Microsoft, the system may ask to confirm that they can run.

Then, in swarmy:

1. **Config**: the directory of the replays, the directory where the map caches are downloaded and,
   optionally, the duplicate replays folder and the new replays watcher.
2. **Scan**, then **Optimize** to generate the snapshot.
3. Explore the views, the **Help** view explains each of them.

## Logs

swarmy writes a log file per day, `swarmy.<date>.log`, and keeps the last 7:

| Platform | Directory |
|----------|-----------|
| Linux    | `~/.local/share/com.swarmy/logs` (`$XDG_DATA_HOME/com.swarmy/logs`) |
| Windows  | `%LOCALAPPDATA%\com.swarmy\logs` |
| macOS    | `~/Library/Logs/com.swarmy` |

The logs are also printed when swarmy is started from a terminal. By default swarmy logs what it
does and the libraries only their warnings, the verbosity flags give more:

```bash
swarmy -v     # info of every library
swarmy -vv    # debug, and the time each step takes: the setup, the plugins, each request of the views
swarmy -vvv   # trace, everything
# A filter of tracing-subscriber's EnvFilter instead of -v, i.e. only swarmy in detail:
SWARMY_LOG=info,swarmy_tauri_lib=trace swarmy
```

s2protocol, the library that reads the replays, stays at `warn` with `-v`, `-vv` and `-vvv`: its
`debug` logs are every field of every replay read, gigabytes for a single Optimize. Only enable them with
`SWARMY_LOG` for a few replays, i.e. `SWARMY_LOG=info,s2protocol=debug`.

The time spans show up as `close time.busy=... time.idle=...` lines, a slow start or view can be
found by looking for the largest ones, or for a gap between the timestamps. On Windows the app
does not print to the console, run it from PowerShell and read the log file:
`& "$env:LOCALAPPDATA\swarmy\swarmy.exe" -vv` for the `-setup.exe` installer, or
`& "C:\Program Files\swarmy\swarmy.exe" -vv` for the `.msi`.

## Remote backend

A computer that cannot handle the snapshot, i.e. a laptop with the 36k replays of aiarena, can use
the swarmy of another computer as its backend. There, swarmy runs without a window:

```bash
# The Replay Path, Map Caches Path, notes and goals of the swarmy app of this computer.
swarmy --serve 0.0.0.0:7878
# Only the requests with the token, a settings directory of its own:
SWARMY_SERVER_TOKEN=... swarmy --serve 0.0.0.0:7878 --data-dir ~/swarmy-server
```

On Linux the .deb, .rpm and Arch packages (`packaging/arch/PKGBUILD`, from the .deb) have it as
a systemd user service, it listens on localhost only until `~/.config/swarmy/server.env` says
otherwise:

```bash
mkdir -p ~/.config/swarmy
cat > ~/.config/swarmy/server.env <<EOF
SWARMY_SERVE_ADDRESS=0.0.0.0:7878
SWARMY_SERVER_TOKEN=$(openssl rand -hex 16)
# The settings and personal database, the ones of the swarmy app by default:
#SWARMY_SERVER_DATA_DIR=$HOME/swarmy-server
EOF
systemctl --user daemon-reload   # once after installing the package
systemctl --user enable --now swarmy-server
loginctl enable-linger   # starts it on boot instead of on login
journalctl --user -u swarmy-server -f
```

It runs with a lower priority (`nice` 10, idle I/O) so that the games and the desktop of that
computer go first. With the AppImage or a build of your own, copy
`packaging/linux/swarmy-server.service` to `~/.config/systemd/user/` and point `ExecStart` to the
binary, i.e. with a drop-in `~/.config/systemd/user/swarmy-server.service.d/override.conf`:

```ini
[Service]
ExecStart=
ExecStart=%h/Applications/swarmy.AppImage --serve ${SWARMY_SERVE_ADDRESS}
```

In the Config of the laptop, Remote Backend is its URL, i.e. `http://wormgat-zt:7878`, and Remote
Backend Token its token. Check the Remote Backend tells whether it answers, and its version: both
should be the same version of swarmy. Scan, Optimize and every query of the snapshot then run in
the backend, with its snapshot, its notes and tags and its progress goals, the progress of
Optimize is shown as usual. Leave Remote Backend empty to run everything in the laptop again.

The rest runs in the laptop: Config itself, the folder dialogs, the new replays watcher, the Patch
views and the Rerun and swarmy-bevy viewers. The viewers open the replays and map caches at the
paths of the backend, they only work when the laptop has them at the same paths.

The backend is plain HTTP, keep it in a private network such as ZeroTier, or behind a proxy with
TLS. The desktop app and `--serve` can run at the same time in the same computer, they share the
personal database.

## Appearance and accessibility

In Config, under the remote backend:

- **Zoom**: the size of the whole interface, text, charts and icons, from 50% to 250% in steps
  of 10%. `Ctrl+=` (or `Ctrl++`) zooms in, `Ctrl+-` out and `Ctrl+0` resets, in any view (`Cmd` on
  macOS, the numpad keys work too); the new zoom shows for a moment in the corner and is saved as
  it changes.
- **Theme**: Swarmy (the dark gray and indigo look swarmy had), Light, Dark, High contrast, the
  Catppuccin flavors (Latte, Frappé, Macchiato, Mocha) and the Tokyo Night variants (Day, Storm,
  Night), or Follow the system, which picks Swarmy or Light from the light or dark preference of
  the desktop and follows its changes. The theme applies as it is picked and is kept with Save.
  The charts use one palette for the dark themes and one for the light ones, checked for color
  vision deficiencies (deutan, protan, tritan) and for contrast with the surface. The light
  themes darken the accents used as text to the 4.5:1 of WCAG AA; High contrast is the theme for
  black surfaces, white text and thicker borders.
- **Motion**: Follow the system uses the Reduce animations preference of the desktop, Reduce
  stops the animations and transitions, Animate keeps them.
- **Links and focus**: underline the links always, not only on hover; always show the focus
  outline, also after a click, to follow the keyboard.

These settings are stored with the others in `settings.json` of the app data directory (see
[Logs](#logs) for the directories), nothing is sent anywhere; with a remote backend they stay on
the computer that shows the interface.

## Building from the sources

See [Development](Development.md).

## The sidecars

swarmy runs two programs next to it, Tauri sidecars, that must be in the `assets` directory, named
`<name>-<target triple>` (`.exe` on Windows), i.e. `assets/rerun-cli-x86_64-unknown-linux-gnu`:

- **rerun-cli**, the [Rerun](https://rerun.io) Viewer that "Load Rerun Preview" streams the replay
  to. It is the `rerun` binary of the Rerun releases, it must be the **same version as the `rerun`
  crate in `Cargo.lock`** (0.40.0), the viewer and the SDK must match.
  - Docs: https://rerun.io/docs/getting-started/installing-viewer
  - Releases: https://github.com/rerun-io/rerun/releases, the `rerun-cli-<version>-<target triple>`
    assets. There are builds for Linux (x86_64, aarch64), Windows (x86_64) and macOS (Apple
    Silicon), there is no rerun-cli for Intel macOS.
  - Source: https://github.com/rerun-io/rerun
  - It can also be installed with `cargo install rerun-cli --locked --version <version>` and copied
    to `assets/rerun-cli-<target triple>`. A viewer already listening on the default port (9876),
    i.e. started with `rerun`, is reused.
- **swarmy-bevy**, the 3D view of a map and of a replay on its map, from
  https://github.com/sebosp/swarmy-bevy/releases, or built from its repository with
  `cargo run --bin tauri-sidecar -- ../swarmy/assets`.

`scripts/download-sidecars.sh [target triple]` downloads both for the current platform, the rerun-cli
version from `Cargo.lock`:

```bash
scripts/download-sidecars.sh
# Another platform or swarmy-bevy release:
SWARMY_BEVY_VERSION=v0.2.0 scripts/download-sidecars.sh x86_64-pc-windows-msvc
```
