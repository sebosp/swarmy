//! Game Events drawing

use super::*;
use rerun::components::Arrow3D;
use rerun::{
    components::{Box3D, Quaternion, Radius, Rigid3, Transform, Vec3D},
    MsgSender,
};
use s2protocol::game_events::*;
use s2protocol::tracker_events::unit_tag_index;

pub fn register_camera_update(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    camera_update: &CameraUpdateEvent,
) -> Result<(), SwarmyError> {
    if let Some(target) = &camera_update.m_target {
        MsgSender::new(format!("Unit/999{}/Player", user_id))
            .with_time(sc2_rerun.timeline, game_loop)
            .with_splat(Box3D::new(3.0, 3.0, 0.0))?
            .with_splat(Transform::Rigid3(Rigid3 {
                rotation: Quaternion::new(0., 0., 0., 0.),
                translation: Vec3D::new(
                    (target.x as f32 / 250f32) - 1.5,
                    (-1. * target.y as f32 / 250f32) - 1.5,
                    0.,
                ),
            }))?
            .with_splat(user_color(user_id))?
            .send(&sc2_rerun.rerun_session)?;
    }
    Ok(())
}

pub fn register_cmd(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    user_id: i64,
    game_cmd: &GameSCmdEvent,
) -> Result<(), SwarmyError> {
    match &game_cmd.m_data {
        GameSCmdData::TargetPoint(target) => {
            /*MsgSender::new(format!("Target/{}/Camera", user_id))
            .with_time(sc2_rerun.timeline, game_loop)
            .with_splat(Point3D::new(
                target.x as f32 / GAME_EVENT_POS_RATIO,
                -1. * target.y as f32 / GAME_EVENT_POS_RATIO,
                target.z as f32 / GAME_EVENT_POS_RATIO,
            ))?
            .with_splat(user_color(user_id))?
            .with_splat(Radius(3.0))?
            .send(&sc2_rerun.rerun_session)?;*/
        }
        GameSCmdData::TargetUnit(target_unit) => {
            register_update_target_unit(sc2_rerun, game_loop, user_id, target_unit)?;
        }
        GameSCmdData::Data(data) => {
            tracing::info!("GameSCmdData: {}", data);
        }
        GameSCmdData::None => {}
    }
    Ok(())
}

pub fn register_update_target_point(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    user_id: i64,
    target_point: &GameSCmdUpdateTargetPointEvent,
) -> Result<(), SwarmyError> {
    let unit_target_pos = Vec3D::new(
        target_point.m_target.x as f32 / GAME_EVENT_POS_RATIO,
        -1. * target_point.m_target.y as f32 / GAME_EVENT_POS_RATIO,
        target_point.m_target.z as f32 / GAME_EVENT_POS_RATIO,
    );
    let mut user_selected_units: Vec<u32> = vec![];
    if let Some(state) = sc2_rerun.user_state.get(&user_id) {
        user_selected_units = state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
    }
    for selected_unit in user_selected_units {
        let unit_index = unit_tag_index(selected_unit as i64);
        if let Some(ref mut registered_unit) = sc2_rerun.units.get_mut(&unit_index) {
            registered_unit.target = Some(unit_target_pos);
            MsgSender::new(format!("Unit/{}/Target", unit_index))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Arrow3D {
                    origin: registered_unit.pos,
                    vector: Vec3D::new(
                        unit_target_pos.x() - registered_unit.pos.x(),
                        unit_target_pos.y() - registered_unit.pos.y(),
                        unit_target_pos.z() - registered_unit.pos.z(),
                    ),
                })?
                .with_splat(user_color(user_id))?
                .send(&sc2_rerun.rerun_session)?;
        }
    }
    Ok(())
}

pub fn register_update_target_unit(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    user_id: i64,
    target_unit: &GameSCmdDataTargetUnit,
) -> Result<(), SwarmyError> {
    let unit_target_pos = Vec3D::new(
        target_unit.m_snapshot_point.x as f32 / GAME_EVENT_POS_RATIO,
        -1. * target_unit.m_snapshot_point.y as f32 / GAME_EVENT_POS_RATIO,
        target_unit.m_snapshot_point.z as f32 / GAME_EVENT_POS_RATIO,
    );
    let mut user_selected_units: Vec<u32> = vec![];
    if let Some(state) = sc2_rerun.user_state.get(&user_id) {
        user_selected_units = state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
    }
    for selected_unit in user_selected_units {
        let unit_index = unit_tag_index(selected_unit as i64);
        if let Some(ref mut registered_unit) = sc2_rerun.units.get_mut(&unit_index) {
            registered_unit.target = Some(unit_target_pos);
            MsgSender::new(format!("Unit/{}/Target", unit_index))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Arrow3D {
                    origin: registered_unit.pos,
                    vector: Vec3D::new(
                        unit_target_pos.x() - registered_unit.pos.x(),
                        unit_target_pos.y() - registered_unit.pos.y(),
                        unit_target_pos.z() - registered_unit.pos.z(),
                    ),
                })?
                .with_splat(user_color(user_id))?
                .send(&sc2_rerun.rerun_session)?;
        }
    }
    Ok(())
}

/// Removes the changes to the units that signify they are selected.
pub fn unmark_previously_selected_units(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    user_id: i64,
) -> Result<(), SwarmyError> {
    if let Some(state) = sc2_rerun.user_state.get_mut(&user_id) {
        for prev_unit in &state.control_groups[ACTIVE_UNITS_GROUP_IDX] {
            let unit_index = unit_tag_index(*prev_unit as i64);
            if let Some(ref mut unit) = sc2_rerun.units.get_mut(&unit_index) {
                if unit.is_selected {
                    unit.is_selected = false;
                    unit.radius = unit.radius * 0.5;
                }
                // Decrease the previous units radius increment.
                // XXX: Technically this is not "Born", we should have a State or Status that
                // contains the radius of the unit.
                MsgSender::new(format!("Unit/{}/Born", unit_index))
                    .with_time(sc2_rerun.timeline, game_loop)
                    .with_splat(Radius(unit.radius))?
                    .send(&sc2_rerun.rerun_session)?;
            }
        }
    }
    Ok(())
}

/// Marks a group of units as selected by increasing the radius.
pub fn mark_selected_units(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    _user_id: i64,
    selected_units: &[u32],
) -> Result<(), SwarmyError> {
    for new_selected_unit in selected_units {
        let unit_index = unit_tag_index(*new_selected_unit as i64);
        if let Some(ref mut unit) = sc2_rerun.units.get_mut(&unit_index) {
            if !unit.is_selected {
                unit.is_selected = true;
                unit.radius = unit.radius * 2.0;
            }
            // Increase the previous units radius increment.
            // XXX: Technically this is not "Born", we should have a State or Status that
            // contains the radius of the unit.
            MsgSender::new(format!("Unit/{}/Born", unit_index))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Radius(unit.radius))?
                .send(&sc2_rerun.rerun_session)?;
        }
    }
    Ok(())
}

/// Registers units as being selected.
/// The previous selected units radius is decreased to its normal state.
/// The new selected units radius is increased.
pub fn register_selection_delta(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    user_id: i64,
    selection_delta: &GameSSelectionDeltaEvent,
) -> Result<(), SwarmyError> {
    unmark_previously_selected_units(sc2_rerun, game_loop, user_id)?;
    if let Some(ref mut state) = sc2_rerun.user_state.get_mut(&user_id) {
        state.control_groups[ACTIVE_UNITS_GROUP_IDX] =
            selection_delta.m_delta.m_add_unit_tags.clone();
    }
    mark_selected_units(
        sc2_rerun,
        game_loop,
        user_id,
        &selection_delta.m_delta.m_add_unit_tags,
    )?;
    Ok(())
}

/// Handles control group update events
/// These may be initializing or recalled
pub fn update_control_group(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    user_id: i64,
    ctrl_group_evt: &GameSControlGroupUpdateEvent,
) -> Result<(), SwarmyError> {
    unmark_previously_selected_units(sc2_rerun, game_loop, user_id)?;
    match ctrl_group_evt.m_control_group_update {
        GameEControlGroupUpdate::ESet => {
            if let Some(ref mut user_state) = sc2_rerun.user_state.get_mut(&user_id) {
                user_state.control_groups[ctrl_group_evt.m_control_group_index as usize] =
                    user_state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
            }
        }
        GameEControlGroupUpdate::ESetAndSteal => {
            if let Some(ref mut user_state) = sc2_rerun.user_state.get_mut(&user_id) {
                // Remove the instances from other groups first
                let current_selected_units =
                    user_state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
                for group_idx in 0..9 {
                    for unit in &current_selected_units {
                        user_state.control_groups[group_idx].retain(|&x| x != *unit);
                    }
                }
                // Set the group index as the value of the current selected units.
                user_state.control_groups[ctrl_group_evt.m_control_group_index as usize] =
                    user_state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
            }
        }
        GameEControlGroupUpdate::EClear => {
            if let Some(ref mut user_state) = sc2_rerun.user_state.get_mut(&user_id) {
                user_state.control_groups[ctrl_group_evt.m_control_group_index as usize] = vec![];
            }
        }
        GameEControlGroupUpdate::EAppend => {
            if let Some(ref mut user_state) = sc2_rerun.user_state.get_mut(&user_id) {
                let mut current_selected_units =
                    user_state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
                user_state.control_groups[ctrl_group_evt.m_control_group_index as usize]
                    .append(&mut current_selected_units);
                user_state.control_groups[ctrl_group_evt.m_control_group_index as usize]
                    .sort_unstable();
                user_state.control_groups[ctrl_group_evt.m_control_group_index as usize].dedup();
            }
        }
        GameEControlGroupUpdate::EAppendAndSteal => {
            if let Some(ref mut user_state) = sc2_rerun.user_state.get_mut(&user_id) {
                // Remove the instances from other groups first
                let mut current_selected_units =
                    user_state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
                for group_idx in 0..9 {
                    for unit in &current_selected_units {
                        user_state.control_groups[group_idx].retain(|&x| x != *unit);
                    }
                }
                user_state.control_groups[ctrl_group_evt.m_control_group_index as usize]
                    .append(&mut current_selected_units);
                user_state.control_groups[ctrl_group_evt.m_control_group_index as usize]
                    .sort_unstable();
                user_state.control_groups[ctrl_group_evt.m_control_group_index as usize].dedup();
            }
        }
        GameEControlGroupUpdate::ERecall => {
            let mut current_selected_units: Vec<u32> = vec![];
            if let Some(ref mut user_state) = sc2_rerun.user_state.get_mut(&user_id) {
                user_state.control_groups[ACTIVE_UNITS_GROUP_IDX] = user_state.control_groups
                    [ctrl_group_evt.m_control_group_index as usize]
                    .clone();
                current_selected_units = user_state.control_groups[ACTIVE_UNITS_GROUP_IDX].clone();
            }
            mark_selected_units(sc2_rerun, game_loop, user_id, &current_selected_units)?;
        }
    }
    Ok(())
}

/// Registers the game events to Rerun.
pub fn add_game_event(
    mut sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    user_id: i64,
    evt: &ReplayGameEvent,
) -> Result<(), SwarmyError> {
    match &evt {
        ReplayGameEvent::CameraUpdate(camera_update) => {
            register_camera_update(&sc2_rerun, game_loop, user_id, camera_update)?;
        }
        ReplayGameEvent::Cmd(game_cmd) => {
            register_cmd(&mut sc2_rerun, game_loop, user_id, game_cmd)?;
        }
        ReplayGameEvent::CmdUpdateTargetPoint(target_point) => {
            register_update_target_point(&mut sc2_rerun, game_loop, user_id, target_point)?;
        }
        ReplayGameEvent::CmdUpdateTargetUnit(target_unit) => {
            register_update_target_unit(&mut sc2_rerun, game_loop, user_id, &target_unit.m_target)?;
        }
        ReplayGameEvent::SelectionDelta(selection_delta) => {
            register_selection_delta(&mut sc2_rerun, game_loop, user_id, &selection_delta)?;
        }
        ReplayGameEvent::ControlGroupUpdate(ctrl_group) => {
            update_control_group(&mut sc2_rerun, game_loop, user_id, &ctrl_group)?;
        }
        _ => {}
    }
    Ok(())
}
