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
- The servers use the key "swarmy" for now (decided 2026-11-08), see `~/git/linuxtweaks`; the
  app sends it as the token of its Remote Backend.

- Battle.net (`src-tauri/src/battlenet.rs`): the StarCraft II community API with the client
  credentials flow (no redirect). The Client ID and Secret are set in Config by the administrator,
  kept in `battlenet.json` of the data directory of the backend (mode 600) and never sent back to
  the views; `SWARMY_BATTLENET_CLIENT_ID`/`_SECRET` replace them. Never commit or log them. The
  profiles are kept 30 minutes. Decided with the author on 2026-11-08.

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
- Views whose value is not proven yet go to the Experimental group of the menu (Map Scout). The
  pages read once (Tour, Help, Changelog) are at the bottom of the menu, Config is with Scan.
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

- A backend command of the snapshot is a function of its module that takes the host first, listed
  once in `src-tauri/src/commands.rs`: it then runs in the app and in `swarmy --serve`.
- Expensive aggregates of the snapshot are computed once per snapshot generation and cached (see
  `ability_usage_cached`, `creep_stats_cached`).
- Tests: unit tests next to the code; queries of a real snapshot are `#[ignore]` tests run with
  `SWARMY_TEST_REPLAY_PATH=<replay path> cargo test --release -- --ignored --nocapture`.
- Each user-visible change gets a line in `src/changelog/view.rs` (by date) and, for a new view or
  statistic, in `src/help/view.rs`.
- `cargo fmt --all` and no warnings in `cargo check` of both the wasm frontend and the backend.
- Commit messages say what changed for the user and why, in full sentences.
