//! Game Events drawing

use super::*;
use rerun::components::Arrow3D;
use rerun::transform::TranslationRotationScale3D;
use rerun::{
    components::{Box3D, Radius, Transform3D, Vec3D},
    MsgSender,
};
use s2protocol::game_events::*;
use s2protocol::tracker_events::unit_tag_index;
use s2protocol::ACTIVE_UNITS_GROUP_IDX;

pub fn register_camera_update(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    camera_update: &CameraUpdateEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if let Some(target) = &camera_update.m_target {
        MsgSender::new(format!("Unit/999{}/Player", user_id))
            .with_time(sc2_rerun.timeline, game_loop)
            .with_splat(Box3D::new(3.0, 3.0, 0.0))?
            .with_splat(Transform3D::new(TranslationRotationScale3D {
                translation: Some(
                    [
                        (target.x as f32 / 250f32) - 1.5,
                        (-1. * target.y as f32 / 250f32) - 1.5,
                        0.,
                    ]
                    .into(),
                ),
                rotation: None,
                scale: None,
            }))?
            .with_splat(user_color(user_id))?
            .send(recording_stream)?;
    }
    Ok(())
}

pub fn register_cmd(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    game_cmd: &GameSCmdEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    match &game_cmd.m_data {
        GameSCmdData::TargetPoint(target) => {
            register_update_target_point(sc2_rerun, game_loop, user_id, target, recording_stream)?;
        }
        GameSCmdData::TargetUnit(target_unit) => {
            register_update_target_unit(
                sc2_rerun,
                game_loop,
                user_id,
                target_unit,
                recording_stream,
            )?;
        }
        GameSCmdData::Data(data) => {
            tracing::info!("GameSCmdData: {}", data);
        }
        GameSCmdData::None => {}
    }
    Ok(())
}

pub fn register_update_target_point(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    target_point: &GameSMapCoord3D,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    let unit_target_pos = Vec3D::new(
        target_point.x as f32 / GAME_EVENT_POS_RATIO,
        -1. * target_point.y as f32 / GAME_EVENT_POS_RATIO,
        target_point.z as f32 / GAME_EVENT_POS_RATIO,
    );
    let mut user_selected_units: Vec<u32> = vec![];
    if let Some(state) = sc2_rerun.sc2_state.user_state.get(&user_id) {
        user_selected_units = state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
    }
    for selected_unit in user_selected_units {
        let unit_index = unit_tag_index(selected_unit as i64);
        if let Some(registered_unit) = sc2_rerun.sc2_state.units.get(&unit_index) {
            let registered_unit_pos = from_vec3d(registered_unit.pos);
            MsgSender::new(format!("Unit/{}/Target", unit_index))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Arrow3D {
                    origin: registered_unit_pos,
                    vector: Vec3D::new(
                        unit_target_pos.x() - registered_unit_pos.x(),
                        unit_target_pos.y() - registered_unit_pos.y(),
                        unit_target_pos.z() - registered_unit_pos.z(),
                    ),
                })?
                .with_splat(user_color(user_id))?
                .send(recording_stream)?;
        }
    }
    Ok(())
}

pub fn register_update_target_unit(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    target_unit: &GameSCmdDataTargetUnit,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    let unit_target_pos = Vec3D::new(
        target_unit.m_snapshot_point.x as f32 / GAME_EVENT_POS_RATIO,
        -1. * target_unit.m_snapshot_point.y as f32 / GAME_EVENT_POS_RATIO,
        target_unit.m_snapshot_point.z as f32 / GAME_EVENT_POS_RATIO,
    );
    let mut user_selected_units: Vec<u32> = vec![];
    if let Some(state) = sc2_rerun.sc2_state.user_state.get(&user_id) {
        user_selected_units = state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
    }
    for selected_unit in user_selected_units {
        let unit_index = unit_tag_index(selected_unit as i64);
        if let Some(registered_unit) = sc2_rerun.sc2_state.units.get(&unit_index) {
            let registered_unit_pos = from_vec3d(registered_unit.pos);
            MsgSender::new(format!("Unit/{}/Target", unit_index))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Arrow3D {
                    origin: registered_unit_pos,
                    vector: Vec3D::new(
                        unit_target_pos.x() - registered_unit_pos.x(),
                        unit_target_pos.y() - registered_unit_pos.y(),
                        unit_target_pos.z() - registered_unit_pos.z(),
                    ),
                })?
                .with_splat(user_color(user_id))?
                .send(recording_stream)?;
        }
    }
    Ok(())
}

/// Removes the changes to the units that signify they are selected.
pub fn unmark_previously_selected_units(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if let Some(state) = sc2_rerun.sc2_state.user_state.get(&user_id) {
        for prev_unit in &state.control_groups[ACTIVE_UNITS_GROUP_IDX] {
            let unit_index = unit_tag_index(*prev_unit as i64);
            if let Some(unit) = sc2_rerun.sc2_state.units.get(&unit_index) {
                // Decrease the previous units radius increment.
                // XXX: Technically this is not "Born", we should have a State or Status that
                // contains the radius of the unit.
                MsgSender::new(format!("Unit/{}/Born", unit_index))
                    .with_time(sc2_rerun.timeline, game_loop)
                    .with_splat(Radius(unit.radius))?
                    .send(recording_stream)?;
            }
        }
    }
    Ok(())
}

/// Marks a group of units as selected by increasing the radius.
pub fn mark_selected_units(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    _user_id: i64,
    selected_units: &[u32],
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    for new_selected_unit in selected_units {
        let unit_index = unit_tag_index(*new_selected_unit as i64);
        if let Some(unit) = sc2_rerun.sc2_state.units.get(&unit_index) {
            // Increase the previous units radius increment.
            // XXX: Technically this is not "Born", we should have a State or Status that
            // contains the radius of the unit.
            MsgSender::new(format!("Unit/{}/Born", unit_index))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Radius(unit.radius))?
                .send(recording_stream)?;
        }
    }
    Ok(())
}

/// Registers units as being selected.
/// The previous selected units radius is decreased to its normal state.
/// The new selected units radius is increased.
/// The event could be for a non-selected group, for example, a unit in a group may have died
/// and that would trigger a selection delta. Same if a unit as Larva is part of a group and
/// then it is born into another unit which triggers a selection delta.
/// In the rust version we are matching the ACTIVE_UNITS_GROUP_IDX to 10, the last item in the
/// array of selceted units which seems to match the blizzard UI so far.
pub fn register_selection_delta(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    selection_delta: &GameSSelectionDeltaEvent,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    if selection_delta.m_control_group_id == ACTIVE_UNITS_GROUP_IDX as u8 {
        unmark_previously_selected_units(sc2_rerun, game_loop, user_id, recording_stream)?;
        mark_selected_units(
            sc2_rerun,
            game_loop,
            user_id,
            &selection_delta.m_delta.m_add_unit_tags,
            recording_stream,
        )?;
    }
    Ok(())
}

/// Handles control group update events
/// These may be initializing or recalled
pub fn update_control_group(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    ctrl_group_evt: &GameSControlGroupUpdateEvent,
    updated_units: Vec<u32>,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    unmark_previously_selected_units(sc2_rerun, game_loop, user_id, recording_stream)?;
    if ctrl_group_evt.m_control_group_update == GameEControlGroupUpdate::ERecall {
        mark_selected_units(
            sc2_rerun,
            game_loop,
            user_id,
            &updated_units,
            recording_stream,
        )?;
    }
    Ok(())
}

/// Registers the game events to Rerun.
pub fn add_game_event(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    evt: &ReplayGameEvent,
    updated_units: Vec<u32>,
    recording_stream: &RecordingStream,
) -> Result<(), SwarmyError> {
    match &evt {
        ReplayGameEvent::CameraUpdate(camera_update) => {
            register_camera_update(
                sc2_rerun,
                game_loop,
                user_id,
                camera_update,
                recording_stream,
            )?;
        }
        ReplayGameEvent::Cmd(game_cmd) => {
            register_cmd(sc2_rerun, game_loop, user_id, game_cmd, recording_stream)?;
        }
        ReplayGameEvent::CmdUpdateTargetPoint(target_point) => {
            register_update_target_point(
                sc2_rerun,
                game_loop,
                user_id,
                &target_point.m_target,
                recording_stream,
            )?;
        }
        ReplayGameEvent::CmdUpdateTargetUnit(target_unit) => {
            register_update_target_unit(
                sc2_rerun,
                game_loop,
                user_id,
                &target_unit.m_target,
                recording_stream,
            )?;
        }
        ReplayGameEvent::SelectionDelta(selection_delta) => {
            register_selection_delta(
                sc2_rerun,
                game_loop,
                user_id,
                selection_delta,
                recording_stream,
            )?;
        }
        ReplayGameEvent::ControlGroupUpdate(ctrl_group) => {
            update_control_group(
                sc2_rerun,
                game_loop,
                user_id,
                ctrl_group,
                updated_units,
                recording_stream,
            )?;
        }
        _ => {}
    }
    Ok(())
}
