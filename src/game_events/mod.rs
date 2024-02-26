//! Game Events drawing

use super::*;
use s2protocol::game_events::*;
use s2protocol::UnitChangeHint;

pub fn register_camera_update(
    user_id: i64,
    camera_update: &CameraUpdateEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if let Some(target) = &camera_update.m_target {
        recording_stream.log(
            "batch",
            &rerun::Boxes3D::from_centers_and_half_sizes(
                [(
                    target.x as f32 / 250f32,
                    -1. * target.y as f32 / 250f32,
                    0.0,
                )],
                [(1.0, 1.0, 0.0)],
            )
            .with_radii([1.5])
            .with_colors([user_color(user_id)])
            .with_labels([user_id.to_string()]),
        )?;
    }
    Ok(())
}

/// Draw an arrow from the unit to the target point.
pub fn register_update_target_point(
    user_id: i64,
    change_hint: UnitChangeHint,
    target_point: &GameSMapCoord3D,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    let unit_target_pos = rerun::Vec3D::new(
        target_point.x as f32 / GAME_EVENT_POS_RATIO,
        -1. * target_point.y as f32 / GAME_EVENT_POS_RATIO,
        target_point.z as f32 / GAME_EVENT_POS_RATIO,
    );
    if let UnitChangeHint::Batch(updated_units) = change_hint {
        for selected_unit in updated_units {
            let selected_unit_pos = rerun::Vec3D::new(
                selected_unit.pos.x(),
                selected_unit.pos.y(),
                selected_unit.pos.z(),
            );
            recording_stream.log(
                format!(
                    "Unit/{}/{}/Target",
                    selected_unit.name, selected_unit.tag_index
                ),
                &rerun::Arrows3D::from_vectors([unit_target_pos])
                    .with_origins([selected_unit_pos])
                    .with_colors([user_color(user_id)]),
            )?;
        }
    }
    Ok(())
}

pub fn register_update_target_unit(
    user_id: i64,
    change_hint: UnitChangeHint,
    target_unit: &GameSCmdDataTargetUnit,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    let unit_target_pos = rerun::Vec3D::new(
        target_unit.m_snapshot_point.x as f32 / GAME_EVENT_POS_RATIO,
        -1. * target_unit.m_snapshot_point.y as f32 / GAME_EVENT_POS_RATIO,
        target_unit.m_snapshot_point.z as f32 / GAME_EVENT_POS_RATIO,
    );
    if let UnitChangeHint::BatchWithTarget(user_selected_units, _target_unit) = change_hint {
        for selected_unit in user_selected_units {
            let selected_unit_pos = rerun::Vec3D::new(
                selected_unit.pos.x(),
                selected_unit.pos.y(),
                selected_unit.pos.z(),
            );
            recording_stream.log(
                format!(
                    "Unit/{}/{}/Target",
                    selected_unit.name, selected_unit.tag_index
                ),
                &rerun::Arrows3D::from_vectors([unit_target_pos])
                    .with_origins([selected_unit_pos])
                    .with_colors([user_color(user_id)]),
            )?;
        }
    }
    Ok(())
}

/// Registers units as being selected.
/// The radius is adjusted on s2protocol side.
/// The event could be for a non-selected group, for example, a unit in a group may have died
/// and that would trigger a selection delta. Same if a unit as Larva is part of a group and
/// then it is born into another unit which triggers a selection delta.
/// In the rust version we are matching the ACTIVE_UNITS_GROUP_IDX to 10, the last item in the
/// array of selceted units which seems to match the blizzard UI so far.
pub fn register_selection_delta(
    change_hint: UnitChangeHint,
    recording_stream: &RecordingStream,
    game_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Batch(changed_units) = change_hint {
        for unit in changed_units {
            // XXX: Technically this is not "Born", we should have a State or Status that
            // contains the radius of the unit.
            recording_stream.log(
                format!("Unit/{}/{}/Born", unit.name, unit.tag_index),
                &rerun::Points2D::new([(unit.pos.x(), unit.pos.y())])
                    .with_radii([unit.radius])
                    .with_draw_order(game_loop as f32),
            )?;
        }
    }
    Ok(())
}

/// Handles control group update events
/// These may be initializing or recalled
pub fn update_control_group(
    change_hint: UnitChangeHint,
    ctrl_group_evt: &GameSControlGroupUpdateEvent,
    recording_stream: &RecordingStream,
    game_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Batch(changed_units) = change_hint {
        if ctrl_group_evt.m_control_group_update == GameEControlGroupUpdate::ERecall {
            for unit in changed_units {
                recording_stream.log(
                    format!("Unit/{}/{}/Born", unit.name, unit.tag_index),
                    &rerun::Points2D::new([(unit.pos.x(), unit.pos.y())])
                        .with_radii([unit.radius])
                        .with_draw_order(game_loop as f32),
                )?;
            }
        }
    }
    Ok(())
}

pub fn register_cmd(
    user_id: i64,
    change_hint: UnitChangeHint,
    game_cmd: &GameSCmdEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    match &game_cmd.m_data {
        GameSCmdData::TargetPoint(target) => {
            register_update_target_point(user_id, change_hint, target, recording_stream)?;
        }
        GameSCmdData::TargetUnit(target_unit) => {
            register_update_target_unit(user_id, change_hint, target_unit, recording_stream)?;
        }
        GameSCmdData::Data(data) => {
            tracing::info!("GameSCmdData: {}", data);
        }
        GameSCmdData::None => {}
    }
    Ok(())
}

/// Registers the game events to Rerun.
pub fn add_game_event(
    user_id: i64,
    evt: &ReplayGameEvent,
    change_hint: UnitChangeHint,
    recording_stream: &RecordingStream,
    game_loop: i64,
) -> Result<(), SwarmyError> {
    match &evt {
        ReplayGameEvent::CameraUpdate(camera_update) => {
            register_camera_update(user_id, camera_update, recording_stream)?;
        }
        ReplayGameEvent::Cmd(game_cmd) => {
            register_cmd(user_id, change_hint, game_cmd, recording_stream)?;
        }
        ReplayGameEvent::CmdUpdateTargetPoint(target_point) => {
            register_update_target_point(
                user_id,
                change_hint,
                &target_point.m_target,
                recording_stream,
            )?;
        }
        ReplayGameEvent::CmdUpdateTargetUnit(target_unit) => {
            register_update_target_unit(
                user_id,
                change_hint,
                &target_unit.m_target,
                recording_stream,
            )?;
        }
        ReplayGameEvent::SelectionDelta(_selection_delta) => {
            register_selection_delta(change_hint, recording_stream, game_loop)?;
        }
        ReplayGameEvent::ControlGroupUpdate(ctrl_group) => {
            update_control_group(change_hint, ctrl_group, recording_stream, game_loop)?;
        }
        _ => {}
    }
    Ok(())
}
