//! Game Events drawing

use std::collections::HashMap;

use super::*;
use rerun::components::Arrow3D;
use rerun::{
    components::{Box3D, Point3D, Quaternion, Radius, Rigid3, Transform, Vec3D},
    MsgSender,
};
use s2protocol::game_events::*;
use s2protocol::versions::read_game_events;

pub fn register_camera_update(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    camera_update: &CameraUpdateEvent,
) -> Result<usize, SwarmyError> {
    if let Some(target) = &camera_update.m_target {
        MsgSender::new(format!("Unit/999{}/Player", user_id))
            .with_time(sc2_rerun.timeline, game_loop)
            .with_splat(Box3D::new(0.8, 0.8, 0.0))?
            .with_splat(Transform::Rigid3(Rigid3 {
                rotation: Quaternion::new(0., 0., 0., 0.),
                translation: Vec3D::new(
                    (target.x as f32 / 1500f32) - 0.3,
                    (-1. * target.y as f32 / 1500f32) - 0.3,
                    0.,
                ),
            }))?
            .with_splat(user_color(user_id))?
            .send(&sc2_rerun.rerun_session)?;
        Ok(1usize)
    } else {
        Ok(0usize)
    }
}

pub fn register_cmd(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    game_cmd: &GameSCmdEvent,
) -> Result<usize, SwarmyError> {
    match &game_cmd.m_data {
        GameSCmdData::TargetPoint(target) => {
            MsgSender::new(format!("Target/{}/Camera", user_id))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Point3D::new(
                    target.x as f32 / GAME_EVENT_POS_RATIO,
                    -1. * target.y as f32 / GAME_EVENT_POS_RATIO,
                    target.z as f32 / GAME_EVENT_POS_RATIO,
                ))?
                .with_splat(user_color(user_id))?
                .with_splat(Radius(0.5))?
                .send(&sc2_rerun.rerun_session)?;
            Ok(1usize)
        }
        GameSCmdData::TargetUnit(target) => {
            MsgSender::new(format!(
                "Target/{}/Unit/{}",
                target.m_snapshot_control_player_id.unwrap_or_default(),
                target.m_tag,
            ))
            .with_time(sc2_rerun.timeline, game_loop)
            .with_splat(Point3D::new(
                target.m_snapshot_point.x as f32 / GAME_EVENT_POS_RATIO,
                -1. * target.m_snapshot_point.y as f32 / GAME_EVENT_POS_RATIO,
                target.m_snapshot_point.z as f32 / GAME_EVENT_POS_RATIO,
            ))?
            .with_splat(FREYA_RED)?
            .with_splat(Radius(0.1))?
            .send(&sc2_rerun.rerun_session)?;
            MsgSender::new(format!("Unit/{}/Target", target.m_tag,))
                .with_time(sc2_rerun.timeline, game_loop)
                .with_splat(Arrow3D {
                    origin: Vec3D::new(
                        0., 0., 0., // XXX
                    ),
                    vector: Vec3D::new(
                        target.m_snapshot_point.x as f32 / GAME_EVENT_POS_RATIO,
                        -1. * target.m_snapshot_point.y as f32 / GAME_EVENT_POS_RATIO,
                        target.m_snapshot_point.z as f32 / GAME_EVENT_POS_RATIO,
                    ),
                })?
                .with_splat(FREYA_LIGHT_GRAY)?
                .with_splat(Radius(0.1))?
                .send(&sc2_rerun.rerun_session)?;
            Ok(2usize)
        }
        GameSCmdData::Data(data) => {
            tracing::info!("GameSCmdData: {}", data);
            Ok(0usize)
        }
        GameSCmdData::None => Ok(0usize),
    }
}

pub fn register_update_target_point(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    target_point: &GameSCmdUpdateTargetPointEvent,
) -> Result<usize, SwarmyError> {
    MsgSender::new(format!("Target/{}", user_id))
        .with_time(sc2_rerun.timeline, game_loop)
        .with_splat(Point3D::new(
            target_point.m_target.x as f32 / GAME_EVENT_POS_RATIO,
            -1. * target_point.m_target.y as f32 / GAME_EVENT_POS_RATIO,
            target_point.m_target.z as f32 / GAME_EVENT_POS_RATIO,
        ))?
        .with_splat(user_color(user_id))?
        .with_splat(Radius(0.5))?
        .send(&sc2_rerun.rerun_session)?;
    Ok(1usize)
}

pub fn register_update_target_unit(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    target_unit: &GameSCmdUpdateTargetUnitEvent,
) -> Result<usize, SwarmyError> {
    MsgSender::new(format!("Unit/{}/UpdateTarget", target_unit.m_target.m_tag))
        .with_time(sc2_rerun.timeline, game_loop)
        .with_splat(Point3D::new(
            target_unit.m_target.m_snapshot_point.x as f32 / GAME_EVENT_POS_RATIO,
            -1. * target_unit.m_target.m_snapshot_point.y as f32 / GAME_EVENT_POS_RATIO,
            target_unit.m_target.m_snapshot_point.z as f32 / GAME_EVENT_POS_RATIO,
        ))?
        .with_splat(FREYA_WHITE)?
        .with_splat(Radius(0.08))?
        .send(&sc2_rerun.rerun_session)?;
    Ok(1usize)
}
/// Registers the game events to Rerun.
pub fn add_game_events(sc2_rerun: &SC2Rerun) -> Result<usize, SwarmyError> {
    let game_events = read_game_events(&sc2_rerun.mpq, &sc2_rerun.file_contents);
    let mut game_loop = 0i64;
    let mut total_events = 0usize;
    let min_filter = sc2_rerun.filters.min_loop.clone();
    let max_filter = sc2_rerun.filters.max_loop.clone();
    let user_id_filter = sc2_rerun.filters.user_id.clone();
    for game_step in game_events {
        game_loop += game_step.delta as i64;
        if let Some(min) = min_filter {
            // Skip the events less than the requested filter.
            if min < game_loop {
                continue;
            }
        }
        if let Some(max) = max_filter {
            // Skip the events greater than the requested filter.
            if max > game_loop {
                continue;
            }
        }
        if let Some(user_id) = user_id_filter {
            // Skip the events greater than the requested filter.
            if game_step.user_id != user_id {
                continue;
            }
        }
        match game_step.event {
            ReplayGameEvent::CameraUpdate(ref camera_update) => {
                total_events +=
                    register_camera_update(sc2_rerun, game_loop, game_step.user_id, camera_update)?
            }
            ReplayGameEvent::Cmd(ref game_cmd) => {
                total_events += register_cmd(sc2_rerun, game_loop, game_step.user_id, game_cmd)?
            }
            ReplayGameEvent::CmdUpdateTargetPoint(ref target_point) => {
                total_events += register_update_target_point(
                    sc2_rerun,
                    game_loop,
                    game_step.user_id,
                    target_point,
                )?
            }
            ReplayGameEvent::CmdUpdateTargetUnit(ref target_unit) => {
                total_events += register_update_target_unit(
                    sc2_rerun,
                    game_loop,
                    game_step.user_id,
                    target_unit,
                )?
            }
            _ => {}
        }
    }
    tracing::info!(
        "Added a total of {} GameEvents. Final Game loop: {}",
        total_events,
        game_loop
    );
    Ok(total_events)
}
