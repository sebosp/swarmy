//! Tracker Event registration.
use super::*;
use convert_case::{Case, Casing};
use rerun::{
    components::{Point3D, Radius, Scalar, TextEntry},
    MsgSender,
};
use s2protocol::tracker_events::*;

pub fn register_unit_init(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    unit_init: &UnitInitEvent,
    updated_units: Vec<u32>,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    for unit_tag in updated_units {
        if let Some(unit) = sc2_rerun.sc2_state.units.get(&unit_tag) {
            let (unit_size, unit_color) =
                get_unit_sized_color(&unit.name, unit.user_id.unwrap_or(99u8) as i64);
            let unit_pos = from_vec3d(unit.pos);
            MsgSender::new(format!("Unit/{}/Init", unit_init.unit_tag_index))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Point3D::new(unit_pos.x(), unit_pos.y(), unit_pos.z()))?
                .with_splat(unit_color)?
                .with_splat(TextEntry::new(&unit.name, None))?
                .with_splat(Radius(unit_size))?
                .send(recording_stream)?;
        }
    }
    Ok(())
}

pub fn register_unit_born(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    unit_born: &UnitBornEvent,
    updated_units: Vec<u32>,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    for unit_tag in updated_units {
        if let Some(unit) = sc2_rerun.sc2_state.units.get(&unit_tag) {
            let (unit_size, unit_color) =
                get_unit_sized_color(&unit.name, unit.user_id.unwrap_or(99u8) as i64);
            let unit_pos = from_vec3d(unit.pos);
            MsgSender::new(format!("Unit/{}/Born", unit_born.unit_tag_index,))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Point3D::new(unit_pos.x(), unit_pos.y(), unit_pos.z()))?
                .with_splat(unit_color)?
                .with_splat(TextEntry::new(&unit.name, None))?
                .with_splat(Radius(unit_size))?
                .send(recording_stream)?;
        }
    }
    Ok(())
}

pub fn register_unit_died(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    unit_dead: &UnitDiedEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    // Clear up the previous unit target.
    recording_stream.record_path_op(rerun::log::PathOp::clear(
        true,
        format!("Unit/{}/Target", unit_dead.unit_tag_index).into(),
    ));
    recording_stream.record_path_op(rerun::log::PathOp::clear(
        true,
        format!("Unit/{}/Born", unit_dead.unit_tag_index).into(),
    ));
    // Create a Path for Death so that it can be drawn on its separate pane.
    // TODO: Create a "triangle soup", maybe something with low resolution to show regions of high
    // activity.
    MsgSender::new(format!(
        "Death/{}/{}",
        unit_tag(unit_dead.unit_tag_index, unit_dead.unit_tag_recycle),
        game_loop
    ))
    .with_time(sc2_rerun.timeline, game_loop)
    .with_splat(Point3D::new(
        unit_dead.x as f32,
        -1. * unit_dead.y as f32,
        game_loop as f32 / 100.,
    ))?
    .with_splat(FREYA_DARK_RED)?
    .with_splat(Radius(0.75))?
    .send(recording_stream)?;
    Ok(())
}

pub fn register_unit_position(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    unit_pos: UnitPositionsEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    let unit_positions = unit_pos.to_unit_positions_vec();
    for unit_pos_item in unit_positions {
        if let Some(sc2_unit) = sc2_rerun.sc2_state.units.get(&unit_pos_item.tag) {
            let unit_pos = from_vec3d(sc2_unit.pos);
            MsgSender::new(format!("Unit/{}/Position", unit_pos_item.tag))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Point3D::new(unit_pos.x(), unit_pos.y(), unit_pos.z()))?
                .send(recording_stream)?;
        } else {
            tracing::error!(
                "Unit {} did not exist but position registered.",
                unit_pos_item.tag
            );
        }
    }
    Ok(())
}

pub fn register_player_stats(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    player_stats: &PlayerStatsEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if !sc2_rerun.sc2_state.include_stats {
        return Ok(());
    }
    for stat_entity_value in player_stats.stats.as_prop_name_value_vec() {
        println!("Stat: {}", stat_entity_value.0);
        let entity_path = stat_entity_value.0.replace('/', "_").to_case(Case::Pascal);
        MsgSender::new(format!("{}/{}", entity_path, player_stats.player_id,))
            .with_time(sc2_rerun.timeline, game_loop)
            .with_splat(Scalar::from(stat_entity_value.1 as f64))?
            .with_splat(user_color(player_stats.player_id as i64))?
            .send(recording_stream)?;
    }
    Ok(())
}

/// Registers the tracker events to Rerun.
pub fn add_tracker_event(
    sc2_rerun: &SC2Rerun,
    tracker_loop: i64,
    evt: &ReplayTrackerEvent,
    updated_units: Vec<u32>,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    match &evt {
        ReplayTrackerEvent::UnitInit(unit_init) => {
            register_unit_init(
                sc2_rerun,
                tracker_loop,
                unit_init,
                updated_units,
                recording_stream,
            )?;
        }
        ReplayTrackerEvent::UnitBorn(unit_born) => {
            register_unit_born(
                sc2_rerun,
                tracker_loop,
                unit_born,
                updated_units,
                recording_stream,
            )?;
        }
        ReplayTrackerEvent::UnitDied(unit_died) => {
            register_unit_died(sc2_rerun, tracker_loop, unit_died, recording_stream)?;
        }
        ReplayTrackerEvent::UnitPosition(unit_pos) => {
            register_unit_position(sc2_rerun, tracker_loop, unit_pos.clone(), recording_stream)?;
        }
        ReplayTrackerEvent::PlayerStats(player_stats) => {
            register_player_stats(sc2_rerun, tracker_loop, player_stats, recording_stream)?;
        }
        _ => {}
    }
    Ok(())
}
