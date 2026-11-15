use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::patch_overview::unit_stats::test_games::forms;

const README: &str = include_str!("README.md");

fn change(from: &str, to: &str, changes: u32) -> UnitFormChange {
    UnitFormChange {
        from: from.to_string(),
        to: to.to_string(),
        changes,
    }
}

#[test]
fn the_types_the_units_change_into_by_patch() {
    let snapshot = forms();
    let by_patch = shape(plan(&snapshot.source()).unwrap().collect().unwrap()).unwrap();
    // The games of the fixtures are of protocol 95248, a patch of the balance data.
    assert_eq!(
        by_patch,
        PatchForms::from([(
            95_248,
            vec![
                change("Roach", "RoachBurrowed", 2),
                change("RoachBurrowed", "Roach", 1),
                change("SiegeTank", "SiegeTankSieged", 1),
            ]
        )])
    );
}

#[test]
fn the_forms_of_a_unit_reach_through_other_forms() {
    let changes = [
        change("Roach", "RoachBurrowed", 2),
        change("RoachBurrowed", "Roach", 1),
        change("SiegeTank", "SiegeTankSieged", 1),
    ];
    // The forms of the Roach, the Siege Tank's are another unit.
    let roach = forms_of("Roach", &changes);
    assert_eq!(roach.len(), 2);
    assert!(roach.iter().all(|c| c.from != "SiegeTank"));
    // The forms reach through another form; the rare pairs are noise.
    let chain = [
        change("Hatchery", "Lair", 500),
        change("Lair", "Hive", 200),
        change("Hatchery", "Lair2", 1),
        change("Roach", "RoachBurrowed", 9),
    ];
    assert_eq!(forms_of("Hatchery", &chain).len(), 2);
    // Only forward: what a unit morphs from is another unit.
    assert_eq!(forms_of("Hive", &chain).len(), 0);
    assert_eq!(forms_of("Lair", &chain).len(), 1);
}

#[test]
fn only_the_units_of_the_balance_data_are_forms() {
    let found = PatchForms::from([(
        98_310,
        vec![
            change("Hatchery", "Lair", 500),
            // A building of a custom map with the name of a melee one.
            change("Hatchery", "Lair2", 3),
            change("SiegeTank", "SiegeTankSieged", 40),
        ],
    )]);
    let valid = validate(found).unwrap();
    assert_eq!(
        valid[&98_310],
        vec![
            change("Hatchery", "Lair", 500),
            change("SiegeTank", "SiegeTankSieged", 40)
        ]
    );
}

#[test]
fn a_patch_without_its_table_takes_the_one_before() {
    let tables = PatchForms::from([
        (95_248, vec![change("Roach", "RoachBurrowed", 1)]),
        (98_310, vec![change("Hatchery", "Lair", 1)]),
    ]);
    assert_eq!(table_of(&tables, 98_310)[0].to, "Lair");
    assert_eq!(table_of(&tables, 97_563)[0].to, "RoachBurrowed");
    // Older than every table: the oldest one.
    assert_eq!(table_of(&tables, 75_689)[0].to, "RoachBurrowed");
    assert!(table_of(&PatchForms::new(), 98_310).is_empty());
}

/// The table compiled in: the newest patch of the balance data has its morphs (a new patch
/// fails here until `write_unit_forms` is run for it), and every pair is valid.
#[test]
fn the_static_table_covers_the_newest_patch() {
    let tables = static_forms().unwrap();
    let latest = *patch_balance_versions().unwrap().last().unwrap();
    assert!(
        tables.contains_key(&latest),
        "unit_forms.json has no table for patch {latest}: run write_unit_forms (forms/README.md)"
    );
    assert_eq!(&validate(tables.clone()).unwrap(), tables);
    let latest = latest_forms().unwrap();
    for (from, to) in [
        ("Roach", "RoachBurrowed"),
        ("SiegeTank", "SiegeTankSieged"),
        ("Hatchery", "Lair"),
        ("CommandCenter", "OrbitalCommand"),
    ] {
        assert!(
            latest.iter().any(|c| c.from == from && c.to == to),
            "{from} -> {to} missing from the newest patch"
        );
    }
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = forms();
    let plan = plan(&snapshot.source()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}

/// Writes `unit_forms.json` from a large snapshot, when a patch comes out (forms/README.md):
///
///     SWARMY_TEST_REPLAYS=~/SCReplaysOnNVMe cargo test --release --lib write_unit_forms -- --ignored --nocapture
#[test]
#[ignore = "needs SWARMY_TEST_REPLAYS, a Replay Path with a large snapshot; writes unit_forms.json"]
fn write_unit_forms() {
    let replays = std::env::var("SWARMY_TEST_REPLAYS").unwrap();
    let source = IpcSnapshot::open(&replays).unwrap();
    let forms = generate(source.ipc_path()).unwrap();
    for (patch, changes) in &forms {
        println!("patch {patch}: {} pairs", changes.len());
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/patch_overview/unit_stats/forms/unit_forms.json");
    std::fs::write(&path, serde_json::to_string_pretty(&forms).unwrap() + "\n").unwrap();
    println!("written {}", path.display());
}
