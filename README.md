# swarmy, a StarCraft II replay batch analyser

swarmy reads a whole directory of StarCraft II replays at once to find trends: which units, openings
and timings are played on each map and patch, and how they change over time. Casters can prepare
statistics about players, maps, race balance (PvT, TvZ, ZvP...) and player vs player records, and go
through the replays of a series while casting.

- **Snapshot**: the replays are decoded by [s2protocol-rs](https://github.com/sebosp/s2protocol-rs)
  into Arrow IPC files partitioned by day, only the new replays are read again, a watcher tells when
  there are new ones and the duplicates can be moved out of the way.
- **Views**: maps (minimap, resources, matchups), replays with filters by matchup, patch, time
  frame, tag, rating and chat messages, the analysis of a replay with notes, ratings and tags,
  players and their head to head records, where the units go in each map, the units of every
  patch and the evolution of their costs.
- **Viewers**: a replay in [Rerun](https://rerun.io) and in 3D on its map with
  [swarmy-bevy](https://github.com/sebosp/swarmy-bevy).
- **Browser**: `swarmy --serve` serves the same views to a browser, the address of each view can be
  shared; an administrator key protects Scan, Optimize and the settings.
- **Battle.net**: with the Client ID and Secret of a Battle.net application, the Player view shows
  the public profiles of the accounts: the best 1v1 finish, the ladders of the season and their MMR.

## Walkthrough

[![The swarmy tour video](docs/screenshots/replay_detail.png)](docs/tour.mp4)

[The tour video](docs/tour.mp4) (3:27) goes through every view of swarmy 0.2.4 in the browser, on
the 36,394 games of a `swarmy --serve` backend:

| Time | Step |
|------|------|
| 0:00 | Home: the snapshot and where it comes from |
| 0:06 | Config: the administrator key, the theme, Battle.net |
| 0:12 | Scan: the replays and the snapshot of the server |
| 0:22 | Map List |
| 0:32 | Map Details: the minimap, the matchups, the resources |
| 0:46 | Map Scout, experimental: where the first units go |
| 0:56 | Replay List: the games and their best rank |
| 1:07 | Replay Detail: Reynor against Serral on Rainfall, the win chance, the lessons, the Focus |
| 1:49 | Player: Serral, the accounts, Battle.net, the insights |
| 2:13 | Patch Overview |
| 2:26 | Patch Evolution: a unit over the patches, the abilities |
| 2:41 | Unit Overview: the race, then the unit |
| 2:53 | Ability: Fungal Growth |
| 3:04 | Race Detail |
| 3:14 | Help and Changelog |

The first steps in the app (Config, Scan, Optimize, the duplicates), Rerun and swarmy-bevy are in
[the walkthrough video of 0.2.1](docs/walkthrough.mp4) (5:19), from a fresh install with 833
replays of the RSL season:

| Time | Step |
|------|------|
| 0:00 | Home |
| 0:09 | Config: the map caches, the replays, the duplicate replays folder and the new replays watcher |
| 0:31 | Scan |
| 0:41 | Optimize |
| 0:57 | Duplicates moved to the duplicates folder |
| 1:16 | Map List, sorted by games |
| 1:23 | Map Details |
| 1:32 | Replay List of the PvT games of the map |
| 1:38 | Replay Detail: notes, rating, tags, locks, previous and next |
| 2:28 | Replay List: tags and chat messages |
| 2:56 | Player |
| 3:16 | Map Scout |
| 3:23 | Patch Overview |
| 3:28 | Unit Overview |
| 3:34 | Patch Evolution |
| 4:13 | Rerun preview |
| 4:43 | swarmy-bevy: the units died, born, of one unit type |

More in [Screenshots](docs/Screenshots.md). Since 0.2.1, besides what the tour shows:

- A [remote backend](docs/Installation.md#remote-backend): a computer that cannot handle the
  snapshot uses the swarmy of another one through Config → Remote Backend.
- [Zoom, themes and accessibility](docs/Installation.md#appearance-and-accessibility) in Config:
  Ctrl+= and Ctrl+- zoom the interface, the theme can be Swarmy, Light, Dark, High contrast,
  Catppuccin or Tokyo Night, or follow the desktop; the motion, link underlines and focus outline
  can be set. All of it is kept on the computer.
- s2protocol 5.0.0: the replay types are read with the serde_mpq derives, checked against
  Blizzard's definitions, and Scan tells when the snapshot is being created or was interrupted.

## Documentation

- [Installation](docs/Installation.md): the releases, the first steps, the sidecars and the logs.
- [Screenshots](docs/Screenshots.md) of every view.
- [Statistics](docs/Statistics.md): what the snapshot has and what swarmy shows from it.
- [Roadmap](docs/Roadmap.md): the ideas for casters and what is left to do.
- [Development](docs/Development.md): building swarmy, the charts, the releases and the walkthrough.
- The Help view in swarmy explains how each statistic is calculated, the Changelog view what is
  new.

## Related projects

- [s2protocol-rs](https://github.com/sebosp/s2protocol-rs): decodes the replays and writes the
  snapshot, a rustified version of Blizzard's [s2protocol](https://github.com/Blizzard/s2protocol).
- [swarmy-bevy](https://github.com/sebosp/swarmy-bevy): the maps and replays in 3D.
- [Rerun](https://github.com/rerun-io/rerun): the replay preview.
- [s2-polars-data-analysis](https://github.com/sebosp/s2-polars-data-analysis): notebooks with
  Polars, DuckDB and functime on the snapshot.
