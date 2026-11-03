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

### Charts
- ApexCharts through `src/components/charts.rs` and `public/charts/chart_init.js`. One series in
  the first chart color; a series compared with an average uses `var(--chart-muted)` for the
  average. Every point has a tooltip in words.
- A two-sided measure (the win chance) is one line around the middle (`AdvantageChart`), not two
  mirrored lines.
- The hovered column is outlined, never filled, so that no bar disappears in any theme. Charts
  animate only when they appear.

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
