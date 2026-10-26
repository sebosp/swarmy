use serde::{Deserialize, Serialize};

/// Contains metadata information related to the minimun, maximum date of the map in the snapshot.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct ReplayListEntry {
    /// The external filesystem id of the replay
    pub ext_fs_id: u64,
    /// The title of the map
    pub map_title: String,
    /// The game date
    pub replay_date: chrono::NaiveDate,
    /// The list of players in this replay
    pub player_list: Vec<String>,
    /// The sha256sum of the replay
    pub sha256_sum: String,
    /// The replay location
    pub replay_location: String,
    /// The replay filename
    pub replay_file_name: String,
    /// The duration of the game, in seconds.
    pub duration: i32,
    /// The result of each player in `player_list`: Win, Loss, Tie or Undecided.
    pub player_results: Vec<String>,
    /// The race of each player in `player_list`.
    pub player_races: Vec<String>,
    /// The sha256 of the replay file, the notes are kept by it.
    pub replay_sha256: String,
    /// The rating of the user, from 1 to 5 stars.
    pub rating: Option<u8>,
    /// The tags of the user.
    pub tags: Vec<String>,
}

/// The replay list returns at most this many replays, the newest ones.
pub const REPLAY_LIST_LIMIT: u32 = 1000;

/// The newest replays that match a query, and how many match in total.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct ReplayList {
    /// The number of replays that match, it can be more than the replays returned.
    pub total: usize,
    /// The newest [`REPLAY_LIST_LIMIT`] replays that match.
    pub replays: Vec<ReplayListEntry>,
}

/// The values of the dropdowns of the replay list.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct ReplayListFilters {
    /// The protocol versions in the snapshot, the newest first, with their games.
    pub protocol_versions: Vec<crate::GamesWins>,
    /// The years of the games in the snapshot, the newest first.
    pub years: Vec<i32>,
    /// The tags of the user, the most used first.
    pub tags: Vec<crate::GamesWins>,
}

/// The column the replay list is sorted by.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ReplayListSort {
    Id,
    Map,
    Players,
    Duration,
    Races,
    #[default]
    Date,
}

/// The time frames of the replay list filter, by the date the games were played.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum TimeFrame {
    #[default]
    AllTime,
    LastWeek,
    LastMonth,
    LastThreeMonths,
    LastYear,
    /// A calendar year, i.e. 2025.
    Year(i32),
}

impl TimeFrame {
    /// The first date of the time frame, before `today`, none for all time.
    pub fn min_date(self, today: chrono::NaiveDate) -> Option<chrono::NaiveDate> {
        let days_before = |days: u64| today.checked_sub_days(chrono::Days::new(days));
        match self {
            Self::AllTime => None,
            Self::LastWeek => days_before(7),
            Self::LastMonth => days_before(30),
            Self::LastThreeMonths => days_before(91),
            Self::LastYear => days_before(365),
            Self::Year(year) => chrono::NaiveDate::from_ymd_opt(year, 1, 1),
        }
    }

    /// The last date of the time frame, none for up to today.
    pub fn max_date(self) -> Option<chrono::NaiveDate> {
        match self {
            Self::Year(year) => chrono::NaiveDate::from_ymd_opt(year, 12, 31),
            _ => None,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::AllTime => "All time".to_string(),
            Self::LastWeek => "Last 7 days".to_string(),
            Self::LastMonth => "Last 30 days".to_string(),
            Self::LastThreeMonths => "Last 3 months".to_string(),
            Self::LastYear => "Last 12 months".to_string(),
            Self::Year(year) => year.to_string(),
        }
    }
}

/// The query of the replay list, the empty fields do not filter.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ReplayListQuery {
    /// Part of the name of the map.
    pub map_title: String,
    /// Part of the name of a player of the game.
    pub player_name: String,
    /// The first date of the games, included.
    pub min_date: Option<chrono::NaiveDate>,
    /// The last date of the games, included.
    pub max_date: Option<chrono::NaiveDate>,
    /// A 1v1 matchup, i.e. `TvP` (the same as `PvT`) or `ZvZ`.
    pub matchup: String,
    /// The protocol version of the games, the patch.
    pub protocol_version: Option<u32>,
    /// The players per team, i.e. 1 for 1v1 and 2 for 2v2.
    pub team_size: Option<u8>,
    /// Part of a chat message of the game, ignoring the case.
    pub message: String,
    /// A tag of the user.
    pub tag: String,
    /// The minimum rating of the user, from 1 to 5 stars.
    pub min_rating: Option<u8>,
    pub sort_by: ReplayListSort,
    pub descending: bool,
}

impl ReplayListQuery {
    /// The newest games first.
    pub fn newest_first() -> Self {
        Self {
            descending: true,
            ..Default::default()
        }
    }
}

/// Sent as a request to the backend to copy part the replay path to the system clipboard.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CopyToClipboardContext {
    pub data: String,
}

/// Sent as a request to the backend to copy part the replay path to the system clipboard.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct OpenFolderContext {
    pub folder: String,
}

/// Player names come with the clan tag HTML escaped, i.e. `&lt;CLAN&gt;<sp/>Name`.
pub fn clean_player_name_clan_escapes(player_name: &str) -> String {
    player_name
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("<sp/>", " ")
}

/// The race as stored by localized clients, i.e. Korean, is returned in English.
pub fn normalize_race(race: &str) -> String {
    match race {
        "테란" | "人類" | "人类" | "Терраны" => "Terran",
        "저그" | "蟲族" | "异虫" | "Зерги" => "Zerg",
        "프로토스" | "神族" | "星灵" | "Протоссы" => "Protoss",
        _ => race,
    }
    .to_string()
}
