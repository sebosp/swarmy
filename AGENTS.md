# Working on swarmy

Instructions for the coding agents (Claude Code reads them through `CLAUDE.md`) and the people
working on swarmy: the decisions taken with the author, so that they are not asked again or
undone. Add to this file when a new decision is taken; `docs/Development.md` has the setup.

swarmy is a StarCraft II replay batch analyser: a Tauri app with a Leptos (wasm) frontend in `src`,
the backend in `src-tauri` (Polars over the Arrow snapshot that s2protocol-rs writes), the types
shared by both in `swarmy-common`. `swarmy --serve` runs the backend alone, as the remote backend of
other swarmy apps. swarmy-bevy, a separate repository, draws the maps and replays in 3D.

## Skills to load

Always, for any change a user sees:

- **frontend-design**: the view must be clear to a player, not too much information at once,
  the data first and the words quiet.
- **dataviz**: every chart, color, stat line, legend or table of statistics, presented as the
  best practices for statistics say (one message per chart, honest scales, labels in words).
- **deep-research** on learning platforms (chess, aim trainers, coaching tools) when a feature
  is about learning: swarmy is a learning platform for StarCraft II players.

Every change checks, without a skill to load:

- Accessibility: contrast in every theme, color never the only cue, safe for color vision
  deficiencies (run the dataviz validator for new colors), keyboard focus, titles on icons.
- The colors of the theme chosen in Config: success, warning and error are the daisyUI
  `text-success`, `text-warning`, `text-error` of the theme, never a fixed green or red.

## Browser version

- `swarmy --serve` serves the same frontend as the app: `public/web_bridge.js` defines
  `window.__TAURI__` in a browser, the views do not know. Every command of the frontend is in the
  server or in the bridge, and every command of the backend is in `ADMIN_COMMANDS` or
  `PUBLIC_COMMANDS` of `src-tauri/src/web.rs`: tests of that file fail otherwise.
- The administrator key: the server keeps its sha512 in a file read on each request, so that
  rotating it needs no restart. What needs it is refused by the server first, hiding it in the views
  (`components/web_session.rs`) comes second. Decided with the author on 2026-11-06.

- The address of the window follows the view (`navigation::sync_with_address`, `#/<page>?...`):
  the views can be shared from the address bar and the app opens such a link (Home, Open a
  link); no copy button (removed at the author's request). A new view or what a
  view shows goes in `view_route` and `open_route`. Decided with the author on 2026-11-08.
- In the browser the zoom is the browser's own (no Zoom setting, no Ctrl+= handling), Config
  always asks for the key (the server never tells whether it has one), the buttons that start
  Rerun or swarmy-bevy have the `native-only` class and `native_title`.
- The servers of the author use one random administrator key, kept with sops in
  `~/git/linuxtweaks` (`secrets/swarmy.enc.yaml`, decided 2026-11-09, "swarmy" before); the app
  sends it as the token of its Remote Backend. wormgat's 7878 and 7879 are on the internet as
  `swarmy.nl` and `arena.swarmy.nl` through a Cloudflare Tunnel.
- The access log and the metrics of the server (`src-tauri/src/access_log.rs`) measure the views
  and the queries, never the visitors: no addresses (a visitor id salted at each start), the
  arguments of the public commands only, `admin` to leave the author's own use out. The browser
  version says its view in the header `X-Swarmy-View` (`public/web_bridge.js`); `/metrics` needs
  the administrator key. Decided with the author on 2026-11-09.

- Battle.net (`src-tauri/src/battlenet.rs`): the StarCraft II community API with the client
  credentials flow (no redirect). The Client ID and Secret are set in Config of the app, kept in
  `battlenet.json` of the data directory of the backend (mode 600) and never sent back to the
  views; `SWARMY_BATTLENET_CLIENT_ID`/`_SECRET` replace them. A server takes them from its
  environment or data directory only: the browser version has no Battle.net section in Config and
  the server refuses `get_battlenet_status`/`set_battlenet_credentials` even with the
  administrator key (`web::DESKTOP_COMMANDS`, 2026-11-11). Never commit or log them. The profiles
  are kept 30 minutes. Decided with the author on 2026-11-08.

- Watch in StarCraft II (Replay Detail, `src-tauri/src/sc2_game.rs`, decided with the author on
  2026-11-21): the app plays the replay in the game itself, offline (no Battle.net, no login),
  through the API of the game (`-listen`, `RequestStartReplay` with real time off, fog off). On Windows
  the game of `sc2_game_dir` is started directly; on Linux the Windows game of the Wine prefix
  (`sc2_wine_prefix`, `~/Games/battlenet` by default) through `umu-run` and a Proton. The version
  of the replay must be installed (Battle.net downloads it when a replay of it is opened once).
  Never touch Battle.net, its launch options or the account (fear of a ban). Desktop app only,
  the browser version dims the button.
- The replay played in StarCraft II (2026-11-21, after the author's tests): swarmy steps it
  (real time off) at the speed chosen, the controls are in the overlay (play/pause, the speed, a
  time slider; a jump back starts the replay again). The game's own controls do not work while
  the API drives a replay: in real time its pause stuck and its timeline crashed the game
  ("e_success"), stepped too. The game refuses to draw during a replay ("Not allowed during
  replays"): the overlay is a small window of swarmy, always on top, hidden until the game plays,
  480×280 at first and resized with the mouse (its edges and a grip at the bottom left, the size
  kept when collapsed to the button and given back when opened, the window stays resizable; the win chance chart a quarter of the height,
  the rest for the upgrades and kills), dragged where the player wants it and collapsed to a thin button
  at its right (» and «), a dark plate whatever the theme (`#/sc2-overlay`, mounted alone by
  `main.rs`); the window of swarmy is behind the game, it has no controls. It shows only what
  happened until the moment of the game, no spoiler: the win chance until now (the points of
  Replay Detail, the same percentages; the first team above, the second below, a band per player
  of a team), and for 20 seconds of the game an upgrade when it finishes (its time against the
  patch average in the dataviz status colors and words, "5s faster", "12s slower") and the first
  kill of a player. A full screen click-through window was tried first and was invisible on KDE
  Wayland, taking the clicks of swarmy. Next: a "Top 1%" rank of an upgrade (needs the
  distribution of its times, a new query file), OBS as a Browser Source.

## Snapshot data

- The views that span games read the aggregations of s2protocol (`game_*.arrow`, the usage tables
  per day, month and year with `scan_usage`), never the event files. `details` and the `game_*`
  tables are kept in memory until the snapshot changes (`HOT_TABLES`).
- The Player view reads `swarmy_player_games.arrow` of each day, written by Optimize from the
  files of the day (`player_detail::snapshot_insights`), a typed column per field. A new field of
  `PlayerGameInsight` bumps `INSIGHTS_VERSION`, the days are computed again on the next Optimize.
- The views never read the replays to answer a query (decided with the author on 2026-11-08): what
  they need from the games is exported by s2protocol-rs, a missing fact is added there. Reading a
  replay is for finding the files missing from the snapshot and for playing one game (Rerun,
  swarmy-bevy). The definitions of the Player view follow the replay analysis
  (`replay_detail::data`); `it_matches_the_analysis_of_the_replays` compares them on a snapshot.
- Missing aggregations are told by the watcher (`missing_aggregates`, checked every 5 minutes) and
  computed by Optimize, the views never compute them themselves.

## After a change of behavior

Update in the same change, briefly (a line, not a log): the screenshots (`docs/screenshots`),
the docs (`README.md`, `docs/`), `src/changelog/view.rs`, `src/help/view.rs` for a new view or
statistic, and the wiki when it describes the changed view. Agents: write the decisions you take
with the author here, explicitly, so that the next agent follows them.

## Interface decisions

### Data first
- The data is the largest and clearest text of a view; headings and labels are smaller and
  quieter. A summary is a sentence at the size of the text, its key counts in `text-primary` (Map
  List: "757 maps, 36,394 games").
- In a sentence or a cell mixing numbers and words, the numbers stand out (`font-medium
  text-base-content tabular-nums`), the words are muted.
- No redundant data: what follows from what is shown is not shown again (the chance of the
  second player is the rest of the first one's, the losses are the games minus the wins, the map
  and players are not repeated in two panes).
- Counts instead of words: `2`, not "Yes, 2 rich geysers"; `-` when there are none.
- Ties are shown only when there are any, in yellow (`text-warning`); wins are green
  (`text-success`); losses and worse values are the red of `text-error` with ▼, better values the
  green with ▲. Never another red or green.

### Numbers and tables
- Number cells and their headers use the `num` utility of `styles.css` (right aligned, digits of
  one width); every table cell has tabular digits.
- Positions on a map are written `(x, y)` everywhere, with the `MapCell` component: hovering it
  highlights the spot on the map of the view.
- A table shows at most the Rows per table of Config (100 by default) at once: `table_sort.js`
  pages every table, the Previous and Next footer only when there are more rows. The tables that
  can have thousands of rows (Player matches and opponents) are paged by the view
  (`components/paged.rs`, `data-server-sort`), so that the webview never draws them all. A list
  of thousands of names is a search field with suggestions, never a `<select>`.
- Tables are sortable (`public/table_sort.js`), a sortable header shows ↕, the sorted one ▲ or ▼.
  The default sort is the most useful one, usually the games, and it is shown as sorted. The sort
  the viewer picks is remembered on the computer (webview storage), also the Replay List one.
- Winners first in the lists of players of a game.
- "Matchup", never "Race vs race"; mirror matchups are left out of win rates (they always have a
  winner and a loser); "First race favored" for the win rate of the first race.

### Layout and controls
- Sections and panels (`Section`, `Panel`, the bands of Replay Detail) have an arrow that
  collapses them; the sections remember it by their title.
- Filters on one line: "Search:" then fields with placeholders ("By name", "By player"); a label
  beside its select, not above it.
- Secondary actions are icon buttons with a `title` saying what they do. Opening in Rerun or
  swarmy-bevy uses their logos, the `BrandIcon` component (a mask in the color of the text).
- While data loads, a `Loading` message says what is loaded ("Analysing replay 1234 on Pylon:
  A vs B..."); the previous data is grayed out and does not react to the mouse; a form is not
  shown with its defaults before its values arrive.
- Buttons that would only ask the backend for nothing are disabled (Next on the latest replay).
- Long views keep what the viewer needs in view: the map of Map Details is sticky; Replay Detail
  has a gutter between the players to scroll the page without scrolling a section.
- Views whose value is not proven yet go to an Experimental group of the menu, under a dashed line
  (none since Map Scout was removed on 2026-11-12). The pages read once (Tour, Help, Changelog) are at the bottom of the menu, Config is with Scan.
- Ctrl+= / Ctrl+- / Ctrl+0 zoom; the themes use the daisyUI semantic colors and the `--chart-*`
  variables, never hard-coded colors (except the map resources, checked against each other).

- The rank badges (abilities, Replay List) use the `--rank-*` colors of styles.css, validated
  with the dataviz script for light and dark themes: top 100 blue, top 10 green, top 5 gold, the
  best purple, and say their rank in words. A tier shows only when there are at least twice as
  many others (`RankBadge`).
- To pick a unit or an ability, the race first (`RaceButtons`, as in Race Detail), then a field
  with the units or abilities of that race only.
- A list of names (players, maps) is the `NameSelect` combobox, not a `<datalist>`: the webview
  of the app on Linux does not show datalist suggestions.

- Replay Detail shows the replay first and its comparisons with the snapshot after (2026-11-21,
  the author's idea): `query_replay_detail_quick` answers the replay and its map (about 0.5 s),
  `query_replay_detail` adds the patch averages, winners' medians, openings, abilities, creep and
  energy, computed over every game the first time a process asks (4.3 s on 36k games), then from
  memory. Their panes show a small spinner (`Comparing`) meanwhile; only the players' sections
  are drawn again when they come.

### Charts
- ApexCharts through `src/components/charts.rs` and `public/charts/chart_init.js`. One series in
  the first chart color; a series compared with an average uses `var(--chart-muted)` for the
  average. Every point has a tooltip in words.
- A two-sided measure (the win chance) is one line around the middle (`AdvantageChart`), not two
  mirrored lines.
- The hovered column is outlined, never filled, so that no bar disappears in any theme. Charts
  animate only when they appear.
- A chart has at most a few hundred points: ApexCharts measures each label with the styles of
  the whole page, a chart of a point per game (3,771 for a player) blocked Firefox for a minute
  and slowed every other chart of the page. Aggregate by day, month or year (`rating_periods`).
- A chart of several kinds can filter them from its own legend, as Grafana does (a click shows
  only that kind, a click on another adds it): the Focus of Replay Detail (`without_legend`),
  its tooltip is written by the view in the colors of the bars.

### Remote backend
- Every view says whether the snapshot is on this computer or on a remote backend, and which
  one ("Remote Replay Path on wormgat-zt:7878", the host links to Config).
- With a remote backend the settings of the server (Map Caches Path, duplicates, scan limits) are
  not shown: they are set on that computer. Actions of the server's files (Find duplicates) are
  not offered.
- New fields of the shared types have `#[serde(default)]`, so that an app and a server of
  different versions still talk; the app and its servers are nevertheless updated together.
- swarmy-bevy reads local files: with a remote backend the replay and its map caches are copied
  from the server first (`GET /api/files/...`, read-only, inside the server's paths only).

### Words
- Plain words, sentence case, active voice. Say what happened and what to do ("The server runs an
  older swarmy that does not send its replay path, update it to 0.2.3 or later").
- Optimize creates the snapshot and Refresh adds the new replays (the same command); Rebuild from
  scratch deletes it and reads all the replays again.

### Players
- A player is its accounts (`region-realm-id` of the toon handle), not a name: an account keeps
  its games when its name or clan tag changes, the accounts with the same main name are one player,
  and the accounts grouped under a player in the personal database (Player view, Accounts) count as
  that player. Queries by player use `data::details_with_players` and its `player` column.
- The arcade and co-op races (Primal Zerg, the commanders) are not StarCraft II races: their
  players are left out; the units without a race are Neutral.
- The replay ids follow the time of the games (s2protocol-rs), a Rebuild numbers them again.

## Code

`ARCHITECTURE.md` says where things are; the rules below say how they are written.

### Modules
- Edition 2024 in the three crates. A module is `name.rs` with its children in `name/`, never
  `mod.rs`: the path of a module is the address of its file, on disk and on GitHub, and the file
  keeps it when tests and documentation grow beside it. Enforced by clippy `mod_module_files`
  (workspace lints) and by `data::contract::no_module_is_a_mod_rs`. Decided with the author on
  2026-11-10, after the research in `reports/Query module layout for swarmy.md`.

### A query is a file
Decided with the author on 2026-11-10, the reasons in `reports/Query module layout for swarmy.md`.
A statistic shown by a panel, a table or a chart is one query, and a query is one file with a
directory of its name beside it:

```
src-tauri/src/<view>/<query>.rs            the query: META, plan, shape, `mod tests;`
src-tauri/src/<view>/<query>/README.md     Purpose, Inputs, Output, Query, Edge cases, Performance, Example
src-tauri/src/<view>/<query>/tests.rs      the tests
```

- The file starts with `#![doc = include_str!("<query>/README.md")]` (the README is its rustdoc),
  then `pub const META: QueryMeta = query_meta!();` (the module, file and line the compiler saw,
  for the "Show Polars Debug Information" links and the contract test).
- `pub fn plan(source: &dyn SnapshotSource, ...) -> Result<LazyFrame, SwarmyError>` builds the
  lazy query from `source.table(DETAILS_IPC)` and the other tables, never from a path and never
  collecting; `pub fn shape(frame: DataFrame) -> Result<Dto, SwarmyError>` turns the collected
  frame into the type of `swarmy-common`. The view's command (`src-tauri/src/<view>.rs`) opens
  `IpcSnapshot`, collects the plans (several as one with `collect_all`) and shapes.
- The README is for a person who knows SQL and the game, not the code: what the panel answers
  (Purpose), the tables and columns read (Inputs), a table of the output columns with dtype and
  meaning (Output), the SQL in a ```sql fence (Query), what is left out and why (Edge cases:
  observers, computer players, ties, arcade races, nulls), which files are scanned and what is
  pushed down (Performance), and the games of the fixture with the expected answer (Example).
- The SQL of the fence is executed by the tests with `polars::sql::SQLContext` on the same
  fixture as `plan`, and `polars_testing::assert_dataframe_equal!` compares the two (row order
  off, dtypes on): the SQL cannot drift from the code. A query SQL cannot express (list and
  struct columns, `group_by_dynamic`, `top_k`, `cut`) writes `SQL: not expressible` in the Query
  section and the test asserts a hand-written expected frame.
- The tests run on `data::fixtures`: a few games described as a person would (`Game`, `Player`,
  `Snapshot::new().game(..).source()`), with the exact columns and dtypes of a real snapshot, so
  that the expected numbers are recomputed by reading the games; the arithmetic goes in a comment
  (`// Alice: 2 games, 1 win -> 50 %`). Named by behaviour
  (`two_games_one_win_gives_fifty_percent`), no `test_` prefix. A test on a real snapshot is
  `#[ignore]`, named `snapshot_...`, reads `SWARMY_TEST_REPLAYS`.
- Literals follow the column: `lit(1u64)` against `ext_fs_id`, never a bare `lit(1)` (Int32).
  The SQL side types too: `COUNT(*)` is u32 like `len()`, `SUM(CASE WHEN .. THEN 1 ELSE 0 END)`
  is Int32, so the plan casts its `.sum()` of a boolean to Int32. Sort on a unique key before
  taking rows; `group_by` order is not stable.
- `data::contract::every_query_file_has_its_readme_and_tests` fails `cargo test` when a query file
  misses any of this, or a README has no query file.
- Every query file has a bench in `src-tauri/benches/queries.rs` (its view's group, the heaviest
  parameter: the most played map or player). Before a release `scripts/bench.sh` on corvid
  compares the medians with `src-tauri/benches/baseline-corvid.json`: a query 20 % slower than
  the baseline blocks the release until it is fixed or the new time is accepted
  (`scripts/bench.sh --accept`, the reason in the commit). The runs are appended to
  `~/ci-runs/bench-history.jsonl` on corvid. Decided with the author on 2026-11-10.
- The view's command names its queries in the answer, `ApiResponseBuilder::new()
  .with_sources(QUERIES.map(QuerySource::from))`, and the frontend wraps each table, field or
  panel that shows a query in `<Debuggable query="map_details::matchups"
  view_source=crate::source!()>` (`components/debug.rs`): with Show Polars Debug Information on
  (Config, saved per computer or browser) a line above the content names the query and its file
  in `<code>` with a bug icon and a Copy link button for the GitHub address of the query file, at
  the commit of the backend (`AppVersions::git_sha`, from `build.rs`; the `develop` branch when
  the build had uncommitted changes). A line, never a button floating over the content: it
  covered the sort arrows of the headers (the author, 2026-11-10). Take what a `Debuggable` block
  shows out of the view's data before the `view!`, its children are a closure. Decided with the
  author on 2026-11-10.
- The views are migrated one by one; Map Details was the first (2026-11-10:
  `map_details/{games_summary,matchups,top_players,recent_games,maps}.rs`, the model to copy),
  then Player (`player_detail/{player_rows,players,game_results,matches,opponents,
  unit_effectiveness,accounts,similar_accounts}.rs`; `head_to_head` and `insights` read the
  analysis of the games and are not query files yet) and Replay List
  (`replay_list/{games,protocol_versions,years}.rs`; the notes, tags and messages come from the
  personal database in the command), Progress (`progress/{games,metrics}.rs`, the one view that
  reads an event file, `stats.arrow`, for the supply blocked) and Race Detail
  (`race_detail/{player_games,matchups,by_patch,maps,top_players,units}.rs` on
  `data::snapshot_players_of`; the abilities come from the ability usage), Map List
  (`map_stats/maps.rs`, run through the majordomo), the charts over time
  (`usage_over_time/{race_games,ability_uses,unit_uses,unit_race,patch_starts}.rs`, read from
  the usage tables of the aggregations through `SnapshotSource::usage(grain, table)`; the
  fixtures build the usage tables of the three grains from the games, as
  `s2protocol::arrow_store::aggregates` does, and the SQL names them `month_player_usage`) and
  the snapshot panel of the Unit Overview (`patch_overview/unit_stats/{games,totals,by_month,
  kills_per_unit,exceptional}.rs`; `exceptional` takes the threshold of `kills_per_unit` as a
  scalar parameter, a literal in the README SQL), the Ability view (`ability_detail/
  player_games.rs`: one frame of every game of the race with its uses, `aggregate` in
  `ability_detail.rs` adds it up in Rust, tested apart) and the Home summary
  (`snapshot_stats/summary.rs`; the Home table has no debug line, a line cannot sit in a row);
  the economy benchmarks of the Player insights are `player_detail/benchmarks.rs` (the
  `game_minutes` fixture table, `Player.minutes`), cached per snapshot in `insights.rs`;
  `valued_units` (`replay_detail/valued_units.rs`) is a frame parameter as `race_units`; the
  fixtures have `unit_born`, `unit_died` and `cmd_target_point` with the columns the queries
  read, from `Player.start`, `.commands` and `.deaths` (Map Scout and the map lessons of Map
  Details were removed on 2026-11-12); the ability usage
  of the snapshot (Patch Overview, the ability ranks of a replay) is
  `ability_usage/{game_uses,totals,by_player}.rs`, the three plans run together with
  `collect_all` once per snapshot (`tracked_abilities` is a frame parameter); the annotations
  of a replay from the snapshot are `replay_detail/patch_timings/{games,upgrade_averages,
  expansion_averages}.rs`, `replay_detail/winner_benchmarks/medians.rs` and
  `replay_detail/opening_answers/answers.rs` (`tech_structures` is a frame parameter; the
  fixtures have `upgrades`, `unit_born.control_player_id` and `Player.born`/`.upgrades`), kept
  in memory per snapshot generation as before; the map of a replay (`replay_detail/map.rs`, not
  a query: it loads the map cache as `map_details::map_picture`) runs `replay_detail/bases.rs`
  (the townhalls of the players, `Born::at` places a fixture unit) and
  `replay_detail/epicenters.rs` (the center and root mean square spread of the deaths of each
  engagement; the engagement windows of the analysis are the frame parameter
  `engagement_windows`, `valued_units` the other) and `replay_detail/deaths.rs` (where and when
  the valued units died) with `replay_detail/death_density.rs` on top of it (the kernel density
  estimate of the Deaths mode of the map, bandwidth, binning, kernel, floor and rank classes all
  in the Polars plan; `kernel_offsets` is a frame parameter, the playable area of the map a
  scalar one; the view only tints the cells and runs marching squares for the contour lines).
  Every aggregation of the views is a Polars plan, the frontend only draws (decided 2026-11-11).
  The forms of a unit (`patch_overview/unit_stats/forms.rs`) are the unit type changes
  (`unit_born` rows of the same tag with another type), a static table per patch
  (`forms/unit_forms.json`, compiled in) generated off-line by `write_unit_forms` over a large
  snapshot one day at a time and validated against the patch's balance data (decided 2026-11-12:
  the morphs change only with a patch; the whole-snapshot sort took 13 GB), the Unit Overview
  takes the pairs of the newest patch connected to its unit (`forms_of`); the upgrade dependency tree is the `requires` of the upgrade levels of the balance
  data (`patch_overview::data::upgrade_trees`). The picture of a map is one component,
  `src/components/map_picture.rs` (`swarmy_common::MapPicture`, `MapFrame` for the marks,
  `base_opacity` to fade it), Map Details and the Replay view draw their marks over it and
  highlight the cells hovered (`MapCell`, `MapSpots` for several, `round` for a radius). The
  Replay map has three modes (`src/replay_detail/map_modes.rs`): Bases, Deaths (the cells
  of `replay_detail::death_density`, six classes of ColorBrewer YlOrBr with marching-squares
  contour lines, the minimap at 0.35) and Over time
  (the fights as discs of their spread, viridis by start time in 5 or 7 classes, 0.3 fill,
  painted in time order); decided with the author on 2026-11-11 after a research of the
  cartographic conventions (`docs/research/replay-map-density-and-time.md`: no rainbow ramps,
  time is ordered, 3 to 7 classes, a floor at 5 % of the peak, fade the base map, one legend). Not query files: `replay_detail/creep.rs`,
  `player_detail/{snapshot_insights,game_insights,head_to_head}.rs` (they read the day files or
  the replays themselves), the balance data views (Patch Overview units, Patch Evolution). A parameter that is a
  frame computed by swarmy (the patch of each protocol version, the army units of a race) is a
  table of the fixtures for the SQL (`patch_versions`, `race_units`), registered with
  `assert_sql_matches_source`. A query with optional filters (the Replay List) documents
  one concrete set of filters in its SQL and the same set in its test; the other filters are
  prose. Frames with list columns compare in order (`assert_sql_matches_plan_in_order`), they
  cannot be sorted; `ARRAY_AGG(x ORDER BY x)` and `ARRAY_TO_STRING` work in polars-sql.
  Until a view is migrated its `try_query_*` functions stay as they are. A new statistic is
  written as a query file from the start.
- The SQL dialect of polars-sql, as the Map Details READMEs use it: CTEs, `ROW_NUMBER() OVER
  (PARTITION BY .. ORDER BY ..)`, `COUNT(*)`, `COUNT(DISTINCT x)`, `MIN`/`MAX`/`AVG`/`SUM(CASE ..)`,
  `STRFTIME(ts, '%Y-%m-%d')`, `CONCAT`, `LEFT JOIN .. ON`, `LIMIT`, `SPLIT_PART`, `STRPOS`,
  `COALESCE`, `CAST(x AS VARCHAR)`, `SUM(..) OVER (PARTITION BY ..)`. A self-join of a CTE on a
  window column returned no rows (2026-11-10): aggregate per group with `MIN`/`MAX`/`SUM(CASE
  WHEN side = 1 ..)` instead. A window `ORDER BY` cannot mix directions ("OVER does not (yet)
  support mixed asc/desc"): write `ORDER BY 0 - CAST(games AS BIGINT), name` for games
  descending then name ascending. A view's queries share a base plan (`player_rows` for Player):
  the base is a query file too, and each README repeats its CTEs so that its SQL runs alone.

### Commands
- A backend command of the snapshot is a function of its module that takes the host first, listed
  once in `src-tauri/src/commands.rs`: it then runs in the app and in `swarmy --serve`.
- Expensive aggregates of the snapshot are computed once per snapshot generation and cached (see
  `ability_usage_cached`, `creep_stats_cached`).
- Tests: unit tests next to the code (`mod tests` in the file, or `<name>/tests.rs` for a query);
  queries of a real snapshot are `#[ignore]` tests named `snapshot_...`, run with
  `SWARMY_TEST_REPLAYS=<replay path> cargo test --release -- --ignored --nocapture`.
  `docs/query-readme-template.md` is the README of a new query.
- Each user-visible change gets a line in `src/changelog/view.rs` (by date) and, for a new view or
  statistic, in `src/help/view.rs`.
- `cargo fmt --all` and no warnings in `cargo check` of both the wasm frontend and the backend.
- Commit messages say what changed for the user and why, in full sentences.
