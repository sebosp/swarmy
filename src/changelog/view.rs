//! The changes of swarmy by date, newest first.

use leptos::prelude::*;

/// The changes of each day, newest first.
const CHANGELOG: &[(&str, &[&str])] = &[
    (
        "2026-10-29",
        &[
            "Replay Detail: Three things to fix, what cost each player the most in the game (supply blocks, idle production, floating, missed injects, chrono boosts and MULEs, the worst fight), with an estimate in resources to compare them.",
            "Map Details: Map lessons, where the fights happen, the proxies of the first 04:00, the creep tumors and the buildings of the first 03:00 near the start (the walls), in the games of the map, and Show in swarmy-bevy draws them as bars, the keys 1 to 4 show or hide each layer.",
            "Replay Detail: Against the opponent's opening, the openings of your race that won the most against the opening of the opponent in the 1v1 games of the snapshot, with yours highlighted.",
            "Progress: with the Main Player Name of Config, the workers at 06:00, income at 08:00, seconds supply blocked and army at 10:00 of your last 200 games, against humans, computers or both, with the win rate of the last 20 games and goals drawn on the charts.",
            "Replay Detail: Economy shows next to the workers, supply, income and army of 04:00 to 10:00 the median of the players who won the same 1v1 matchup in the patch.",
            "Replay Detail: the turning points of the win chance, the minutes it changed the most with the fight behind them, and a button that opens swarmy-bevy at that moment.",
            "Replay Detail: Energy used, the injects, chrono boosts and MULEs compared to what the energy of the Queens, Nexuses and Orbital Commands allowed. Control groups and camera, the groups used, recalls and camera moves per minute.",
            "Replay Detail: a Win chance chart, who the game favors every 30 seconds from the army value, income, workers and resources killed of each side, fitted on 27,441 games. The tooltip says what each side is ahead in. It does not see the mistakes, a side ahead can still lose.",
            "Config: Browse buttons pick the Replay Path, the Map Caches Path and the Duplicate Replays Folder with the folder dialog of the system.",
        ],
    ),
    (
        "2026-10-28",
        &[
            "Replay Detail: an Abilities section with the uses of the abilities with energy or a cooldown, i.e. Abduct, Fungal Growth, Neural Parasite, Transfuse, Inject Larva or Yamato Cannon, compared to the average of the patch, with the rank of the game and of the player and a top 10 badge.",
            "Replay Detail: the injects, chrono boosts and MULEs of Macro were always 0.",
            "Patch Overview: the abilities commanded in the games of the patch, per game, in how many games and the top players. Patch Evolution: the ability use per game of a race over the patches.",
            "Race Detail: the matchups and patches of a race, the maps it wins the most on, the units it makes, its abilities and its players with the most games. The race links open it.",
            "Map Details: the Siege Tank spots, where the tanks sieged and the enemy units they killed there, Show in swarmy-bevy draws them as bars on the terrain.",
            "Tour: a guided tour of the configuration and of every view. Home shows the replay and map caches directories, or that the replay directory is not set yet.",
            "Scan shows the Replay Path of Config, it is no longer an input that was not saved.",
            "Config: the map caches go to swarmy-caches in the home directory by default, the replays are in Documents\\StarCraft II\\Accounts by default on Windows. With the watcher on, a message can announce each new replay.",
        ],
    ),
    (
        "2026-10-27",
        &[
            "s2protocol 4.0.0: the units selected and in control groups are tracked when they die or are removed from a selection, the commands of a selection go to the right units. Optimize parses the replays again, the snapshot of another s2protocol version is rebuilt.",
            "Config: the explanation of the Duplicate Replays Folder is in the amber tooltip of its question mark.",
        ],
    ),
    (
        "2026-10-26",
        &[
            "Faster start: the frontend is built in release mode, it was a 160 MB debug build that took seconds to load.",
            "Optimize shows its progress right away, it was only shown after the headers of all the new replays were read, a long wait the first time a directory is optimized.",
            "Logs: a log file per day in the logs directory of the app, -v, -vv and -vvv for more logs and the time each step takes, or a filter in SWARMY_LOG. See Troubleshooting in Help.",
            "Replay Detail: without a replay chosen the newest one is shown. Previous and Next go through the replays by date, the map and players can be locked so that they are of the same map and players.",
            "Replay Detail: notes, a 5 stars rating and tags for each replay, kept in a personal DuckDB database by the sha256 of the replay. The Replay List filters by tag and rating and shows them.",
            "Replay List: search the chat messages of the games.",
            "Scan: find the duplicate replays and move them to the Duplicate Replays Folder of Config, keeping their directories. Nothing is deleted.",
            "Patch Evolution: lines of the costs of units and of what they train, build, research and upgrade over the patches, the queries can be saved.",
            "The unit icons are embedded, the Balance Data Path is no longer needed.",
            "swarmy-bevy: the bars of the units died or born, chosen in a dropdown, in the color of each player, a units panel to show the bars of one unit type, the map title at the top, clearer options and their help when started without a map.",
        ],
    ),
    (
        "2026-10-25",
        &[
            "Optimize no longer shows the snapshot statistics as an error, the new replays banner ignores the copies of another replay that Optimize skips.",
            "Replay List: the games of the same map are all listed, they showed the first one repeated.",
            "Replay Detail fits in the window, the long games made it wider than the window.",
            "Config: the minimum and maximum patch versions are 0, no filter, by default.",
            "README: a walkthrough video and screenshots of every view.",
            "Replay List: filter by matchup (TvP is the same as PvT), game type (1v1 to 4v4), time frame and patch. The headers sort the replays, the newest first by default, and the duration of each game is shown.",
            "Map Details: clicking a Race vs race row opens the Replay List with the games of that map and matchup.",
            "Every table can be sorted by clicking its headers, ascending then descending.",
            "Config: watch the replay path for new replays. A banner tells how many replays are not in the snapshot, Rebuild only parses those and the ones parsed by another version of s2protocol.",
            "Map List: the buttons column is named Actions.",
            "Replay Detail: the army and bases of games shorter than 10:00 are the ones at the end of the game, they were empty.",
            "Charts drawn with ApexCharts as the rust-ui charts, with tooltips: games and wins, unit use per month, rating, commands per minute and focus.",
            "swarmy-bevy: the bars of each player in their color, the units lost in a darker shade, on gray terrain.",
            "Home: a shorter welcome, where the data comes from and the projects swarmy is based on. The changes moved to this Changelog.",
            "Matchups are written as the casters do: PvT, PvZ, TvZ, mirrors as ZvZ.",
            "Replay List and Replay Detail: load a replay in swarmy-bevy, the units born and died over time on its map with the players, their upgrades and most effective unit and ability.",
            "Unit Overview: the unit in the snapshot, units made, died and killed, use per month and the games in which it was exceptionally effective. The unit is chosen from the dropdown only.",
            "Rerun preview: hovering a unit shows its type, hovering a death what killed it.",
            "Optimize only reads the replays that are not in the snapshot yet, the snapshot is partitioned by day. Rebuild generates it from scratch.",
            "Optimize: older replays (2021) are no longer skipped, and the copies of a replay are reported.",
            "Map Details: the minimap of the map cache with the resources and start locations, the minerals and vespene of the map, gold and rich vespene, and the mining distances of each main base.",
            "Replay Detail: each expansion and upgrade shows how much faster or slower it was than the average of the same patch.",
        ],
    ),
    (
        "2026-10-24",
        &[
            "Player: choose players from a dropdown and compare two of them, their preferred strategies, what each one played against the other and the replays between them. Map Details has a map dropdown.",
            "Unit Overview: a row per unit in Strong vs and Weak vs, patches without changes merged into one column.",
            "Replay Detail: sections aligned in columns, player focus (what each player selects and commands minute by minute) and the longest time with only the army.",
            "Replay Detail: ▲ marks who is ahead in workers, supply, income and army at each economy time, the commands chart has axes shared by both players.",
            "Optimize shows its progress: files found, each step, the replays processed and the size of each file written.",
            "Unit button icons from the balance data textures, next to every unit name and as quick filters in Map Scout.",
            "Names that open another view have a dotted underline: players, maps, replays, units, races and patch versions.",
            "Replay List: ▲ marks the winners and ▼ the losers in the player list.",
            "Drones morphing into structures and templars merging into Archons are no longer counted as losses.",
            "Rerun 0.40, the viewer runs as the rerun-cli sidecar; units on top of the terrain, camera boxes, economy and unit type charts.",
        ],
    ),
    (
        "2026-10-09",
        &["Replays have a sequential id (ext_fs_id), the clan tags of the player names are decoded."],
    ),
    (
        "2026-10-06",
        &["swarmy-bevy runs as a sidecar bundled with swarmy."],
    ),
    (
        "2026-10-02",
        &["The slow operations (scan, optimize, cache downloads) run in the background."],
    ),
    (
        "2026-06-07",
        &["The map caches are downloaded once, to a global location shared by the snapshots."],
    ),
    (
        "2026-05-24",
        &["Replays can be previewed in Rerun."],
    ),
    (
        "2026-05-23",
        &["Replay List, with buttons to copy the path of a replay and open its directory."],
    ),
    (
        "2026-05-12",
        &["The map height can be explored in 3D with swarmy-bevy."],
    ),
    (
        "2026-02-23",
        &["The map caches are downloaded from the Blizzard depots, with their size and count."],
    ),
];

#[component]
pub fn Changelog() -> impl IntoView {
    view! {
        <div class="max-w-3xl p-4 space-y-4">
            <h1 class="text-2xl font-semibold text-white">"Changelog"</h1>
            {CHANGELOG
                .iter()
                .map(|(date, changes)| view! {
                    <div>
                        <h2 class="text-base/7 font-semibold text-white">{*date}</h2>
                        <ul class="mt-1 list-disc list-inside text-sm text-gray-400">
                            {changes.iter().map(|change| view! { <li>{*change}</li> }).collect_view()}
                        </ul>
                    </div>
                })
                .collect_view()}
        </div>
    }
}
