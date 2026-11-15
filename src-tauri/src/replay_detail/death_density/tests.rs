use super::*;
use crate::data::fixtures::assert_sql_matches_source;
use crate::replay_detail::test_games::fight;
use crate::replay_detail::valued_units::{VALUED_UNITS_IPC, valued_units};

const README: &str = include_str!("README.md");

fn frame() -> Frame {
    Frame {
        left: 0.,
        bottom: 0.,
        right: 100.,
        top: 100.,
    }
}

#[test]
fn the_deaths_make_hills_in_six_classes() {
    let snapshot = fight();
    let cells = shape(
        plan(
            &snapshot.source(),
            1,
            valued_units().unwrap(),
            kernel_offsets().unwrap(),
            frame(),
        )
        .unwrap()
        .collect()
        .unwrap(),
    )
    .unwrap();
    assert!(!cells.is_empty());
    let at = |x: f32, y: f32| {
        cells
            .iter()
            .find(|cell| {
                cell.column == (x / DENSITY_CELL) as u32 && cell.row == (y / DENSITY_CELL) as u32
            })
            .cloned()
    };
    let fight = at(50., 50.).expect("the fight is dense");
    assert_eq!(fight.class, DENSITY_CLASSES, "{fight:?}");
    let home = at(30., 30.).expect("the SCV died at home");
    let overlord = at(10., 90.).expect("the Overlord died far away");
    assert!(fight.density > home.density && home.density > 0.);
    assert!(overlord.density > 0.);
    assert!(
        at(70., 10.).is_none(),
        "nothing died in the bottom right corner"
    );
    // The kernels sum to the six deaths, less what spreads beyond the edges and under the floor.
    let total: f32 = cells.iter().map(|cell| cell.density).sum();
    assert!((4.5..=6.).contains(&total), "{total}");
    // The classes are 1 to 6, each about a sixth of the cells.
    let classes: Vec<u32> = (1..=DENSITY_CLASSES).collect();
    for class in &classes {
        let count = cells.iter().filter(|cell| cell.class == *class).count();
        assert!(
            (count as f32 - cells.len() as f32 / 6.).abs() <= 1.,
            "class {class}: {count} of {}",
            cells.len()
        );
    }
    assert!(
        cells
            .windows(2)
            .all(|pair| (pair[0].row, pair[0].column) < (pair[1].row, pair[1].column))
    );
}

#[test]
fn the_kernel_offsets_reach_three_bandwidths() {
    let offsets = kernel_offsets().unwrap();
    assert_eq!(offsets.height() as i32, (2 * KERNEL_REACH + 1).pow(2));
    assert_eq!(KERNEL_REACH, 14);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = fight();
    let valued = valued_units().unwrap();
    let offsets = kernel_offsets().unwrap();
    let source = snapshot
        .source()
        .with(VALUED_UNITS_IPC, valued.clone())
        .with(OFFSETS_IPC, offsets.clone());
    let plan = plan(&snapshot.source(), 1, valued, offsets, frame()).unwrap();
    assert_sql_matches_source(README, &source, plan, false);
}
