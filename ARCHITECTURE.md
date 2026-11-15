# The map of swarmy

Where things are, for a reader who opens the repository for the first time (person or agent). It
names modules and functions rather than linking to lines: search for the names, the lines move.
The decisions behind the layout are in `AGENTS.md`; this file says where to look.

## Three crates, one workspace

| Crate | Directory | What it is |
|---|---|---|
| `swarmy-ui` | `src/` | The frontend: Leptos, compiled to wasm by trunk. One module per view, `src/<view>/view.rs`, the shared pieces in `src/components/`. The same frontend runs in the Tauri window and in a browser (`swarmy --serve`). |
| `swarmy` (library `swarmy_tauri_lib`) | `src-tauri/src/` | The backend: the Tauri commands and the HTTP server (`server.rs`) over the same functions, the Polars queries of the snapshot, the personal database (DuckDB), the replay scanner and watcher. |
| `swarmy-common` | `swarmy-common/src/` | The types both sides speak: the DTOs of every query, the settings, the constants of the snapshot tables (`DETAILS_IPC`...). New fields take `#[serde(default)]`. |

The snapshot is the Arrow IPC directory `<Replay Path>/ipcs` that s2protocol-rs writes: one row
per player per game in `details.arrow`, the events per game loop in the event files, the
aggregations per game (`game_*.arrow`) and per period (`*_usage.arrow`), partitioned by day.
`src-tauri/src/data.rs` scans it: `scan_snapshot_ipc`, the hot tables kept in memory, the
expression helpers (`race_name_expr`, `is_in_replays`) and the `SnapshotSource` trait that the
queries read from (`data/source.rs`: `IpcSnapshot` on disk, `MemSnapshot` in the tests).

## Modules are files named after themselves

A module is `name.rs`, its children in `name/`; there is no `mod.rs` (clippy
`mod_module_files`, and `data/contract.rs` checks it). The path of a module is then the address of
its file: `crate::map_details::recent_games` is `src-tauri/src/map_details/recent_games.rs`, on
GitHub `blob/<commit>/src-tauri/src/map_details/recent_games.rs`, and a file keeps that address
when tests or documentation grow next to it in `name/`.

## A view, backend side

`src-tauri/src/<view>.rs` holds the command of the view, a function taking the host first
(`pub async fn query_<view>(host: &dyn SwarmyHost, ...)`), listed once in `commands.rs`
(`backend_commands!`): the same list serves the Tauri handler and the routes of `--serve`, and
`web.rs` says which commands are public and which need the administrator key. The command opens
the snapshot (`IpcSnapshot::open`), runs the queries of the view and shapes the answer.

## A query is a file

Each statistic a panel shows is one query file, `src-tauri/src/<view>/<query>.rs`, with a
directory of the same name beside it:

```
src-tauri/src/map_details.rs                        the view: declares the queries, composes them
src-tauri/src/map_details/recent_games.rs           the query
src-tauri/src/map_details/recent_games/README.md    what it computes, the SQL, the example
src-tauri/src/map_details/recent_games/tests.rs     its tests, on the fixtures and on a snapshot
```

The query file has, in this order:

```rust
#![doc = include_str!("recent_games/README.md")]

pub const META: QueryMeta = query_meta!();   // module, file and line, for the debug links

pub fn plan(source: &dyn SnapshotSource, params: &Params) -> Result<LazyFrame, SwarmyError>
pub fn shape(frame: DataFrame) -> Result<Dto, SwarmyError>   // the collected frame to the DTO

#[cfg(test)]
mod tests;
```

`plan` builds the lazy query and never collects; `shape` turns the collected frame into the type
of `swarmy-common`; the view's command is the only place that collects (so that several plans can
run as one with `collect_all`). The README has the sections Purpose, Inputs, Output, Query,
Edge cases, Performance and Example; its ```sql fence is executed by the tests
(`polars::sql::SQLContext`) on the same frames as `plan` and the two results must be equal, so the
SQL cannot drift from the code. A query SQL cannot express says `SQL: not expressible` instead.
`data/contract.rs` fails `cargo test` when a query file lacks any of this.

## Tests

- `src-tauri/src/data/fixtures.rs`: snapshots of a few games written as a person would describe
  them (`Game`, `Player`, `Snapshot::source()`), with the columns and dtypes of a real snapshot
  (`snapshot_fixture_schemas_match_a_real_snapshot` compares them). The tests of a query use them
  and the README shows the same games, so that a reader recomputes the expected numbers.
- Tests on a real snapshot are `#[ignore]`, their names start with `snapshot_`, and they read
  `SWARMY_TEST_REPLAYS` (a Replay Path): `SWARMY_TEST_REPLAYS=~/SC2Replays/PRO cargo test
  --release -- --ignored snapshot_`.
- `src-tauri/benches/queries.rs`: the criterion benchmarks of the query files on a real
  snapshot, one group per view; `scripts/bench.sh` runs them (pinned threads and CPUs) and
  `scripts/bench-compare.py` compares the medians with `src-tauri/benches/baseline-corvid.json`,
  the baseline committed from corvid: a query 20 % slower fails, 10 % is noise. Run before a
  release, `--accept` after a deliberate change. `bench_queries.rs` (an ignored test printing the
  times of the views not migrated yet) stays until every view is a set of query files.

## Frontend side

`src/<view>/view.rs` is the view; `src/navigation.rs` the routes (`#/<page>?...`, `view_route`
and `open_route`); `src/common/` the calls to the backend (`invoke_json`); `src/components/` the
shared pieces (charts, links, tables, `NameSelect`, the web session). `public/web_bridge.js` makes
a browser look like Tauri to the views.

## Elsewhere

- `swarmy-common/src/`: one file per domain of types, `abilities.rs`, `player.rs`...
- `docs/`: the user documentation, `Installation.md` (the server, the browser version),
  `Development.md` (building). `src/help/view.rs` is the Help view, `src/changelog/view.rs` the
  changelog by date, both updated with each user-visible change.
- `packaging/`: the Linux service file and the Arch PKGBUILD of the server.
- `scripts/`: the sidecars (rerun, swarmy-bevy) download and release helpers.
