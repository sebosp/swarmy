# Swarmy - Starcrat II - Replay collection organizer / analysis

This repo is a frontend to bridge different tools to work together on data analysis.

# Tooling scopes:
- Frontend-backend: tauri with leptos (this repo)
- Decoding/batch-transformation into Apache Arrow via [s2protocol-rs](https://www.github.com/sebosp/s2protocol-rs)
- Map T3 Terrain Height navigation with [swarmy-bevy](https://www.github.com/sebosp/swarmy-bevy)
- Visualization Replay with [rerun](https://github.com/rerun-io/rerun) 
- Pola.rs/DuckDB analysis via [s2-polars-data-analysis](https://github.com/sebosp/s2-polars-data-analysis), these queries will become part of the frontend.

![Starcraft 2 Replay in Rerun](https://user-images.githubusercontent.com/873436/231281746-40fde3f1-fec6-49fe-8cf1-5fbd197589b7.png)

## Requirements: the sidecars

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

## Web UI Setup

The timeline must be switched to the "game_timeline" in the dropdown.
The gamespeed seems to be 22 FPS for "faster" game speed. This kindof matches replays.

![Screenshot for 2025EWC Grand Finals](https://github.com/user-attachments/assets/bf6ca086-db19-4054-94d9-0f1ae9a74b11)

## Example Recorded/Processed Replays for web.

Rerun 0.26.0:

[2025EWCClassicSerralGame8GrandFinals](https://sebosp.github.io/swarmy/public/0.26.0/index.html)

## Initial Focus
- Colored fog of war, which player has scouted which area.
- Hint on next region of interest: Since this is for replays, we know when important events are going to happen.
  we can guide the caster to position the window to specific regions where in the next X gameloops, an important
  activity is happening, for example massive damage or massive death on an area, i.e. splash from widow mines, disruptors, tanks.
  This could be similar to how a warning is seen for a Nydus or Nukes.
- Different visualizations on events, for example, we could draw the amount of deaths per regions of the map, like piling up bodies
  and showing what regions of a map have been more active than others.
- We can add many data visualizations.

# DONE

- "caches" should be stored in a global location to avoid double downloading for different snapshots.
- cache_handles are analized to be able to process the MapInfo uniqueness. However the version of the map hasn't been found on the MapInfo contents.

# TODO

## Tauri

- Currently only one version can be running at a time because of the settings.json that is written on "scan" tab.
- In the current queries, I only use one-player, however it should be easy to add multi-user queries,
  either for 2v2 or for specific player vs specific player.
- [ ] Watch the replay directories: when a new SC2Replay file is detected in the source directories
  (i.e. with the `notify` crate), add it to the snapshot automatically. The snapshot is partitioned by
  day and Optimize only adds the replays that are not in it yet, so a watch only has to run the same
  incremental Optimize, debounced so that a batch of new replays is added at once, and refresh the
  open views when it finishes.

## Ideas for casters

Inspired by what the Dota 2 broadcasts use: stat sites with matchup win rates and an API (Dotabuff,
Stratz, OpenDota), the "stat bibles" that statisticians prepare for casters and analysts before a
tournament (datDota), win prediction models that explain a game as it happens (presented at the MIT
Sloan Sports Analytics Conference), and HUD managers that put live comparisons and timers on the
stream (Lexogrine HUD Manager).

- [ ] **Stat bible**: one page per player of a tournament for the casters, exported as HTML/PDF:
  matchup and map win rates, signature openings and builds, head-to-head records, recent form,
  records (fastest third base, most workers at 6:00) and fun facts.
- [ ] **Win probability over the game**: a model trained on the snapshot from the leads at each
  minute (workers, army value, income, bases, upgrades), the curve of a replay shows the turning
  points. functime/polars in s2-polars-data-analysis is a place to prototype it.
- [ ] **Map veto helper**: each player's win rate per map and matchup in the current pool, which
  maps each one would veto, for best-of series.
- [ ] **Turning points and highlights**: the engagements with the largest swing in army value or win
  probability, with the replay time to jump to in Rerun or swarmy-bevy.
- [ ] **Live overlay**: a browser source for OBS with the army, income and worker comparisons of
  the game being casted, from the SC2 client API (`localhost:6119`) while it is played.
- [ ] **Fun facts generator**: streaks, records per map and patch, first time a build was played,
  "the last time player A beat player B on this map".
- [ ] **Build order popularity per patch**: the "pick/ban rates" of StarCraft II, which openings are
  rising or falling after a balance patch.
- [ ] **Balance dashboard**: the matchup win rates per patch with confidence intervals and the
  number of games, so that a 60% from 20 games is not read as imbalance.
- [ ] **An API or export of the aggregates** (like OpenDota) so that other tools and overlays can
  use them.
- [ ] Normalize the localized race names (테란, Терраны, 神族...) in s2protocol-rs, swarmy and the
  notebooks map them to T, Z and P for now.

## Rerun
- Some units generate too many actions to be displayed in the current way as separate events in rerun,  for example a Lurker may create a temporary InvisibleTargetUnit for every cycle the spines shoots, this needs to be somehow shortened.
- Sometimes the state management in s2protocol-rs may lose sync, at this point rerun is very useful for debugging and seeing how the state transists through events and what abilities may be missing.
- Some events seem a bit redundanct, i.e. a Larva "dies" to become a "drone" or an "Adept" has an "AdeptPhaseShift" temporary unit that "dies" and this also causes a lot of events.
- The Target Position arrows (where each unit is pointing to) are batched in a single entity per player and cleared after a couple of seconds, they may still be too busy in big fights.

## Statistics from the snapshot

The snapshot written by Optimize contains the s2protocol-rs `FlatRow` structs:
`PlayerLobbyDetailsFlatRow` (details.arrow), `PlayerStatsFlatRow` (stats.arrow),
`UpgradeEventFlatRow` (upgrades.arrow), `UnitBornEventFlatRow` (unit_born.arrow),
`UnitDiedEventFlatRow` (unit_died.arrow), `CmdTargetPointEventFlatRow` (cmd_target_point.arrow) and
`CmdTargetUnitEventFlatRow` (cmd_target_unit.arrow). `UserInitDataFlatRow` exists but is not written
to the snapshot yet. Ideas of what could be shown with them:

Most of them are in the Player view, from the analysis of every game of the player, and in the
Replay Detail view, see the Help view for how each one is calculated.

### For players that want to improve
- [x] Benchmarks: workers, supply, income and army value at 4, 6, 8 and 10 minutes against the snapshot average of the same matchup.
- [x] Supply blocks: time with `food_used` reaching `food_made` below 200.
- [x] Floating resources: time with more than 1000 minerals and vespene unspent.
- [x] Expansion timings (Replay Detail) and upgrade priorities (Player). Comparing them to the pros of the same map and matchup is still to do.
- [x] Expansion and upgrade timings compared to the average of the games of the same patch (Replay Detail), i.e. "Metabolic Boost 5s faster than average".
- [x] Map resources (Map Details): minerals and vespene of the map, gold minerals and rich vespene, and the distance from each start location to its main base mineral fields and geysers, from the Objects of the map cache, over the Minimap.tga of the map cache.
- [x] Trades: engagements with the resources and mining time lost by each side, and the trade ratio of each unit type.
- [x] What kills my units: losses by killer unit type, per game and per player.
- [x] Where I lose my units on each map: the Deaths layer of Map Scout.
- [x] Idle production: time between units of the same production structure.
- [x] Macro cadence: injects, chrono boosts and MULEs per game. The intervals between them are still to do.
- [x] Commands per minute through the game, per minute in Replay Detail and on average in Player.
- [x] Player focus: the selections and commands of each minute by army, macro, economy, expand and upgrades, and the longest time with only army actions (Replay Detail). Read from the replay files, selections are not in the snapshot, so it is not averaged across games in the Player view yet.
- [x] Win rates by matchup, map and game length.
- [x] Rating progression, read from the replays as `UserInitDataFlatRow` is not in the snapshot. Pro games played on LAN have no rating.

### For casters of online tournaments
- [x] Openings: the first three tech structures and the first army unit, by matchup and map, with the share of games and win rate.
- [x] Head to head records and recent form (Player, opponents and the last ten games).
- [x] Proxies: production structures far from the start location before 04:00.
- [x] Timing of the first attack: the first enemy unit killed.
- [x] Favourite army composition at 10 minutes and the most produced army unit by map.
- [x] Comebacks: games won after being below 70% of the enemy army value.
- [x] Bases at 10 minutes and how often games end in a base trade.
- [x] Upgrade priorities: the first upgrade researched.
- [x] Map veto help: win rate of each player per map.
- [x] Scouting habits: when and with which unit the enemy start location is first scouted, and Map Scout.

### Missing data
- [ ] Camera positions and chat messages are only read from the replay files, adding them to the snapshot would allow camera based statistics (where the player looks) across games.
- [x] The protocol version of each replay is in details.arrow (`ext_protocol_version`), with the s2protocol-rs `fix-snapshot-ipc-schemas` branch.
- [x] metadata.json in the snapshot with the s2protocol-rs version that wrote it, shown in the Scan statistics with whether it is compatible.
- [x] The snapshot is partitioned by day (`ipcs/year=YYYY/month=MM/day=DD/`), Optimize only adds the replays that are not in it yet and Rebuild generates it from scratch.
- [ ] Selections (control groups and selection deltas) are not in the snapshot, the focus statistics need the replay files.
- [ ] The `ext_replay_seconds` column is computed by s2protocol-rs with 68.58 ms per game loop, about 14.6 loops per second, Faster speed is 22.4. Swarmy computes the seconds from `ext_replay_loop` instead.

# Development

## Dependencies
Tauri requirement plus tailwindcss and daisyui
```
$ curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/download/v4.2.4/tailwindcss-linux-x64
$ vim README.md
$ chmod a+x tailwindcss-linux-x64
$ mv tailwindcss-linux-x64 tailwindcss
$ mv tailwindcss ~/local/bin/
$ curl -sLO https://github.com/dobicinaitis/tailwind-cli-extra/releases/download/v2.8.3/tailwindcss-extra-linux-x64
$ chmod a+x tailwindcss-extra-linux-x64
$ mv tailwindcss-extra-linux-x64 tailwindcss-extra
$ mv tailwindcss-extra ~/local/bin/
$ npm install daisyui
```

There's a bit of a strange bug when installing tailwindcss / tailwindcss-extra in Trunk.toml, for some reason installing one tries to
I think I had to use something like
```toml
[tool]
tailwindcss = "2.7.0" # This is actually tailwindcss-extra version
```
Then `trunk build` downloads the tailwindcss-extra plugin version 2.7.0 :shrug:
And then **sometimes** it works?

### Charts
The charts are drawn with [ApexCharts](https://apexcharts.com) 5.3.6 the way the
[rust-ui charts](https://leptos.rust-ui.com) do it: the Leptos components in
`src/components/charts.rs` write the data as `data-chart-*` attributes and
`public/charts/chart_init.js` (adapted from rust-ui, MIT) renders them. `apexcharts.5.3.6.min.js` is
vendored in `public/charts`, ApexCharts 5 is dual licensed, its community license
(`public/charts/APEXCHARTS-LICENSE.md`) is free for individuals and organizations under 2M USD of
annual revenue, with attribution.

### Sortable tables
`public/table_sort.js` sorts any table by clicking on the headers of its `<thead>`, a table sorted by
the backend has `data-server-sort`, a header that should not sort has `data-no-sort`.

## Releases
`.github/workflows/ci.yaml` checks the formatting, runs the tests and checks the frontend.
`.github/workflows/release.yaml` builds the bundles of Linux (x86_64, aarch64), Windows (x86_64) and
macOS (Apple Silicon) for a `v*` tag, the platforms of the rerun-cli releases, with the sidecars
downloaded by `scripts/download-sidecars.sh`, so the swarmy-bevy release must exist first. The
updater artifacts are signed with the `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets, without them the bundles have no updater
artifacts.
