//! Game Events drawing

use super::*;
use s2protocol::game_events::*;
use s2protocol::state::SC2UnitCmdData;
use s2protocol::UnitChangeHint;

pub fn register_camera_update(
    user_id: i64,
    player_name: &str,
    camera_update: &CameraUpdateEvent,
    recording_stream: &RecordingStream,
    _game_loop: i64,
) -> Result<(), SwarmyError> {
    if let Some(target) = &camera_update.m_target {
        recording_stream.log(
            format!("Player/{}:{}/Cam", player_name, user_id),
            &rerun::Boxes3D::from_centers_and_half_sizes(
                [(target.x as f32 / 250f32, 1. * target.y as f32 / 250f32, 0.)],
                [(5.0, 5.0, 0.025)],
            )
            .with_radii([0.025])
            .with_labels([player_name])
            .with_colors([user_color(user_id)]),
        )?;
    }
    Ok(())
}

pub fn register_camera_save(
    user_id: i64,
    player_name: &str,
    camera_save: &CameraSaveEvent,
    recording_stream: &RecordingStream,
    _game_loop: i64,
) -> Result<(), SwarmyError> {
    recording_stream.log(
        format!(
            "CamSave/{}:{}/{}",
            player_name, user_id, camera_save.m_which
        ),
        &rerun::TextLog::new(format!(
            "{}:{:?}",
            camera_save.m_which, camera_save.m_target
        ))
        .with_level(rerun::TextLogLevel::TRACE),
    )?;
    recording_stream.log(
        format!(
            "Player/{}:{}/CamSave/{}",
            user_id, player_name, camera_save.m_which
        ),
        &rerun::Boxes3D::from_centers_and_half_sizes(
            [(
                camera_save.m_target.x as f32 / 250f32,
                1. * camera_save.m_target.y as f32 / 250f32,
                0.,
            )],
            [(5.0, 5.0, 0.025)],
        )
        .with_radii([0.025])
        //.with_labels([format!("{}", camera_save.m_which)])
        .with_colors([user_color(user_id)]),
    )?;
    Ok(())
}

/// Draw an arrow from the unit to the target point.
pub fn register_update_target_point(
    user_id: i64,
    player_name: &str,
    change_hint: UnitChangeHint,
    _target_point: &GameSMapCoord3D,
    recording_stream: &RecordingStream,
    _game_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::TargetPoints(updated_units) = change_hint {
        for selected_unit in updated_units {
            if let SC2UnitCmdData::TargetPoint(target_point) = &selected_unit.cmd.data {
                let unit_target_pos = rerun::Vec3D::new(
                    target_point.x() - selected_unit.pos.x(),
                    -(target_point.y() + selected_unit.pos.y()),
                    0.,
                );
                let selected_unit_pos =
                    rerun::Vec3D::new(selected_unit.pos.x(), selected_unit.pos.y(), 0.);
                recording_stream.log(
                    format!(
                        "Log/{}:{}/{}/{}/TP/{}",
                        player_name,
                        user_id,
                        selected_unit.name,
                        selected_unit.tag_index,
                        selected_unit
                            .cmd
                            .abil
                            .map(|a| a.ability)
                            .unwrap_or("".to_string()),
                    ),
                    &rerun::TextLog::new(format!(
                        "TP{:?}:{:?}",
                        selected_unit_pos, unit_target_pos
                    ))
                    .with_level(rerun::TextLogLevel::TRACE),
                )?;
                /*recording_stream.log(
                    format!("Unit/{}/{}/TP", selected_unit.name, selected_unit.tag_index),
                    &rerun::Arrows3D::from_vectors([unit_target_pos])
                        .with_origins([selected_unit_pos])
                        .with_colors([user_color(user_id)]),
                )?;*/
            }
        }
    }
    Ok(())
}

pub fn register_update_target_unit(
    user_id: i64,
    player_name: &str,
    change_hint: UnitChangeHint,
    _target_unit: &GameSCmdDataTargetUnit,
    recording_stream: &RecordingStream,
    _game_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::TargetUnits {
        units: user_selected_units,
        target: target_unit,
    } = change_hint
    {
        for selected_unit in user_selected_units {
            if let SC2UnitCmdData::TargetUnit(target_unit_data) = &selected_unit.cmd.data {
                let unit_target_pos = rerun::Vec3D::new(
                    target_unit_data.snapshot_point.x() - selected_unit.pos.x(),
                    -(target_unit_data.snapshot_point.y() + selected_unit.pos.y()),
                    0.,
                );
                let selected_unit_pos =
                    rerun::Vec3D::new(selected_unit.pos.x(), selected_unit.pos.y(), 0.);
                recording_stream.log(
                    format!(
                        "Log/{}:{}/{}/{}/TU",
                        player_name, user_id, selected_unit.name, selected_unit.tag_index
                    ),
                    &rerun::TextLog::new(format!(
                        "{}({:?})->{}({:?})",
                        selected_unit.name, selected_unit_pos, target_unit.name, unit_target_pos
                    ))
                    .with_level(rerun::TextLogLevel::TRACE),
                )?;
                /*recording_stream.log(
                    format!("Unit/{}/{}/TU", selected_unit.name, selected_unit.tag_index),
                    &rerun::Arrows3D::from_vectors([unit_target_pos])
                        .with_origins([selected_unit_pos])
                        .with_colors([FREYA_RED]),
                )?;*/
            }
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
    _game_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Selection(changed_units) = change_hint {
        for unit in changed_units {
            // XXX: Technically this is not "Born", we should have a State or Status that
            // contains the radius of the unit.
            recording_stream.log(
                format!("Unit/{}/{}/Born", unit.name, unit.tag_index),
                &rerun::Points3D::new([(unit.pos.x(), unit.pos.y(), 0.)])
                    //.with_draw_order(game_loop as f32)
                    .with_radii([unit.radius]),
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
    _game_loop: i64,
) -> Result<(), SwarmyError> {
    if let UnitChangeHint::Selection(changed_units) = change_hint {
        if ctrl_group_evt.m_control_group_update == GameEControlGroupUpdate::ERecall {
            for unit in changed_units {
                // XXX: Technically this is not "Born", we should have a State or Status that
                // contains the radius of the unit.
                recording_stream.log(
                    format!("Unit/{}/{}/Born", unit.name, unit.tag_index),
                    &rerun::Points3D::new([(unit.pos.x(), unit.pos.y(), 0.)])
                        //.with_draw_order(game_loop as f32)
                        .with_radii([unit.radius]),
                )?;
            }
        }
    }
    Ok(())
}

pub fn register_cmd(
    user_id: i64,
    player_name: &str,
    change_hint: UnitChangeHint,
    game_cmd: &GameSCmdEvent,
    recording_stream: &RecordingStream,
    game_loop: i64,
) -> Result<(), SwarmyError> {
    match &game_cmd.m_data {
        GameSCmdData::TargetPoint(target) => {
            register_update_target_point(
                user_id,
                player_name,
                change_hint.clone(),
                target,
                recording_stream,
                game_loop,
            )?;
        }
        GameSCmdData::TargetUnit(target_unit) => {
            register_update_target_unit(
                user_id,
                player_name,
                change_hint.clone(),
                target_unit,
                recording_stream,
                game_loop,
            )?;
        }
        GameSCmdData::Data(data) => {
            tracing::info!("GameSCmdData: {}", data);
        }
        GameSCmdData::None => {}
    }
    if let UnitChangeHint::Abilities {
        units,
        event,
        target,
    } = change_hint
    {
        for unit in units {
            let target_str = if let Some(tgt) = &target {
                format!(";Tgt:{}", tgt.name)
            } else {
                "".to_string()
            };

            let abil_str = if let Some(abil) = &event.m_abil {
                format!(
                    "{}: {} l:{:?};i:{:?};d:{:?}{}",
                    unit.name,
                    abil.ability,
                    abil.m_abil_link,
                    abil.m_abil_cmd_index,
                    abil.m_abil_cmd_data,
                    target_str
                )
            } else {
                "".to_string()
            };
            recording_stream.log(
                format!(
                    "Tgt/{}:{}/{}/{}",
                    player_name, user_id, unit.name, unit.tag_index
                ),
                &rerun::TextLog::new(abil_str).with_level(rerun::TextLogLevel::INFO),
            )?;
        }
    }
    Ok(())
}

pub fn handle_chat_message(
    user_id: i64,
    player_name: &str,
    _change_hint: UnitChangeHint,
    chat_message: &GameSTriggerChatMessageEvent,
    recording_stream: &RecordingStream,
    _game_loop: i64,
) -> Result<(), SwarmyError> {
    recording_stream.log(
        format!("Chat/{}:{}", player_name, user_id),
        &rerun::TextLog::new(chat_message.m_chat_message.to_string())
            .with_level(rerun::TextLogLevel::TRACE),
    )?;
    Ok(())
}

/// Registers the game events to Rerun.
pub fn add_game_event(
    user_id: i64,
    player_name: &str,
    evt: &ReplayGameEvent,
    change_hint: UnitChangeHint,
    recording_stream: &RecordingStream,
    game_loop: i64,
) -> Result<(), SwarmyError> {
    match &evt {
        ReplayGameEvent::CameraSave(camera_save) => {
            register_camera_save(
                user_id,
                player_name,
                camera_save,
                recording_stream,
                game_loop,
            )?;
        }
        ReplayGameEvent::CameraUpdate(camera_update) => {
            register_camera_update(
                user_id,
                player_name,
                camera_update,
                recording_stream,
                game_loop,
            )?;
        }
        ReplayGameEvent::Cmd(game_cmd) => {
            register_cmd(
                user_id,
                player_name,
                change_hint,
                game_cmd,
                recording_stream,
                game_loop,
            )?;
        }
        ReplayGameEvent::CmdUpdateTargetPoint(target_point) => {
            register_update_target_point(
                user_id,
                player_name,
                change_hint,
                &target_point.m_target,
                recording_stream,
                game_loop,
            )?;
        }
        ReplayGameEvent::CmdUpdateTargetUnit(target_unit) => {
            register_update_target_unit(
                user_id,
                player_name,
                change_hint,
                &target_unit.m_target,
                recording_stream,
                game_loop,
            )?;
        }
        ReplayGameEvent::ControlGroupUpdate(ctrl_group) => {
            update_control_group(change_hint, ctrl_group, recording_stream, game_loop)?;
        }
        ReplayGameEvent::SelectionDelta(_selection_delta) => {
            register_selection_delta(change_hint, recording_stream, game_loop)?;
        }
        ReplayGameEvent::TriggerChatMessage(chat_message) => {
            handle_chat_message(
                user_id,
                player_name,
                change_hint,
                chat_message,
                recording_stream,
                game_loop,
            )?;
        }
        ReplayGameEvent::DropUser(_) => {}
        ReplayGameEvent::SelectionSyncCheck(_) => {}
        ReplayGameEvent::UnitClick(_) => {}
        ReplayGameEvent::UnitHighlight(_) => {}
        ReplayGameEvent::TriggerReplySelected(_) => {}
        ReplayGameEvent::TriggerMouseClicked(_) => {}
        ReplayGameEvent::TriggerMouseMoved(_) => {}
        ReplayGameEvent::TriggerHotkeyPressed(_) => {}
        ReplayGameEvent::TriggerTargetModeUpdate(_) => {}
        ReplayGameEvent::TriggerKeyPressed(_) => {}
        ReplayGameEvent::TriggerMouseWheel(_) => {}
        ReplayGameEvent::TriggerButtonPressed(_) => {}
        ReplayGameEvent::CommandManagerState(_) => {}
    }
    Ok(())
}
