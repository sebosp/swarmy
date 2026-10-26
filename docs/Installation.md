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
   optionally, the duplicate limbo directory and the new replays watcher.
2. **Scan**, then **Optimize** to generate the snapshot.
3. Explore the views, the **Help** view explains each of them.

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
