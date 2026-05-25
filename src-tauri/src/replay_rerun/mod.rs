//! Tracker Event registration.
//!
pub mod unit_colors;
use s2protocol::state::unit_props::*;

pub mod game_events;

use convert_case::{Case, Casing};
use rerun::RecordingStream;
use s2protocol::{SC2Unit, UnitChangeHint, tracker_events::*};
use swarmy_common::SwarmyError;

pub fn register_unit(
    unit: &SC2Unit,
    creator: &Option<SC2Unit>,
    path_suffix: &'static str,
    recording_stream: &RecordingStream,
    unit_tag_index: u32,
) -> Result<(), SwarmyError> {
    let user_id = unit.user_id.unwrap_or(99u8) as i64;
    let player_name = unit.player_name.clone().unwrap_or(String::from("SYS"));
    let unit_pos_x = unit.pos.x();
    let unit_pos_y = unit.pos.y();
    recording_stream.log(
        format!(
            "{}/Unit/{}/{}/{}",
            player_name, unit.name, unit_tag_index, path_suffix
        ),
        &rerun::Points3D::new([(unit_pos_x, unit_pos_y, 0.)])
            .with_colors([unit.color])
            .with_radii([unit.radius]),
    )?;
    let mut unit_name_trunc = unit.name.clone();
    unit_name_trunc.truncate(8);
    recording_stream.log(
        format!(
            "{}/Unit/{}/{}/{}",
            player_name, unit.name, unit_tag_index, path_suffix
        ),
        &rerun::TextLog::new(format!(
            "U:{user_id} [{0:16}@{unit_tag_index:3}] pos: ({unit_pos_x:3},{unit_pos_y:3})",
            unit_name_trunc
        ))
        .with_level(rerun::TextLogLevel::INFO),
    )?;
    if let Some(creator) = creator {
        // Maybe add an arrow  to show the creator unit and the created position.
        recording_stream.log(
            format!(
                "{}/Unit/{}/{}/Creates/{}/{}",
                player_name, creator.name, creator.tag_index, unit.name, unit_tag_index
            ),
            &rerun::TextLog::new(format!(
                "U:{user_id} [{0:8}@{unit_tag_index:3}] created by {1:8}",
                unit_name_trunc, creator.name,
            ))
            .with_level(rerun::TextLogLevel::TRACE),
        )?;
    }
    Ok(())
}

pub fn register_unit_init(
    unit_init: &UnitInitEvent,
    change_hint: UnitChangeHint,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Registered { unit, creator } = change_hint {
        register_unit(
            &unit,
            &creator,
            "Init",
            recording_stream,
            unit_init.unit_tag_index,
        )?;
    } else {
        tracing::info!(
            "UnitInitEvent {:?} with unexpected UnitChangeHint: {:?}",
            unit_init,
            change_hint
        );
    }
    Ok(())
}

pub fn register_unit_type_change(
    unit_type_change: &UnitTypeChangeEvent,
    change_hint: UnitChangeHint,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Registered { unit, creator } = change_hint {
        register_unit(
            &unit,
            &creator,
            "TypeChange",
            recording_stream,
            unit_type_change.unit_tag_index,
        )?;
    } else {
        tracing::info!(
            "UnitTypeChangeEvent {:?} with unexpected UnitChangeHint: {:?}",
            unit_type_change,
            change_hint
        );
    }
    Ok(())
}

pub fn register_unit_born(
    unit_born: &UnitBornEvent,
    change_hint: UnitChangeHint,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Registered { unit, creator } = change_hint {
        register_unit(
            &unit,
            &creator,
            "Born",
            recording_stream,
            unit_born.unit_tag_index,
        )?;
    } else {
        tracing::info!(
            "Unit Born event {:?} with unexpected UnitChangeHint: {:?}",
            unit_born,
            change_hint
        );
    }
    Ok(())
}

pub fn register_unit_died(
    unit_dead: &UnitDiedEvent,
    change_hint: UnitChangeHint,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Unregistered { killer, killed } = change_hint {
        let user_id = killed.user_id.unwrap_or(99u8) as i64;
        let killed_unit_player_name = killed.player_name.clone().unwrap_or(String::from("SYS"));
        let killer_unit_player_name = match &killer {
            Some(killer) => killer.player_name.clone().unwrap_or(String::from("SYS")),
            None => String::from("SYS0"), // Maybe log this as SYS0 and see if it's useful or maybe
                                          // better name comes up later.
        };
        let mut unit_name_trunc = killed.name.clone();
        unit_name_trunc.truncate(8);
        let unit_tag_index = killed.tag_index;
        recording_stream.log(
            format!(
                "{}/Died/Killer/{}:{}/{}",
                killed_unit_player_name, killer_unit_player_name, killed.name, unit_tag_index
            ),
            &rerun::TextLog::new(format!(
                "U:{user_id} [{0:8}@{unit_tag_index:3}]",
                unit_name_trunc
            ))
            .with_level(rerun::TextLogLevel::INFO),
        )?;
        // Clear up the killed unit target
        // TODO: maybe recursive work at the level of the Unit/Tag?
        // no need for both TU and TP if we clear recursively.
        recording_stream.log(
            format!(
                "{}/Unit/{}/{}/TU",
                killed_unit_player_name,
                killed.name.clone(),
                unit_dead.unit_tag_index
            ),
            &rerun::Clear::recursive(),
        )?;
        recording_stream.log(
            format!(
                "{}/Unit/{}/{}/TP",
                killed_unit_player_name,
                killed.name.clone(),
                unit_dead.unit_tag_index
            ),
            &rerun::Clear::recursive(),
        )?;
        // Clear up the killed unit born data
        recording_stream.log(
            format!(
                "{}/Unit/{}/{}/Born",
                killed_unit_player_name,
                killed.name.clone(),
                unit_dead.unit_tag_index
            ),
            &rerun::Clear::recursive(),
        )?;
        // Clear up the killed unit init data
        recording_stream.log(
            format!(
                "{}/Unit/{}/{}/Init",
                killed_unit_player_name,
                killed.name.clone(),
                unit_dead.unit_tag_index
            ),
            &rerun::Clear::recursive(),
        )?;
        // Create a Path for Death so that it can be drawn on its separate pane.
        // TODO: Create a "triangle soup", maybe something with low resolution to show regions of high
        // activity.
        recording_stream.log(
            format!(
                "{}/Death/{}/{}",
                killed_unit_player_name,
                killed.name,
                unit_tag(unit_dead.unit_tag_index, unit_dead.unit_tag_recycle)
            ),
            &rerun::Points3D::new([(unit_dead.x as f32, unit_dead.y as f32, 0.)])
                //.with_labels([killed.name.clone()])
                .with_colors([FREYA_RED])
                .with_radii([0.75]),
        )?;
        tracing::info!("Killer Unit {:?} died at {:?}", killer, unit_dead,);
        if let (Some(unit_killer_tag_index), Some(killer_tag_recycle), Some(killer_unit)) = (
            unit_dead.killer_unit_tag_index,
            unit_dead.killer_unit_tag_recycle,
            killer,
        ) {
            let killer_tag = unit_tag(unit_killer_tag_index, killer_tag_recycle);
            recording_stream.log(
                format!(
                    "{}/Kills/{}/{}",
                    killed_unit_player_name, killer_unit.name, killer_tag
                ),
                &rerun::Points3D::new([(unit_dead.x as f32, unit_dead.y as f32, 0.)])
                    //.with_instance_keys([unit_tag as u64])
                    .with_colors([FREYA_RED])
                    .with_radii([0.75]),
            )?;
        } else {
            recording_stream.log(
                format!("{}/Kills/{}", killed_unit_player_name, killed.name),
                &rerun::Points3D::new([(unit_dead.x as f32, unit_dead.y as f32, 0.)])
                    //.with_labels([killed.name.clone()])
                    .with_colors([FREYA_GREEN])
                    .with_radii([0.75]),
            )?;
        }
    } else {
        tracing::info!(
            "Unit Died event {:?} with unexpected UnitChangeHint: {:?}",
            unit_dead,
            change_hint
        );
    }
    Ok(())
}

pub fn register_unit_position(
    change_hint: UnitChangeHint,
    unit_pos: UnitPositionsEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Positions(units) = change_hint {
        for unit in units {
            register_unit(&unit, &None, "Position", recording_stream, unit.tag_index)?;
        }
    } else {
        tracing::info!(
            "Unit Positions event {:?} with unexpected UnitChangeHint: {:?}",
            unit_pos,
            change_hint
        );
    }
    Ok(())
}

pub fn register_player_stats(
    player_stats: &PlayerStatsEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    // TODO: record timeless the initial setup, at spawn time probably:
    //     rec.log_static(
    //     "TheStat",
    //     &rerun::SeriesPoint::new()
    //         .with_color([255, 0, 0])
    //         .with_colors(user_color(player_stats.player_id as i64)),
    //         .with_name("sin(0.01t)")
    //         .with_marker(rerun::components::MarkerShape::Circle)
    //         .with_marker_size(4.0),
    // )?;
    for stat_entity_value in player_stats.stats.as_prop_name_value_vec() {
        println!("Stat: {}", stat_entity_value.0);
        let entity_path = stat_entity_value.0.replace('/', "_").to_case(Case::Pascal);
        recording_stream.log(
            format!("{}/{}", entity_path, player_stats.player_id),
            &rerun::Scalars::new([stat_entity_value.1 as f64]),
        )?;
    }
    Ok(())
}

/// Registers the tracker events to Rerun.
pub fn add_tracker_event(
    evt: &ReplayTrackerEvent,
    change_hint: UnitChangeHint,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    match &evt {
        ReplayTrackerEvent::UnitInit(unit_init) => {
            register_unit_init(unit_init, change_hint, recording_stream)?;
        }
        ReplayTrackerEvent::UnitBorn(unit_born) => {
            register_unit_born(unit_born, change_hint, recording_stream)?;
        }
        ReplayTrackerEvent::UnitDied(unit_died) => {
            register_unit_died(unit_died, change_hint, recording_stream)?;
        }
        ReplayTrackerEvent::UnitPosition(unit_pos) => {
            register_unit_position(change_hint, unit_pos.clone(), recording_stream)?;
        }
        ReplayTrackerEvent::PlayerStats(player_stats) => {
            register_player_stats(player_stats, recording_stream)?;
        }
        ReplayTrackerEvent::Upgrade(upgrade) => {
            // For some reason this is not matching, all shows up as SYS.
            recording_stream.log(
                format!(
                    "{}/Upgrade/{}",
                    upgrade.player_name.clone().unwrap_or(String::from("SYS")),
                    upgrade.upgrade_type_name
                ),
                &rerun::TextLog::new(format!(
                    "U:{} [{}@{}]",
                    upgrade.player_id, upgrade.upgrade_type_name, upgrade.count
                ))
                .with_level(rerun::TextLogLevel::TRACE),
            )?;
        }
        ReplayTrackerEvent::UnitTypeChange(unit_type_change) => {
            register_unit_type_change(unit_type_change, change_hint, recording_stream)?;
        }
        _ => {}
    }
    Ok(())
}
