//! Tracker Event registration.
use super::*;
use convert_case::{Case, Casing};
use rerun::RecordingStream;
use s2protocol::{tracker_events::*, SC2Unit, UnitChangeHint};

pub fn register_unit(
    unit: &SC2Unit,
    creator: &Option<SC2Unit>,
    path_suffix: &'static str,
    recording_stream: &RecordingStream,
    tracker_loop: i64,
    unit_tag_index: u32,
) -> Result<(), SwarmyError> {
    let user_id = unit.user_id.unwrap_or(99u8) as i64;
    let unit_pos_x = unit.pos.x();
    let unit_pos_y = unit.pos.y();
    recording_stream.log(
        format!("Unit/{}/{}/{}", unit.name, unit_tag_index, path_suffix),
        &rerun::Points3D::new([(unit_pos_x, unit_pos_y, (tracker_loop as f32 / 100.))])
            //.with_labels([unit.name.clone()])
            //.with_draw_order(tracker_loop as f32)
            //.with_keypoint_ids([unit_tag_index as u64])
            .with_colors([unit.color])
            .with_radii([unit.radius]),
    )?;
    let mut unit_name_trunc = unit.name.clone();
    unit_name_trunc.truncate(8);
    if let Some(creator) = creator {
        let creator_name_trunc = creator.name.clone();
        recording_stream.log(
            format!("Unit/{}/{}/Creator", unit.name, unit_tag_index),
            &rerun::TextLog::new(format!(
                "U:{user_id} [{0:8}@{unit_tag_index:3}] created by {1:8}",
                unit_name_trunc, creator_name_trunc
            ))
            .with_level(rerun::TextLogLevel::TRACE),
        )?;
    }
    let mut path_suffix: String = path_suffix.into();
    path_suffix.truncate(4);
    recording_stream.log(
        format!("Log/{}", path_suffix),
        &rerun::TextLog::new(format!(
            "U:{user_id} [{0:16}@{unit_tag_index:3}] pos: ({unit_pos_x:3},{unit_pos_y:3})",
            unit_name_trunc
        ))
        .with_level(rerun::TextLogLevel::TRACE),
    )?;
    Ok(())
}

pub fn register_unit_init(
    unit_init: &UnitInitEvent,
    change_hint: UnitChangeHint,
    recording_stream: &RecordingStream,
    tracker_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Registered { unit, creator } = change_hint {
        register_unit(
            &unit,
            &creator,
            "Init",
            recording_stream,
            tracker_loop,
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
    tracker_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Registered { unit, creator } = change_hint {
        register_unit(
            &unit,
            &creator,
            "TypeChange",
            recording_stream,
            tracker_loop,
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
    tracker_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Registered { unit, creator } = change_hint {
        register_unit(
            &unit,
            &creator,
            "Born",
            recording_stream,
            tracker_loop,
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
    tracker_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Unregistered { killer, killed } = change_hint {
        let user_id = killed.user_id.unwrap_or(99u8) as i64;
        let mut unit_name_trunc = killed.name.clone();
        unit_name_trunc.truncate(8);
        let unit_tag_index = killed.tag_index;
        recording_stream.log(
            "Log/Died",
            &rerun::TextLog::new(format!(
                "U:{user_id} [{0:8}@{unit_tag_index:3}]",
                unit_name_trunc
            ))
            .with_level(rerun::TextLogLevel::TRACE),
        )?;
        // Clear up the killed unit target
        recording_stream.log(
            format!(
                "Unit/{}/{}/Target",
                killed.name.clone(),
                unit_dead.unit_tag_index
            ),
            &rerun::Clear::recursive(),
        )?;
        // Clear up the killed unit born data
        recording_stream.log(
            format!(
                "Unit/{}/{}/Born",
                killed.name.clone(),
                unit_dead.unit_tag_index
            ),
            &rerun::Clear::recursive(),
        )?;
        // Clear up the killed unit init data
        recording_stream.log(
            format!(
                "Unit/{}/{}/Init",
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
                "Death/{}/{}",
                killed.name,
                unit_tag(unit_dead.unit_tag_index, unit_dead.unit_tag_recycle)
            ),
            &rerun::Points3D::new([(
                unit_dead.x as f32,
                unit_dead.y as f32,
                tracker_loop as f32 / 100.,
            )])
            //.with_instance_keys([unit_tag as u64])
            //.with_labels([killed.name.clone()])
            //.with_draw_order(tracker_loop as f32)
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
                format!("Kills/{}/{}", killer_unit.name, killer_tag),
                &rerun::Points3D::new([(
                    unit_dead.x as f32,
                    unit_dead.y as f32,
                    tracker_loop as f32 / 100.,
                )])
                //.with_labels([killed.name.clone()])
                //.with_draw_order(tracker_loop as f32)
                //.with_instance_keys([unit_tag as u64])
                .with_colors([FREYA_RED])
                .with_radii([0.75]),
            )?;
        } else {
            recording_stream.log(
                format!("Kills/{}", killed.name),
                &rerun::Points3D::new([(
                    unit_dead.x as f32,
                    unit_dead.y as f32,
                    tracker_loop as f32 / 100.,
                )])
                //.with_labels([killed.name.clone()])
                //.with_draw_order(tracker_loop as f32)
                //.with_instance_keys([unit_tag as u64])
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
    tracker_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Positions(units) = change_hint {
        for unit in units {
            register_unit(
                &unit,
                &None,
                "Position",
                recording_stream,
                tracker_loop,
                unit.tag_index,
            )?;
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
    //     rec.log_timeless(
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
    tracker_loop: i64,
) -> Result<(), SwarmyError> {
    match &evt {
        ReplayTrackerEvent::UnitInit(unit_init) => {
            register_unit_init(unit_init, change_hint, recording_stream, tracker_loop)?;
        }
        ReplayTrackerEvent::UnitBorn(unit_born) => {
            register_unit_born(unit_born, change_hint, recording_stream, tracker_loop)?;
        }
        ReplayTrackerEvent::UnitDied(unit_died) => {
            register_unit_died(unit_died, change_hint, recording_stream, tracker_loop)?;
        }
        ReplayTrackerEvent::UnitPosition(unit_pos) => {
            register_unit_position(
                change_hint,
                unit_pos.clone(),
                recording_stream,
                tracker_loop,
            )?;
        }
        ReplayTrackerEvent::PlayerStats(player_stats) => {
            register_player_stats(player_stats, recording_stream)?;
        }
        ReplayTrackerEvent::Upgrade(upgrade) => {
            recording_stream.log(
                "Upgrade",
                &rerun::TextLog::new(format!(
                    "U:{} [{}@{}]",
                    upgrade.player_id, upgrade.upgrade_type_name, upgrade.count
                ))
                .with_level(rerun::TextLogLevel::TRACE),
            )?;
        }
        ReplayTrackerEvent::UnitTypeChange(unit_type_change) => {
            register_unit_type_change(
                unit_type_change,
                change_hint,
                recording_stream,
                tracker_loop,
            )?;
        }
        _ => {}
    }
    Ok(())
}
