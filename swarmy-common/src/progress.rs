//! The progress of the main player of Config over their games: a few numbers of each game, and
//! the goals the player set for them.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A number measured in each game, the higher the better unless [`ProgressMetric::lower_is_better`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressMetric {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub lower_is_better: bool,
}

pub const PROGRESS_METRICS: [ProgressMetric; 4] = [
    ProgressMetric {
        key: "workers_6",
        label: "Workers at 06:00",
        description:
            "The workers mining at 06:00, a steady worker production is the base of the economy",
        lower_is_better: false,
    },
    ProgressMetric {
        key: "income_8",
        label: "Income at 08:00",
        description: "The minerals and vespene collected per minute at 08:00",
        lower_is_better: false,
    },
    ProgressMetric {
        key: "supply_blocked",
        label: "Seconds supply blocked",
        description: "The seconds the supply was full in the whole game, unable to make units",
        lower_is_better: true,
    },
    ProgressMetric {
        key: "army_10",
        label: "Army value at 10:00",
        description: "The minerals and vespene of the army alive at 10:00",
        lower_is_better: false,
    },
];

/// A game of the main player.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProgressGame {
    pub ext_fs_id: u64,
    pub date: String,
    pub map_title: String,
    pub race: String,
    pub result: String,
    /// The players of the other team, as stored in the replay.
    pub opponents: Vec<String>,
    /// At least one opponent is a computer, i.e. Cheater 3 (Insane).
    pub vs_computer: bool,
    /// The values of [`PROGRESS_METRICS`] by key, the games shorter than the minute have none.
    pub values: BTreeMap<String, f64>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProgressReport {
    /// The main player of Config, empty when it is not set.
    pub player: String,
    /// The last games of the player, the oldest first.
    pub games: Vec<ProgressGame>,
    /// The goals by metric key.
    pub goals: BTreeMap<String, f64>,
}
