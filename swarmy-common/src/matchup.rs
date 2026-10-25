//! The matchups of 1v1 games, as the casters write them: PvT, TvZ, ZvZ, etc.

/// The race names of the replay details, they are localized.
pub const TERRAN: [&str; 8] = [
    "Terran",
    "테란",
    "人类",
    "人類",
    "Терраны",
    "Terraner",
    "Terranie",
    "Terrano",
];
pub const ZERG: [&str; 6] = ["Zerg", "저그", "异虫", "蟲族", "Зерги", "Zergi"];
pub const PROTOSS: [&str; 6] = ["Protoss", "프로토스", "星灵", "神族", "Протоссы", "Protosi"];

/// T, Z or P, `?` for the races of the arcade maps, i.e. "Primal Zerg", or an unknown language.
pub fn race_letter(race: &str) -> char {
    if TERRAN.contains(&race) {
        'T'
    } else if ZERG.contains(&race) {
        'Z'
    } else if PROTOSS.contains(&race) {
        'P'
    } else {
        '?'
    }
}

/// The matchup of the race against the other one, i.e. `PvT` for Protoss vs Terran.
pub fn matchup_label(race: &str, other_race: &str) -> String {
    format!("{}v{}", race_letter(race), race_letter(other_race))
}

/// Whether both races of a matchup label are the same, i.e. `ZvZ`.
pub fn is_mirror_matchup(label: &str) -> bool {
    match label.split_once('v') {
        Some((race, other_race)) => race == other_race,
        None => false,
    }
}

/// The matchup with its races sorted, `TvP` and `PvT` are both `PvT`, empty if it is not a
/// matchup.
pub fn normalize_matchup(label: &str) -> String {
    let Some((race, other_race)) = label.split_once('v') else {
        return String::new();
    };
    let (race, other_race) = (race.to_uppercase(), other_race.to_uppercase());
    if race <= other_race {
        format!("{race}v{other_race}")
    } else {
        format!("{other_race}v{race}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_names_the_matchups() {
        assert_eq!(matchup_label("Protoss", "Terran"), "PvT");
        assert_eq!(matchup_label("Terran", "Zerg"), "TvZ");
        assert_eq!(matchup_label("Zerg", "Zerg"), "ZvZ");
        assert_eq!(matchup_label("", "Zerg"), "?vZ");
        assert_eq!(matchup_label("테란", "Протоссы"), "TvP");
        assert_eq!(matchup_label("Primal Zerg", "神族"), "?vP");
        assert!(is_mirror_matchup("ZvZ"));
        assert!(!is_mirror_matchup("PvT"));
        assert_eq!(normalize_matchup("TvP"), "PvT");
        assert_eq!(normalize_matchup("zvz"), "ZvZ");
        assert_eq!(normalize_matchup(""), "");
    }
}
