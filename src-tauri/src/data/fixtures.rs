//! Snapshots made by hand for the tests of the queries: a few games described as a person would
//! ("Alice, Terran, won against Bob"), written as the tables of the snapshot with the columns and
//! dtypes of the real one. A test's expected numbers can then be recomputed by reading its games;
//! the README of each query shows the same games as its example. See AGENTS.md, "A query is a
//! file".
//!
//! ```ignore
//! let snapshot = Snapshot::new()
//!     .game(Game::default()) // Alice (Terran) beats Bob (Zerg) on Pylon LE
//!     .game(Game { id: 2, players: vec![Player::lost("Alice", "Terran"), Player::won("Carol", "Protoss")], ..Default::default() });
//! let details = snapshot.source().table(DETAILS_IPC)?;
//! ```

use super::source::MemSnapshot;
use polars::prelude::*;
use swarmy_common::*;

/// A player of a test game. `team` 0 means "a team of their own, numbered by position".
#[derive(Debug, Clone)]
pub struct Player {
    pub name: &'static str,
    /// `Terran`, `Zerg` or `Protoss`; another name is an arcade race, left out of the views.
    pub race: &'static str,
    /// `Win`, `Loss` or `Tie`, as the details of the replays say it.
    pub result: &'static str,
    pub team: u8,
    pub observer: bool,
    /// A computer player (the A.I. of the game).
    pub computer: bool,
    /// The ladder rating when the game started.
    pub rating: Option<i32>,
    /// The id of the Battle.net account (`player_toon_id`), 0 for one made from the name: a name
    /// is one account unless said otherwise. The computers have no account.
    pub account: u64,
    /// The units of the player in the game, for `game_units.arrow`.
    pub units: Vec<Unit>,
    /// The stats of the player during the game, for `stats.arrow` (one every 10 seconds in a
    /// replay, as many as the test needs here).
    pub stats: Vec<Stat>,
    /// The abilities the player used in the game, for `game_abilities.arrow` and the usage
    /// tables.
    pub abilities: Vec<Ability>,
    /// The economy of the player at the minutes of the game, for `game_minutes.arrow`.
    pub minutes: Vec<Minute>,
    /// Where the player's first townhall stands, for `unit_born.arrow` at game loop 0; none for
    /// a player without a start location (an observer).
    pub start: Option<(f32, f32)>,
    /// The commands the player gave on a point of the map, for `cmd_target_point.arrow`.
    pub commands: Vec<Command>,
    /// The units of the player that died, for `unit_died.arrow` and their birth in
    /// `unit_born.arrow`.
    pub deaths: Vec<Death>,
    /// The structures and units the player made during the game, for `unit_born.arrow`: the
    /// expansions and the tech structures of the openings.
    pub born: Vec<Born>,
    /// The upgrades the player finished, for `upgrades.arrow`.
    pub upgrades: Vec<Upgrade>,
}

/// A unit born during the game, a row of `unit_born.arrow`.
#[derive(Debug, Clone)]
pub struct Born {
    /// The game second it finished.
    pub seconds: u32,
    pub unit: &'static str,
    /// The tag index of the unit, unique in the game; the same tag twice is the same unit born
    /// again (a Command Center landing).
    pub tag: u32,
    /// Where it stands, in map units; at the player's start when None.
    pub at: Option<(f32, f32)>,
}

impl Born {
    pub fn new(seconds: u32, unit: &'static str, tag: u32) -> Self {
        Self {
            seconds,
            unit,
            tag,
            at: None,
        }
    }

    /// A unit born at a position of the map, i.e. an expansion.
    pub fn at(seconds: u32, unit: &'static str, tag: u32, at: (f32, f32)) -> Self {
        Self {
            seconds,
            unit,
            tag,
            at: Some(at),
        }
    }
}

/// An upgrade finished, a row of `upgrades.arrow`.
#[derive(Debug, Clone)]
pub struct Upgrade {
    /// The game second it finished.
    pub seconds: u32,
    pub name: &'static str,
}

impl Upgrade {
    pub fn new(seconds: u32, name: &'static str) -> Self {
        Self { seconds, name }
    }
}

/// A command on a point of the map, a row of `cmd_target_point.arrow`.
#[derive(Debug, Clone)]
pub struct Command {
    /// The game second.
    pub seconds: u32,
    /// The target, in map units.
    pub x: f32,
    pub y: f32,
    /// The units commanded, comma separated.
    pub unit_names: &'static str,
}

impl Command {
    pub fn new(seconds: u32, x: f32, y: f32, unit_names: &'static str) -> Self {
        Self {
            seconds,
            x,
            y,
            unit_names,
        }
    }
}

/// The death of a unit of the player, a row of `unit_died.arrow` (and the unit's birth, a row of
/// `unit_born.arrow` at game loop 1 with the same tag).
#[derive(Debug, Clone)]
pub struct Death {
    /// The game second.
    pub seconds: u32,
    /// Where it died, in map units.
    pub x: u8,
    pub y: u8,
    pub unit: &'static str,
    /// The tag index of the unit, unique in the game.
    pub tag: u32,
    /// Whether an enemy unit killed it; a death without a killer is a morph or a sacrifice.
    pub killed: bool,
}

impl Death {
    pub fn new(seconds: u32, x: u8, y: u8, unit: &'static str, tag: u32, killed: bool) -> Self {
        Self {
            seconds,
            x,
            y,
            unit,
            tag,
            killed,
        }
    }
}

/// The economy of a player at a minute of a game, a row of `game_minutes.arrow`
/// (`s2protocol::arrow_store::aggregates::GameMinuteRow`).
#[derive(Debug, Clone)]
pub struct Minute {
    pub minute: u32,
    pub workers: i32,
    pub supply: i32,
    pub minerals_rate: i32,
    pub vespene_rate: i32,
    pub army_value: i32,
}

impl Minute {
    pub fn new(
        minute: u32,
        workers: i32,
        supply: i32,
        income: (i32, i32),
        army_value: i32,
    ) -> Self {
        Self {
            minute,
            workers,
            supply,
            minerals_rate: income.0,
            vespene_rate: income.1,
            army_value,
        }
    }
}

/// The key of a usage row: period, protocol version, race, account and player.
type UsageKey = (String, u32, String, String, String);

/// The uses of an ability by a player in a game, a row of `game_abilities.arrow`.
#[derive(Debug, Clone)]
pub struct Ability {
    /// The ability as the tracker names it, i.e. `SpawnLarva.Execute`.
    pub name: &'static str,
    pub uses: u32,
    /// The game second of the first use.
    pub first_seconds: u32,
}

impl Ability {
    pub fn new(name: &'static str, uses: u32, first_seconds: u32) -> Self {
        Self {
            name,
            uses,
            first_seconds,
        }
    }
}

/// The stats of a player at a game loop, a row of `stats.arrow` (the columns the queries read).
#[derive(Debug, Clone, Default)]
pub struct Stat {
    /// The game loop, 22.4 per second at Faster speed.
    pub game_loop: i64,
    pub food_made: i32,
    pub food_used: i32,
    pub workers: i32,
    pub minerals_rate: i32,
    pub vespene_rate: i32,
    pub army_minerals: i32,
    pub army_vespene: i32,
}

impl Stat {
    /// The stats at a time of the game, in seconds.
    pub fn at_seconds(seconds: f64) -> Self {
        Self {
            game_loop: (seconds * 22.4) as i64,
            ..Default::default()
        }
    }
}

/// The units of a type a player made in a game and what they did, a row of `game_units.arrow`.
#[derive(Debug, Clone)]
pub struct Unit {
    pub name: &'static str,
    pub born: u32,
    pub died: u32,
    /// The enemy units that cost resources they killed, and the resources of those.
    pub kills: u32,
    pub value_killed: u64,
}

impl Unit {
    pub fn new(name: &'static str, born: u32, kills: u32, value_killed: u64) -> Self {
        Self {
            name,
            born,
            died: 0,
            kills,
            value_killed,
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            name: "Alice",
            race: "Terran",
            result: "Win",
            team: 0,
            observer: false,
            computer: false,
            rating: None,
            account: 0,
            units: vec![],
            stats: vec![],
            abilities: vec![],
            minutes: vec![],
            start: None,
            commands: vec![],
            deaths: vec![],
            born: vec![],
            upgrades: vec![],
        }
    }
}

impl Player {
    pub fn won(name: &'static str, race: &'static str) -> Self {
        Self {
            name,
            race,
            result: "Win",
            ..Default::default()
        }
    }

    pub fn lost(name: &'static str, race: &'static str) -> Self {
        Self {
            name,
            race,
            result: "Loss",
            ..Default::default()
        }
    }

    pub fn observer(name: &'static str) -> Self {
        Self {
            name,
            race: "",
            result: "",
            observer: true,
            ..Default::default()
        }
    }
}

/// A test game. The defaults: Alice (Terran) beats Bob (Zerg) on Pylon LE, 10 minutes long.
#[derive(Debug, Clone)]
pub struct Game {
    /// The replay id, `ext_fs_id`; the games of a snapshot have different ids.
    pub id: u64,
    pub map: &'static str,
    /// When the game was played, `YYYY-MM-DD HH:MM:SS`.
    pub played_at: &'static str,
    /// The protocol version (base build) of the replay.
    pub protocol_version: u32,
    /// The length of the game, in seconds at Faster speed.
    pub seconds: u32,
    /// The comma separated cache handles of the map, the same for the games of a map version.
    pub cache_handles: &'static str,
    /// The size of the map in map units, `lobby_slot_map_size_x` and `_y` of the details.
    pub map_size: (u8, u8),
    pub players: Vec<Player>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            id: 1,
            map: "Pylon LE",
            played_at: "2026-01-15 20:30:00",
            protocol_version: 95_248,
            seconds: 600,
            cache_handles: "pylon-v1",
            map_size: (168, 180),
            players: vec![Player::won("Alice", "Terran"), Player::lost("Bob", "Zerg")],
        }
    }
}

impl Game {
    /// The team of the player at `index`: the one given, or their position.
    fn team(&self, index: usize) -> u8 {
        match self.players[index].team {
            0 => index as u8 + 1,
            team => team,
        }
    }

    /// The tracker id of the player at `index`: the players of the game numbered from 1, the
    /// observers have none.
    fn player_id(&self, index: usize) -> Option<u8> {
        if self.players[index].observer {
            return None;
        }
        Some(
            self.players[..index]
                .iter()
                .filter(|player| !player.observer)
                .count() as u8
                + 1,
        )
    }

    fn nanos(&self) -> i64 {
        chrono::NaiveDateTime::parse_from_str(self.played_at, "%Y-%m-%d %H:%M:%S")
            .unwrap_or_else(|err| panic!("played_at {:?}: {err}", self.played_at))
            .and_utc()
            .timestamp_nanos_opt()
            .expect("a date of this century")
    }
}

/// The games of a test snapshot, see the module documentation.
#[derive(Debug, Default, Clone)]
pub struct Snapshot {
    pub games: Vec<Game>,
    /// The accounts grouped under a player in the personal database, `(account, player)`:
    /// `("2-1-42", "Alice")` makes the games of that account Alice's.
    pub groups: Vec<(&'static str, &'static str)>,
}

/// The table of the groups of accounts for the SQL of the READMEs, `player_groups`: the personal
/// database has no table in the snapshot, the plans take the groups as a frame.
pub const PLAYER_GROUPS_IPC: &str = "player_groups.arrow";

/// The table of the patch of each protocol version for the SQL of the READMEs,
/// `patch_versions`: swarmy computes it from the balance data (`data::patch_versions`).
pub const PATCH_VERSIONS_IPC: &str = "patch_versions.arrow";

impl Snapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn game(mut self, game: Game) -> Self {
        self.games.push(game);
        self
    }

    /// Groups an account under a player, as the Player view's Accounts panel does.
    pub fn group(mut self, account: &'static str, player: &'static str) -> Self {
        self.groups.push((account, player));
        self
    }

    /// The groups as the personal database gives them, account to player.
    pub fn groups_map(&self) -> std::collections::HashMap<String, String> {
        self.groups
            .iter()
            .map(|(account, player)| (account.to_string(), player.to_string()))
            .collect()
    }

    /// The account of a player as the details write it, `region-realm-id`.
    pub fn account_of(player: &Player) -> String {
        format!("2-1-{}", Self::toon_id(player))
    }

    fn toon_id(player: &Player) -> u64 {
        if player.computer {
            return 0;
        }
        if player.account != 0 {
            return player.account;
        }
        player
            .name
            .bytes()
            .fold(1_000_000u64, |id, byte| id * 31 + byte as u64)
    }

    /// The tables of the snapshot as the queries read them, and the groups for the SQL.
    pub fn source(&self) -> MemSnapshot {
        MemSnapshot::new()
            .with(DETAILS_IPC, self.details())
            .with(GAME_PLAYERS_IPC, self.game_players())
            .with(GAME_UNITS_IPC, self.game_units())
            .with(STATS_IPC, self.stats())
            .with(PLAYER_GROUPS_IPC, self.player_groups())
            .with(PATCH_VERSIONS_IPC, self.patch_versions())
            .with(GAME_ABILITIES_IPC, self.game_abilities())
            .with(GAME_MINUTES_IPC, self.game_minutes())
            .with(UNIT_BORN_IPC, self.unit_born())
            .with(UNIT_DIED_IPC, self.unit_died())
            .with(CMD_TARGET_POINT_IPC, self.cmd_target_point())
            .with(UPGRADES_IPC, self.upgrades())
            .with_usage(
                TimeGrain::Day,
                PLAYER_USAGE_IPC,
                self.player_usage(TimeGrain::Day),
            )
            .with_usage(
                TimeGrain::Month,
                PLAYER_USAGE_IPC,
                self.player_usage(TimeGrain::Month),
            )
            .with_usage(
                TimeGrain::Year,
                PLAYER_USAGE_IPC,
                self.player_usage(TimeGrain::Year),
            )
            .with_usage(
                TimeGrain::Day,
                ABILITY_USAGE_IPC,
                self.ability_usage(TimeGrain::Day),
            )
            .with_usage(
                TimeGrain::Month,
                ABILITY_USAGE_IPC,
                self.ability_usage(TimeGrain::Month),
            )
            .with_usage(
                TimeGrain::Year,
                ABILITY_USAGE_IPC,
                self.ability_usage(TimeGrain::Year),
            )
            .with_usage(
                TimeGrain::Day,
                UNIT_USAGE_IPC,
                self.unit_usage(TimeGrain::Day),
            )
            .with_usage(
                TimeGrain::Month,
                UNIT_USAGE_IPC,
                self.unit_usage(TimeGrain::Month),
            )
            .with_usage(
                TimeGrain::Year,
                UNIT_USAGE_IPC,
                self.unit_usage(TimeGrain::Year),
            )
    }

    /// The period of a game at a grain, as the directories of the aggregations name it:
    /// `2026-01-15`, `2026-01` or `2026`.
    pub fn period(game: &Game, grain: TimeGrain) -> String {
        let date = &game.played_at[..10];
        match grain {
            TimeGrain::Day => date.to_string(),
            TimeGrain::Month => date[..7].to_string(),
            TimeGrain::Year => date[..4].to_string(),
        }
    }

    /// The account of the usage rows: `region-realm-id`, empty for a computer.
    fn usage_account(player: &Player) -> String {
        if player.computer {
            String::new()
        } else {
            Self::account_of(player)
        }
    }

    /// The name without the clan tag, the `player` of the usage rows.
    fn usage_player(player: &Player) -> String {
        player
            .name
            .rsplit("<sp/>")
            .next()
            .unwrap_or(player.name)
            .to_string()
    }

    /// Whether a game is a 1v1 of two different races (as stored), the `versus` games of the
    /// aggregations.
    fn is_versus(game: &Game) -> bool {
        let races: Vec<&str> = game
            .players
            .iter()
            .filter(|player| !player.observer)
            .map(|player| player.race)
            .collect();
        races.len() == 2 && races[0] != races[1]
    }

    /// The players of the games with the key of the usage rows: period, version, race, account
    /// and player, with whether they won, and the game.
    fn usage_players(&self, grain: TimeGrain) -> Vec<(UsageKey, bool, &Game, &Player)> {
        self.games
            .iter()
            .flat_map(move |game| {
                game.players
                    .iter()
                    .filter(|player| !player.observer)
                    .map(move |player| {
                        (
                            (
                                Self::period(game, grain),
                                game.protocol_version,
                                player.race.to_string(),
                                Self::usage_account(player),
                                Self::usage_player(player),
                            ),
                            player.result == "Win",
                            game,
                            player,
                        )
                    })
            })
            .collect()
    }

    /// `game_abilities.arrow`: a row per ability used by a player in a game,
    /// `s2protocol::arrow_store::aggregates::GameAbilityRow`.
    pub fn game_abilities(&self) -> DataFrame {
        let rows: Vec<(&Game, &Player, &Ability)> = self
            .games
            .iter()
            .flat_map(|game| {
                game.players.iter().flat_map(move |player| {
                    player
                        .abilities
                        .iter()
                        .map(move |ability| (game, player, ability))
                })
            })
            .collect();
        DataFrame::new(
            rows.len(),
            vec![
                Column::new(
                    "ext_fs_id".into(),
                    rows.iter()
                        .map(|(game, _, _)| game.id)
                        .collect::<Vec<u64>>(),
                ),
                Column::new(
                    "player_name".into(),
                    rows.iter()
                        .map(|(_, player, _)| player.name)
                        .collect::<Vec<&str>>(),
                ),
                Column::new(
                    "ability".into(),
                    rows.iter()
                        .map(|(_, _, ability)| ability.name)
                        .collect::<Vec<&str>>(),
                ),
                Column::new(
                    "uses".into(),
                    rows.iter()
                        .map(|(_, _, ability)| ability.uses)
                        .collect::<Vec<u32>>(),
                ),
                Column::new(
                    "first_seconds".into(),
                    rows.iter()
                        .map(|(_, _, ability)| ability.first_seconds)
                        .collect::<Vec<u32>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `player_usage.arrow` of a grain: the games of each player per period, as
    /// `s2protocol::arrow_store::aggregates::PlayerUsageRow` adds them up.
    pub fn player_usage(&self, grain: TimeGrain) -> DataFrame {
        let mut totals: std::collections::BTreeMap<_, [u32; 4]> = std::collections::BTreeMap::new();
        for (key, won, game, _) in self.usage_players(grain) {
            let versus = Self::is_versus(game);
            let total = totals.entry(key).or_default();
            total[0] += 1;
            total[1] += won as u32;
            total[2] += versus as u32;
            total[3] += (versus && won) as u32;
        }
        let keys: Vec<_> = totals.keys().cloned().collect();
        let values: Vec<[u32; 4]> = totals.values().copied().collect();
        DataFrame::new(
            keys.len(),
            vec![
                Column::new(
                    "period".into(),
                    keys.iter().map(|k| k.0.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "ext_protocol_version".into(),
                    keys.iter().map(|k| k.1).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "player_race".into(),
                    keys.iter().map(|k| k.2.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "account".into(),
                    keys.iter().map(|k| k.3.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "player".into(),
                    keys.iter().map(|k| k.4.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "games".into(),
                    values.iter().map(|v| v[0]).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "wins".into(),
                    values.iter().map(|v| v[1]).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "versus_games".into(),
                    values.iter().map(|v| v[2]).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "versus_wins".into(),
                    values.iter().map(|v| v[3]).collect::<Vec<u32>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `ability_usage.arrow` of a grain, `AbilityUsageRow`: the games, wins and uses of each
    /// ability by player and period.
    pub fn ability_usage(&self, grain: TimeGrain) -> DataFrame {
        let mut totals: std::collections::BTreeMap<_, (u32, u32, u64)> =
            std::collections::BTreeMap::new();
        for (key, won, _, player) in self.usage_players(grain) {
            for ability in &player.abilities {
                let total = totals
                    .entry((key.clone(), ability.name.to_string()))
                    .or_default();
                total.0 += 1;
                total.1 += won as u32;
                total.2 += ability.uses as u64;
            }
        }
        let keys: Vec<_> = totals.keys().cloned().collect();
        let values: Vec<_> = totals.values().copied().collect();
        DataFrame::new(
            keys.len(),
            vec![
                Column::new(
                    "period".into(),
                    keys.iter().map(|k| k.0.0.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "ext_protocol_version".into(),
                    keys.iter().map(|k| k.0.1).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "player_race".into(),
                    keys.iter().map(|k| k.0.2.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "account".into(),
                    keys.iter().map(|k| k.0.3.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "player".into(),
                    keys.iter().map(|k| k.0.4.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "ability".into(),
                    keys.iter().map(|k| k.1.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "games".into(),
                    values.iter().map(|v| v.0).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "wins".into(),
                    values.iter().map(|v| v.1).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "uses".into(),
                    values.iter().map(|v| v.2).collect::<Vec<u64>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `unit_usage.arrow` of a grain, `UnitUsageRow`: the units of each type made by player and
    /// period, with their games, wins, deaths and kills.
    pub fn unit_usage(&self, grain: TimeGrain) -> DataFrame {
        let mut totals: std::collections::BTreeMap<_, [u64; 6]> = std::collections::BTreeMap::new();
        for (key, won, _, player) in self.usage_players(grain) {
            for unit in &player.units {
                let made = unit.born > 0;
                let total = totals
                    .entry((key.clone(), unit.name.to_string()))
                    .or_default();
                total[0] += made as u64;
                total[1] += (made && won) as u64;
                total[2] += unit.born as u64;
                total[3] += unit.died as u64;
                total[4] += unit.kills as u64;
                total[5] += unit.value_killed;
            }
        }
        let keys: Vec<_> = totals.keys().cloned().collect();
        let values: Vec<[u64; 6]> = totals.values().copied().collect();
        DataFrame::new(
            keys.len(),
            vec![
                Column::new(
                    "period".into(),
                    keys.iter().map(|k| k.0.0.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "ext_protocol_version".into(),
                    keys.iter().map(|k| k.0.1).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "player_race".into(),
                    keys.iter().map(|k| k.0.2.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "account".into(),
                    keys.iter().map(|k| k.0.3.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "player".into(),
                    keys.iter().map(|k| k.0.4.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "unit".into(),
                    keys.iter().map(|k| k.1.as_str()).collect::<Vec<_>>(),
                ),
                Column::new(
                    "games".into(),
                    values.iter().map(|v| v[0] as u32).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "wins".into(),
                    values.iter().map(|v| v[1] as u32).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "born".into(),
                    values.iter().map(|v| v[2]).collect::<Vec<u64>>(),
                ),
                Column::new(
                    "died".into(),
                    values.iter().map(|v| v[3]).collect::<Vec<u64>>(),
                ),
                Column::new(
                    "kills".into(),
                    values.iter().map(|v| v[4]).collect::<Vec<u64>>(),
                ),
                Column::new(
                    "value_killed".into(),
                    values.iter().map(|v| v[5]).collect::<Vec<u64>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `patch_versions`: the patch of the balance data of each protocol version of the games, as
    /// `data::patch_versions` computes it (a frame made by swarmy, for the SQL of the READMEs).
    pub fn patch_versions(&self) -> DataFrame {
        let mut versions: Vec<u32> = self
            .games
            .iter()
            .map(|game| game.protocol_version)
            .collect();
        versions.sort_unstable();
        versions.dedup();
        let patches: Vec<u32> = versions
            .iter()
            .map(|version| {
                crate::replay_rerun::decisions::closest_balance_version(*version)
                    .expect("a balance version")
            })
            .collect();
        DataFrame::new(
            versions.len(),
            vec![
                Column::new("ext_protocol_version".into(), versions),
                Column::new("patch".into(), patches),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `stats.arrow`: the stats of each player every few seconds, the columns the queries read
    /// (the real table has forty, `s2protocol::tracker_events::arrow_store::PlayerStatsFlatRow`).
    pub fn stats(&self) -> DataFrame {
        let rows: Vec<(&Game, u8, &Stat)> = self
            .games
            .iter()
            .flat_map(|game| {
                (0..game.players.len()).flat_map(move |index| {
                    let player_id = game.player_id(index);
                    game.players[index]
                        .stats
                        .iter()
                        .filter_map(move |stat| player_id.map(|id| (game, id, stat)))
                })
            })
            .collect();
        let i32_col = |name: &str, value: &dyn Fn(&Stat) -> i32| {
            Column::new(
                name.into(),
                rows.iter()
                    .map(|(_, _, stat)| value(stat))
                    .collect::<Vec<i32>>(),
            )
        };
        DataFrame::new(
            rows.len(),
            vec![
                Column::new(
                    "player_id".into(),
                    rows.iter().map(|(_, id, _)| *id).collect::<Vec<u8>>(),
                ),
                i32_col("minerals_collection_rate", &|stat| stat.minerals_rate),
                i32_col("vespene_collection_rate", &|stat| stat.vespene_rate),
                i32_col("workers_active_count", &|stat| stat.workers),
                i32_col("minerals_used_current_army", &|stat| stat.army_minerals),
                i32_col("vespene_used_current_army", &|stat| stat.army_vespene),
                i32_col("food_used", &|stat| stat.food_used),
                i32_col("food_made", &|stat| stat.food_made),
                Column::new(
                    "ext_replay_loop".into(),
                    rows.iter()
                        .map(|(_, _, stat)| stat.game_loop)
                        .collect::<Vec<i64>>(),
                ),
                Column::new(
                    "ext_fs_id".into(),
                    rows.iter()
                        .map(|(game, _, _)| game.id)
                        .collect::<Vec<u64>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `game_minutes.arrow`: the economy of each player at each minute, by tracker player id.
    pub fn game_minutes(&self) -> DataFrame {
        let rows: Vec<(&Game, u8, &Minute)> = self
            .games
            .iter()
            .flat_map(|game| {
                (0..game.players.len()).flat_map(move |index| {
                    let player_id = game.player_id(index);
                    game.players[index]
                        .minutes
                        .iter()
                        .filter_map(move |minute| player_id.map(|id| (game, id, minute)))
                })
            })
            .collect();
        let i32_col = |name: &str, value: &dyn Fn(&Minute) -> i32| {
            Column::new(
                name.into(),
                rows.iter()
                    .map(|(_, _, minute)| value(minute))
                    .collect::<Vec<i32>>(),
            )
        };
        DataFrame::new(
            rows.len(),
            vec![
                Column::new(
                    "ext_fs_id".into(),
                    rows.iter()
                        .map(|(game, _, _)| game.id)
                        .collect::<Vec<u64>>(),
                ),
                Column::new(
                    "player_id".into(),
                    rows.iter().map(|(_, id, _)| *id).collect::<Vec<u8>>(),
                ),
                Column::new(
                    "minute".into(),
                    rows.iter()
                        .map(|(_, _, minute)| minute.minute)
                        .collect::<Vec<u32>>(),
                ),
                i32_col("workers", &|minute| minute.workers),
                i32_col("supply", &|minute| minute.supply),
                i32_col("minerals_rate", &|minute| minute.minerals_rate),
                i32_col("vespene_rate", &|minute| minute.vespene_rate),
                i32_col("army_value", &|minute| minute.army_value),
            ],
        )
        .expect("the columns have the same height")
    }

    /// The game loop of a second, rounded up so that `data::game_seconds` gives the second back.
    fn game_loop(seconds: u32) -> i64 {
        (seconds as f64 * 22.4).ceil() as i64
    }

    /// The townhall of a race at the start of a game.
    fn townhall(race: &str) -> &'static str {
        match race {
            "Terran" => "CommandCenter",
            "Protoss" => "Nexus",
            _ => "Hatchery",
        }
    }

    /// `unit_born.arrow`, the columns the queries read: the first townhall of each player with a
    /// start (game loop 0, tag 100 + the player's index) and the units that die later (game loop
    /// 1, their tag).
    pub fn unit_born(&self) -> DataFrame {
        struct Row<'a> {
            game: &'a Game,
            player: &'a Player,
            player_id: Option<u8>,
            tag: u32,
            unit: &'static str,
            x: f32,
            y: f32,
            game_loop: i64,
        }
        let mut rows: Vec<Row> = vec![];
        for game in &self.games {
            for (index, player) in game.players.iter().enumerate() {
                let player_id = game.player_id(index);
                let (start_x, start_y) = player.start.unwrap_or_default();
                if let Some((x, y)) = player.start {
                    rows.push(Row {
                        game,
                        player,
                        player_id,
                        tag: 100 + index as u32,
                        unit: Self::townhall(player.race),
                        x,
                        y,
                        game_loop: 0,
                    });
                }
                for death in &player.deaths {
                    rows.push(Row {
                        game,
                        player,
                        player_id,
                        tag: death.tag,
                        unit: death.unit,
                        x: death.x as f32,
                        y: death.y as f32,
                        game_loop: 1,
                    });
                }
                for born in &player.born {
                    let (x, y) = born.at.unwrap_or((start_x, start_y));
                    rows.push(Row {
                        game,
                        player,
                        player_id,
                        tag: born.tag,
                        unit: born.unit,
                        x,
                        y,
                        game_loop: Self::game_loop(born.seconds),
                    });
                }
            }
        }
        DataFrame::new(
            rows.len(),
            vec![
                Column::new(
                    "unit_tag_index".into(),
                    rows.iter().map(|row| row.tag).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "unit_tag_recycle".into(),
                    rows.iter().map(|_| 1u32).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "unit_type_name".into(),
                    rows.iter().map(|row| row.unit).collect::<Vec<&str>>(),
                ),
                Column::new(
                    "control_player_id".into(),
                    rows.iter()
                        .map(|row| row.player_id)
                        .collect::<Vec<Option<u8>>>(),
                ),
                Column::new(
                    "x".into(),
                    rows.iter().map(|row| row.x).collect::<Vec<f32>>(),
                ),
                Column::new(
                    "y".into(),
                    rows.iter().map(|row| row.y).collect::<Vec<f32>>(),
                ),
                Column::new(
                    "ext_replay_loop".into(),
                    rows.iter().map(|row| row.game_loop).collect::<Vec<i64>>(),
                ),
                Column::new(
                    "ext_fs_id".into(),
                    rows.iter().map(|row| row.game.id).collect::<Vec<u64>>(),
                ),
                Column::new(
                    "player_name".into(),
                    rows.iter()
                        .map(|row| Some(row.player.name))
                        .collect::<Vec<Option<&str>>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `unit_died.arrow`, the columns the queries read: the deaths of the players' units, the
    /// killer tag 999 when killed.
    pub fn unit_died(&self) -> DataFrame {
        let rows: Vec<(&Game, &Death)> = self
            .games
            .iter()
            .flat_map(|game| {
                game.players
                    .iter()
                    .flat_map(move |player| player.deaths.iter().map(move |death| (game, death)))
            })
            .collect();
        DataFrame::new(
            rows.len(),
            vec![
                Column::new(
                    "unit_died_name".into(),
                    rows.iter()
                        .map(|(_, death)| death.unit)
                        .collect::<Vec<&str>>(),
                ),
                Column::new(
                    "unit_tag_index".into(),
                    rows.iter()
                        .map(|(_, death)| death.tag)
                        .collect::<Vec<u32>>(),
                ),
                Column::new(
                    "unit_tag_recycle".into(),
                    rows.iter().map(|_| 1u32).collect::<Vec<u32>>(),
                ),
                Column::new(
                    "x".into(),
                    rows.iter().map(|(_, death)| death.x).collect::<Vec<u8>>(),
                ),
                Column::new(
                    "y".into(),
                    rows.iter().map(|(_, death)| death.y).collect::<Vec<u8>>(),
                ),
                Column::new(
                    "killer_unit_tag_index".into(),
                    rows.iter()
                        .map(|(_, death)| death.killed.then_some(999u32))
                        .collect::<Vec<Option<u32>>>(),
                ),
                Column::new(
                    "ext_replay_loop".into(),
                    rows.iter()
                        .map(|(_, death)| Self::game_loop(death.seconds))
                        .collect::<Vec<i64>>(),
                ),
                Column::new(
                    "ext_fs_id".into(),
                    rows.iter().map(|(game, _)| game.id).collect::<Vec<u64>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `cmd_target_point.arrow`, the columns the queries read: the commands of the players on a
    /// point, the coordinates in the fixed point of the game events (4096 per map unit).
    pub fn cmd_target_point(&self) -> DataFrame {
        let rows: Vec<(&Game, &Player, &Command)> = self
            .games
            .iter()
            .flat_map(|game| {
                game.players.iter().flat_map(move |player| {
                    player
                        .commands
                        .iter()
                        .map(move |command| (game, player, command))
                })
            })
            .collect();
        DataFrame::new(
            rows.len(),
            vec![
                Column::new(
                    "player_name".into(),
                    rows.iter()
                        .map(|(_, player, _)| Some(player.name))
                        .collect::<Vec<Option<&str>>>(),
                ),
                Column::new(
                    "target_point_x".into(),
                    rows.iter()
                        .map(|(_, _, command)| (command.x * 4096.) as i64)
                        .collect::<Vec<i64>>(),
                ),
                Column::new(
                    "target_point_y".into(),
                    rows.iter()
                        .map(|(_, _, command)| (command.y * 4096.) as i64)
                        .collect::<Vec<i64>>(),
                ),
                Column::new(
                    "unit_names".into(),
                    rows.iter()
                        .map(|(_, _, command)| command.unit_names)
                        .collect::<Vec<&str>>(),
                ),
                Column::new(
                    "ext_replay_loop".into(),
                    rows.iter()
                        .map(|(_, _, command)| Self::game_loop(command.seconds))
                        .collect::<Vec<i64>>(),
                ),
                Column::new(
                    "ext_fs_id".into(),
                    rows.iter()
                        .map(|(game, _, _)| game.id)
                        .collect::<Vec<u64>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `upgrades.arrow`, the columns the queries read: the upgrades each player finished, by
    /// tracker player id.
    pub fn upgrades(&self) -> DataFrame {
        let rows: Vec<(&Game, u8, &Upgrade)> = self
            .games
            .iter()
            .flat_map(|game| {
                (0..game.players.len()).flat_map(move |index| {
                    let player_id = game.player_id(index);
                    game.players[index]
                        .upgrades
                        .iter()
                        .filter_map(move |upgrade| player_id.map(|id| (game, id, upgrade)))
                })
            })
            .collect();
        DataFrame::new(
            rows.len(),
            vec![
                Column::new(
                    "player_id".into(),
                    rows.iter().map(|(_, id, _)| *id).collect::<Vec<u8>>(),
                ),
                Column::new(
                    "name".into(),
                    rows.iter()
                        .map(|(_, _, upgrade)| upgrade.name)
                        .collect::<Vec<&str>>(),
                ),
                Column::new(
                    "ext_replay_loop".into(),
                    rows.iter()
                        .map(|(_, _, upgrade)| Self::game_loop(upgrade.seconds))
                        .collect::<Vec<i64>>(),
                ),
                Column::new(
                    "ext_fs_id".into(),
                    rows.iter()
                        .map(|(game, _, _)| game.id)
                        .collect::<Vec<u64>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `player_groups`: the groups of accounts, `account` and `grouped_as`.
    pub fn player_groups(&self) -> DataFrame {
        DataFrame::new(
            self.groups.len(),
            vec![
                Column::new(
                    "account".into(),
                    self.groups
                        .iter()
                        .map(|(account, _)| *account)
                        .collect::<Vec<&str>>(),
                ),
                Column::new(
                    "grouped_as".into(),
                    self.groups
                        .iter()
                        .map(|(_, player)| *player)
                        .collect::<Vec<&str>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `game_units.arrow`: a row per unit type of each player of each game,
    /// `s2protocol::arrow_store::aggregates::GameUnitRow`.
    pub fn game_units(&self) -> DataFrame {
        let rows: Vec<(&Game, &Player, &Unit)> = self
            .games
            .iter()
            .flat_map(|game| {
                game.players.iter().flat_map(move |player| {
                    player.units.iter().map(move |unit| (game, player, unit))
                })
            })
            .collect();
        DataFrame::new(
            rows.len(),
            vec![
                Column::new(
                    "ext_fs_id".into(),
                    rows.iter()
                        .map(|(game, _, _)| game.id)
                        .collect::<Vec<u64>>(),
                ),
                Column::new(
                    "player_name".into(),
                    rows.iter()
                        .map(|(_, player, _)| player.name)
                        .collect::<Vec<&str>>(),
                ),
                Column::new(
                    "unit".into(),
                    rows.iter()
                        .map(|(_, _, unit)| unit.name)
                        .collect::<Vec<&str>>(),
                ),
                Column::new(
                    "born".into(),
                    rows.iter()
                        .map(|(_, _, unit)| unit.born)
                        .collect::<Vec<u32>>(),
                ),
                Column::new(
                    "died".into(),
                    rows.iter()
                        .map(|(_, _, unit)| unit.died)
                        .collect::<Vec<u32>>(),
                ),
                Column::new(
                    "kills".into(),
                    rows.iter()
                        .map(|(_, _, unit)| unit.kills)
                        .collect::<Vec<u32>>(),
                ),
                Column::new(
                    "value_killed".into(),
                    rows.iter()
                        .map(|(_, _, unit)| unit.value_killed)
                        .collect::<Vec<u64>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }

    /// `details.arrow`: a row per player of each game, observers included, with the columns of
    /// `s2protocol::details::PlayerLobbyDetailsFlatRow`.
    pub fn details(&self) -> DataFrame {
        let rows: Vec<(&Game, usize)> = self
            .games
            .iter()
            .flat_map(|game| (0..game.players.len()).map(move |index| (game, index)))
            .collect();
        let player = |index: usize| &rows[index].0.players[rows[index].1];
        let str_col = |name: &str, value: &dyn Fn(usize) -> &'static str| {
            Column::new(
                name.into(),
                (0..rows.len()).map(value).collect::<Vec<&str>>(),
            )
        };
        let u8_col = |name: &str, value: &dyn Fn(usize) -> u8| {
            Column::new(name.into(), (0..rows.len()).map(value).collect::<Vec<u8>>())
        };
        let u32_col = |name: &str, value: &dyn Fn(usize) -> u32| {
            Column::new(
                name.into(),
                (0..rows.len()).map(value).collect::<Vec<u32>>(),
            )
        };
        let u64_col = |name: &str, value: &dyn Fn(usize) -> u64| {
            Column::new(
                name.into(),
                (0..rows.len()).map(value).collect::<Vec<u64>>(),
            )
        };
        let i64_col = |name: &str, value: &dyn Fn(usize) -> i64| {
            Column::new(
                name.into(),
                (0..rows.len()).map(value).collect::<Vec<i64>>(),
            )
        };
        let toon_id = |index: usize| -> u64 { Self::toon_id(player(index)) };
        let columns = vec![
            str_col("player_name", &|i| player(i).name),
            u8_col("player_toon_region", &|_| 2),
            u32_col("player_toon_program_id", &|_| 0),
            u32_col("player_toon_realm", &|_| 1),
            u64_col("player_toon_id", &toon_id),
            str_col("player_race", &|i| player(i).race),
            u8_col("player_color_a", &|_| 255),
            u8_col("player_color_r", &|i| if rows[i].1 == 0 { 180 } else { 0 }),
            u8_col("player_color_g", &|i| if rows[i].1 == 0 { 20 } else { 66 }),
            u8_col("player_color_b", &|i| if rows[i].1 == 0 { 30 } else { 255 }),
            u8_col("player_control", &|i| {
                if player(i).computer { 3 } else { 2 }
            }),
            u8_col("player_team_id", &|i| rows[i].0.team(rows[i].1)),
            u8_col("player_observe", &|i| player(i).observer as u8),
            str_col("player_result", &|i| player(i).result),
            Column::new(
                "player_working_set_slot_id".into(),
                (0..rows.len())
                    .map(|i| Some(rows[i].1 as u8))
                    .collect::<Vec<Option<u8>>>(),
            ),
            str_col("player_hero", &|_| ""),
            str_col("title", &|i| rows[i].0.map),
            str_col("map_info_sha256", &|_| "0000"),
            Column::new("is_blizzard_map".into(), vec![true; rows.len()]),
            i64_col("time_utc", &|i| rows[i].0.nanos() / 100),
            i64_col("time_local_offset", &|_| 0),
            Column::new(
                "lobby_slot_user_id".into(),
                (0..rows.len())
                    .map(|i| (!player(i).computer).then_some(rows[i].1 as i64))
                    .collect::<Vec<Option<i64>>>(),
            ),
            u8_col("lobby_slot_observe", &|i| player(i).observer as u8),
            u8_col("lobby_slot_map_size_x", &|i| rows[i].0.map_size.0),
            u8_col("lobby_slot_map_size_y", &|i| rows[i].0.map_size.1),
            i64_col("lobby_slot_difficulty", &|i| {
                if player(i).computer { 7 } else { 0 }
            }),
            i64_col("lobby_slot_ai_build", &|i| {
                if player(i).computer { 5 } else { 0 }
            }),
            u32_col("lobby_slot_handicap", &|_| 100),
            Column::new(
                "player_scaled_rating".into(),
                (0..rows.len())
                    .map(|i| player(i).rating)
                    .collect::<Vec<Option<i32>>>(),
            ),
            u32_col("lobby_random_seed", &|i| rows[i].0.id as u32 * 7919),
            str_col("cache_handle_region", &|_| "eu"),
            str_col("cache_handle_extension", &|_| "s2ma"),
            str_col("cache_handles", &|i| rows[i].0.cache_handles),
            str_col("ext_fs_sha256", &|_| "0000"),
            Column::new(
                "ext_fs_file_name".into(),
                rows.iter()
                    .map(|(game, _)| format!("{}.SC2Replay", game.id))
                    .collect::<Vec<String>>(),
            ),
            u64_col("ext_fs_id", &|i| rows[i].0.id),
            i64_col("ext_datetime", &|i| rows[i].0.nanos())
                .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))
                .expect("a datetime column"),
            u32_col("ext_protocol_version", &|i| rows[i].0.protocol_version),
        ];
        DataFrame::new(rows.len(), columns).expect("the columns have the same height")
    }

    /// `game_players.arrow`: a row per player of each game with the tracker `player_id` and the
    /// length of the game, `s2protocol::arrow_store::aggregates::GamePlayerRow`.
    pub fn game_players(&self) -> DataFrame {
        let rows: Vec<(&Game, usize, u8)> = self
            .games
            .iter()
            .flat_map(|game| {
                (0..game.players.len())
                    .filter_map(move |index| game.player_id(index).map(|id| (game, index, id)))
            })
            .collect();
        DataFrame::new(
            rows.len(),
            vec![
                Column::new(
                    "ext_fs_id".into(),
                    rows.iter()
                        .map(|(game, _, _)| game.id)
                        .collect::<Vec<u64>>(),
                ),
                Column::new(
                    "player_name".into(),
                    rows.iter()
                        .map(|(game, index, _)| game.players[*index].name)
                        .collect::<Vec<&str>>(),
                ),
                Column::new(
                    "player_id".into(),
                    rows.iter().map(|(_, _, id)| *id).collect::<Vec<u8>>(),
                ),
                Column::new(
                    "game_seconds".into(),
                    rows.iter()
                        .map(|(game, _, _)| game.seconds)
                        .collect::<Vec<u32>>(),
                ),
            ],
        )
        .expect("the columns have the same height")
    }
}

/// The first ```sql fence of a README, the query the tests run.
pub fn sql_of(readme: &str) -> String {
    let fence = "```sql\n";
    let start = readme.find(fence).expect("a ```sql fence in the README") + fence.len();
    let end = readme[start..]
        .find("\n```")
        .expect("the end of the ```sql fence")
        + start;
    readme[start..end].to_string()
}

/// Runs the SQL of the README on the tables of the snapshot (`details`, `game_players`: the file
/// names without `.arrow`) and compares it with the plan: the same rows, in any order, with the
/// same dtypes. The SQL of a README cannot drift from the code.
pub fn assert_sql_matches_plan(readme: &str, snapshot: &Snapshot, plan: LazyFrame) {
    assert_sql_matches_source(readme, &snapshot.source(), plan, false);
}

/// As [`assert_sql_matches_plan`], the rows in the same order too: for the queries whose order
/// is their meaning (a list sorted by the viewer) and for the frames with list columns, which
/// cannot be sorted for the comparison.
pub fn assert_sql_matches_plan_in_order(readme: &str, snapshot: &Snapshot, plan: LazyFrame) {
    assert_sql_matches_source(readme, &snapshot.source(), plan, true);
}

/// As the two above, on a source with tables added to the snapshot's (a parameter of the query
/// as a table, i.e. the units of a race).
pub fn assert_sql_matches_source(
    readme: &str,
    source: &MemSnapshot,
    plan: LazyFrame,
    in_order: bool,
) {
    let ctx = polars::sql::SQLContext::new();
    source.register_in(&ctx);
    let mut ctx = ctx;
    let by_sql = ctx
        .execute(&sql_of(readme))
        .expect("the SQL of the README parses")
        .collect()
        .expect("the SQL of the README runs");
    let by_plan = plan.collect().expect("the plan runs");
    // The order of the columns means nothing, the dtypes do.
    if let Err(err) = polars_testing::asserts::assert_dataframe_equal(
        &by_plan,
        &by_sql,
        polars_testing::asserts::DataFrameEqualOptions::default()
            .with_check_row_order(in_order)
            .with_check_column_order(false),
    ) {
        panic!("{err}\nthe plan gives:\n{by_plan}\nthe SQL of the README gives:\n{by_sql}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::SnapshotSource;

    #[test]
    fn the_default_game_is_alice_beating_bob() {
        let details = Snapshot::new().game(Game::default()).details();
        assert_eq!(details.height(), 2);
        let names = crate::data::str_values(&details, "player_name").unwrap();
        let results = crate::data::str_values(&details, "player_result").unwrap();
        assert_eq!(names, ["Alice", "Bob"]);
        assert_eq!(results, ["Win", "Loss"]);
        let teams = crate::data::u8_values(&details, "player_team_id").unwrap();
        assert_eq!(teams, [Some(1), Some(2)]);
    }

    #[test]
    fn observers_have_no_tracker_id() {
        let game = Game {
            players: vec![
                Player::won("Alice", "Terran"),
                Player::observer("Caster"),
                Player::lost("Bob", "Zerg"),
            ],
            ..Default::default()
        };
        let players = Snapshot::new().game(game).game_players();
        let names = crate::data::str_values(&players, "player_name").unwrap();
        let ids = crate::data::u8_values(&players, "player_id").unwrap();
        assert_eq!(names, ["Alice", "Bob"]);
        assert_eq!(ids, [Some(1), Some(2)]);
    }

    #[test]
    fn the_source_serves_the_tables_lazily() {
        let source = Snapshot::new().game(Game::default()).source();
        let games = source
            .table(DETAILS_IPC)
            .unwrap()
            .select([col("ext_fs_id").n_unique()])
            .collect()
            .unwrap();
        assert_eq!(
            games.column("ext_fs_id").unwrap().u32().unwrap().get(0),
            Some(1)
        );
        assert!(
            source.table(UNIT_INIT_IPC).is_err(),
            "the fixtures have no unit_init table"
        );
    }

    /// The dtypes of the fixtures are the ones of a real snapshot, column by column: a query
    /// tested on the fixtures then behaves the same on the snapshot (an Int32 literal against an
    /// Int64 column is where it would differ).
    #[test]
    #[ignore = "needs SWARMY_TEST_REPLAYS, a Replay Path with a snapshot"]
    fn snapshot_fixture_schemas_match_a_real_snapshot() {
        let replays = std::env::var("SWARMY_TEST_REPLAYS").unwrap();
        let real = crate::data::IpcSnapshot::open(&replays).unwrap();
        let fixture = Snapshot::new()
            .game(Game {
                players: vec![
                    Player {
                        units: vec![Unit::new("Marine", 10, 5, 250)],
                        ..Player::won("Alice", "Terran")
                    },
                    Player::lost("Bob", "Zerg"),
                ],
                ..Default::default()
            })
            .source();
        for table in [
            DETAILS_IPC,
            GAME_PLAYERS_IPC,
            GAME_UNITS_IPC,
            GAME_ABILITIES_IPC,
            GAME_MINUTES_IPC,
        ] {
            let real_schema = real.table(table).unwrap().collect_schema().unwrap();
            let fixture_schema = fixture.table(table).unwrap().collect_schema().unwrap();
            let real: Vec<(String, String)> = real_schema
                .iter()
                .map(|(name, dtype)| (name.to_string(), dtype.to_string()))
                .collect();
            let fixture: Vec<(String, String)> = fixture_schema
                .iter()
                .map(|(name, dtype)| (name.to_string(), dtype.to_string()))
                .collect();
            assert_eq!(fixture, real, "{table}");
        }
        // The usage tables of each grain.
        for (grain, table) in [
            (TimeGrain::Month, PLAYER_USAGE_IPC),
            (TimeGrain::Year, ABILITY_USAGE_IPC),
            (TimeGrain::Day, UNIT_USAGE_IPC),
        ] {
            let real_schema = real.usage(grain, table).unwrap().collect_schema().unwrap();
            let fixture_schema = fixture
                .usage(grain, table)
                .unwrap()
                .collect_schema()
                .unwrap();
            let real: Vec<(String, String)> = real_schema
                .iter()
                .map(|(name, dtype)| (name.to_string(), dtype.to_string()))
                .collect();
            let fixture: Vec<(String, String)> = fixture_schema
                .iter()
                .map(|(name, dtype)| (name.to_string(), dtype.to_string()))
                .collect();
            assert_eq!(fixture, real, "{grain:?} {table}");
        }
        // The stats have forty columns and the tracker and game events many: the fixture has the
        // ones the queries read, each of them as in the real table.
        for table in [
            STATS_IPC,
            UNIT_BORN_IPC,
            UNIT_DIED_IPC,
            CMD_TARGET_POINT_IPC,
            UPGRADES_IPC,
        ] {
            let real_schema = real.table(table).unwrap().collect_schema().unwrap();
            let fixture_schema = fixture.table(table).unwrap().collect_schema().unwrap();
            for (name, dtype) in fixture_schema.iter() {
                assert_eq!(
                    real_schema.get(name).map(|dtype| dtype.to_string()),
                    Some(dtype.to_string()),
                    "{table}.{name}"
                );
            }
        }
    }
}
