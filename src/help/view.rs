//! How to use swarmy and how its statistics are calculated.

use crate::components::section::Panel;
use leptos::prelude::*;

/// A term and its explanation.
type Entry = (&'static str, &'static str);

const GETTING_STARTED: [Entry; 4] = [
    (
        "1. Config",
        "Set the directory with the replays (it is searched recursively) and the directory where \
         the map caches (.s2ma files) are downloaded. The Duplicate Limbo Path, outside of the \
         replay path, is where Scan moves the duplicate replays.",
    ),
    (
        "2. Scan",
        "Counts the replays found and shows the statistics of the current snapshot. Find \
         duplicates lists the replays with the same contents as another one, by their sha256, \
         and moves them to the Duplicate Limbo Path keeping the directories they were found in. \
         Nothing is deleted.",
    ),
    (
        "3. Optimize",
        "Reads the replays and writes the snapshot, Arrow IPC files in the ipcs directory of the \
         replays, one directory per day the games were played (year=/month=/day=): details, \
         stats, upgrades, unit_born, unit_died, cmd_target_point and cmd_target_unit, plus \
         metadata.json with the s2protocol version that wrote them. Only the replays that are \
         not in the snapshot yet are read, so run it again after adding replays. Rebuild deletes \
         the snapshot and reads every replay again. The days written by another version of \
         s2protocol are parsed again by Optimize too. With \"Watch the replay path\" in Config, a \
         banner tells when there are new replays to add. The progress is shown while it runs.",
    ),
    (
        "4. Explore",
        "Names with a dotted underline open their own view: map titles, replay IDs, players, \
         units, races (the patch changes of the race) and patch versions. Clicking a table \
         header sorts the table.",
    ),
];

const TROUBLESHOOTING: [Entry; 2] = [
    (
        "Logs",
        "swarmy writes a log file per day, the last 7 are kept: in ~/.local/share/com.swarmy/logs \
         on Linux, %LOCALAPPDATA%\\com.swarmy\\logs on Windows and ~/Library/Logs/com.swarmy on \
         macOS. They are also printed when swarmy is started from a terminal.",
    ),
    (
        "Verbosity",
        "Start swarmy with -v for more logs, -vv to also log the time each step takes (the \
         setup, the plugins and each request of the views), -vvv for everything. The SWARMY_LOG \
         environment variable takes a filter instead, i.e. \
         SWARMY_LOG=info,swarmy_tauri_lib=trace. swarmy --help shows the options.",
    ),
];

const VIEWS: [Entry; 15] = [
    (
        "Map List",
        "The maps of the snapshot with their number of games and dates. The cube opens the map \
         in swarmy-bevy.",
    ),
    (
        "Map Details",
        "Games, length, matchup win rates (PvT, PvZ, TvZ, the win rate is the one of the first \
         race, clicking a matchup opens its games in the Replay List), top players and recent games. From the map cache: the minimap \
         with the mineral fields, geysers and start locations, the minerals and vespene of the \
         map, whether it has gold minerals or rich vespene, and for each start location the \
         distance to the mineral fields and geysers of its main base. The amounts are the \
         defaults of each resource type unless the map sets them. The dropdown lists every map \
         of the snapshot.",
    ),
    (
        "Replay List",
        "The games of the snapshot, ▲ marks the winners and ▼ the losers. The dropdowns filter \
         by 1v1 matchup (TvP and PvT are the same), game type, time frame, patch, tag and \
         rating, the headers sort the replays (the newest first by default). Message searches \
         the chat of the games, press Enter: the messages of the replays are read the first \
         time, then kept in the personal database. The Notes column has the rating and tags. \
         The strategy button streams the replay to the Rerun viewer, the cube opens it in \
         swarmy-bevy.",
    ),
    (
        "Replay Detail",
        "The analysis of a single replay: expansions, resources, macro, focus, economy, army at \
         10:00, unit choices, effective units, kills, build order, engagements and chat. The \
         patch version opens the Patch Overview at that patch. The expansions and upgrades show \
         how much faster or slower they were than the average of the games of the same patch in \
         the snapshot: the upgrades by when they finish, the expansions by when they start (the \
         townhall number, i.e. the second Nexus, is compared to the second Nexus of the other \
         games). Averages of fewer than 3 games are not shown. The army and bases of games \
         shorter than 10:00 are the ones at the end of the game. Load in swarmy-bevy shows the \
         replay on its map. Without a replay chosen the newest one of the snapshot is shown, \
         Previous and Next go through the replays by date, locking the map or players keeps \
         them: the previous and next replays are of the locked map and players.",
    ),
    (
        "Notes",
        "A rating from 1 to 5 stars, notes and tags for each replay, saved in the personal \
         database (swarmy.duckdb in the app data directory) by the sha256 of the replay, so they \
         are kept when the snapshot is generated again. The Replay List filters by tag and \
         rating. The database can be opened with DuckDB, i.e. from a notebook.",
    ),
    (
        "Player",
        "Every game of the player is analysed, players are identified by their name without the \
         clan tag. Results over time and per patch, units, opponents, economy against the \
         snapshot, openings, tendencies and macro. Choose the player in the dropdown, and a \
         second one in Compare with to see the preferred strategies of both side by side, what \
         each one played in the games between them and the replays of those games.",
    ),
    (
        "Map Scout",
        "Where the players send their units (commands) or lose them (deaths) in a map, oriented \
         so that the player always starts on the bottom left. Use the unit filter, i.e. SCV or \
         Overlord, and the first minutes to see the scouting patterns.",
    ),
    (
        "Patch Overview",
        "The unit properties that changed in a patch compared to the previous one.",
    ),
    (
        "Patch Evolution",
        "Lines of the costs of units, and of what they train, build, research and upgrade, over \
         the patches: minerals, vespene, supply, time, whether it exists and its number of \
         requirements. Add as many lines as needed, each one has its color. The queries can be \
         saved by name in the settings and loaded later.",
    ),
    (
        "Unit Overview",
        "A unit in every patch. ▲ red: the unit costs more, or anything was added (i.e. a \
         requirement or a unit it is weak against). ▼ green: it costs less. For properties that \
         are better the higher, i.e. life, range or a unit it is strong against, ▲ green and ▼ \
         red. ● changed. Strong vs and Weak vs have a row per unit. 〃 marks a value that did \
         not change, patches without any change of the unit are merged into one column. In the \
         snapshot: the games in which the unit was made, the units made, died and the enemy \
         units they killed, the percentage of the games of each month in which it was made, and \
         the exceptional games, the players that made at least 3 and killed two standard \
         deviations more enemy units per unit than the average.",
    ),
    (
        "swarmy-bevy replay",
        "The map is flattened and drawn in grays, every 4x4 cells have a bar per player, in the \
         color of the player, for the units died or born up to the current time (the dropdown, \
         the units died by default), units that cost resources only. The units panel lists the \
         unit types with their count, click one to see only its bars. Move in time with < > or \
         the Left/Right arrows (30 seconds), Home/End for the start/end. The players panel has \
         their color, race and result, their upgrades with the time they finished, and their \
         most effective unit and ability: the ability most of their killer units were last \
         commanded. Hover an icon for its name, click a bar for its unit types.",
    ),
    (
        "Rerun preview",
        "One entity per player and kind: units (points, hover for the unit type and tag), deaths \
         (hover for what killed them), commands (arrows), cameras (wireframe boxes) on top of \
         the map terrain, the events log and charts for the stats, unit types and unit \
         choices, zoomed to the whole game.",
    ),
    (
        "Map terrain",
        "The t3HeightMap of the map cache, each map cell corner has a level from 0 to 6. In \
         Rerun it is a triangle mesh, units are drawn on top of the highest corner of their cell.",
    ),
    (
        "Unit icons",
        "The button textures of the balance data exported from the SC2 editor, embedded in \
         s2protocol, nothing to configure.",
    ),
    (
        "Times",
        "All times are game time at Faster speed, 22.4 game loops per second, as mm:ss.",
    ),
];

const LOGIC: [Entry; 17] = [
    (
        "Unit choices",
        "When a unit is produced it is scored against the enemy army alive at that moment. For \
         each enemy unit: +1 if the new unit lists it as a strength or it lists the new unit as \
         a weakness, -1 the other way around. The score is weighted by the supply of the enemy \
         units, from -1 to 1. Above 0.05 is good, below -0.05 bad. Workers, structures and \
         armies without counters are not scored.",
    ),
    (
        "Most effective units",
        "Kills per unit is the enemy units killed by the unit type divided by the units \
         produced. The trade is the minerals and vespene of the enemy units killed divided by \
         what was spent on the unit type, above 1x the unit paid for itself.",
    ),
    (
        "Costs",
        "From the s2protocol balance data of the replay patch, or the closest one. Zerglings \
         cost half of the pair, morphs (i.e. Baneling) only cost the morph.",
    ),
    (
        "Mining time lost",
        "The minerals and vespene of each unit lost divided by the income per minute of the \
         player when it died: the time it takes to mine them back.",
    ),
    (
        "Engagements",
        "Units dying less than 12 seconds apart, at most 60 seconds long, with at least 400 \
         minerals and vespene lost. A side losing more than 1.5 times what the other lost made \
         a bad trade ▼, the other side a good one ▲.",
    ),
    (
        "Morphs",
        "A Drone dying without a killer morphed into a structure, High and Dark Templars dying \
         without a killer merged into an Archon. They are not losses.",
    ),
    (
        "Focus",
        "Every selection and command of the player by what it is about: army units; townhalls, \
         production, larva, injects, chrono boosts, MULEs and structures built (macro); workers \
         (economy); townhalls built (expand); research structures and research (upgrades). A \
         selection with an army unit is army. The longest army only time is the longest stretch \
         after 02:00 with army actions and no macro, expand or upgrade action.",
    ),
    (
        "Ahead ▲",
        "In the economy of a replay, ▲ marks the values above the best of the enemies at that \
         time.",
    ),
    (
        "Supply blocked",
        "The time with all the supply used, below 200.",
    ),
    (
        "Floating resources",
        "The time with more than 1000 minerals and vespene unspent, after the first minute.",
    ),
    (
        "Production idle",
        "For Barracks, Factories, Starports, Gateways, Robotics Facilities, Stargates and \
         townhalls: the time between the end of a unit and the start of the next one in the \
         same structure.",
    ),
    (
        "Economy",
        "Workers, supply, income and army value at 04:00, 06:00, 08:00 and 10:00, compared to \
         the average of every game of the snapshot with the same matchup.",
    ),
    (
        "Opening",
        "The first three tech structures, supply, gas, townhalls and defenses are not tech.",
    ),
    (
        "Proxy",
        "A production structure, or Spawning Pool, started more than 50 map units from the \
         start location before 04:00.",
    ),
    (
        "First scout",
        "The first command targeting less than 30 map units from the enemy start location. Not \
         shown for Zerg, the first Overlord almost always scouts.",
    ),
    (
        "Comebacks",
        "Games won after the army value was below 70% of the enemy one at some point after 05:00.",
    ),
    (
        "Base trade",
        "Games in which both sides lost at least two townhalls.",
    ),
];

fn entries(entries: &[Entry]) -> impl IntoView + use<> {
    let rows = entries
        .iter()
        .map(|(term, explanation)| {
            view! {
                <tr class="align-top">
                    <td class="pr-4 py-1 text-white whitespace-nowrap">{*term}</td>
                    <td class="py-1">{*explanation}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <table class="min-w-full text-sm text-gray-400">
            <tbody class="divide-y divide-white/5">{rows}</tbody>
        </table>
    }
}

#[component]
pub fn Help() -> impl IntoView {
    view! {
        <div class="max-w-5xl p-2 space-y-4">
            <h1 class="text-xl font-semibold text-white">"Help"</h1>
            <Panel title="Getting started">{entries(&GETTING_STARTED)}</Panel>
            <Panel title="Views">{entries(&VIEWS)}</Panel>
            <Panel title="How the statistics are calculated">{entries(&LOGIC)}</Panel>
            <Panel title="Troubleshooting">{entries(&TROUBLESHOOTING)}</Panel>
        </div>
    }
}
