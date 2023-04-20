//! Game Events drawing

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
    sc2_rerun: &mut SC2Rerun,
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
        GameSCmdData::TargetUnit(target_unit) => {
            /*MsgSender::new(format!(
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
            .send(&sc2_rerun.rerun_session)?;*/
            register_update_target_unit(sc2_rerun, game_loop, user_id, target_unit)
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
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    user_id: i64,
    target_unit: &GameSCmdDataTargetUnit,
) -> Result<usize, SwarmyError> {
    let unit_target_pos = Vec3D::new(
        target_unit.m_snapshot_point.x as f32 / GAME_EVENT_POS_RATIO,
        -1. * target_unit.m_snapshot_point.y as f32 / GAME_EVENT_POS_RATIO,
        target_unit.m_snapshot_point.z as f32 / GAME_EVENT_POS_RATIO,
    );
    let mut unit_pos = Vec3D::new(0., 0., 0.);
    if let Some(ref mut unit) = sc2_rerun.units.get_mut(&(target_unit.m_tag as i64)) {
        unit.target = Some(unit_target_pos);
        unit_pos = unit.pos.clone();
        unit.last_game_loop = game_loop;
    } else {
        tracing::error!(
            "Unit {} Position updated but unit does not exist.",
            target_unit.m_tag
        );
    }
    MsgSender::new(format!("Unit/{}/Position", target_unit.m_tag))
        .with_time(sc2_rerun.timeline, game_loop)
        .with_splat(Arrow3D {
            origin: unit_pos,
            vector: unit_target_pos,
        })?
        .with_splat(FREYA_LIGHT_GREEN)?
        .send(&sc2_rerun.rerun_session)?;
    Ok(1usize)
}
/// Registers the game events to Rerun.
pub fn add_game_events(mut sc2_rerun: &mut SC2Rerun) -> Result<usize, SwarmyError> {
    let game_events = read_game_events(&sc2_rerun.mpq, &sc2_rerun.file_contents);
    let mut game_loop = 0i64;
    let mut total_events = 0usize;
    let min_filter = sc2_rerun.filters.min_loop.clone();
    let max_filter = sc2_rerun.filters.max_loop.clone();
    let user_id_filter = sc2_rerun.filters.user_id.clone();
    let max_events = sc2_rerun.filters.max_events.clone();
    for (idx, game_step) in game_events.iter().enumerate() {
        game_loop += game_step.delta as i64;
        if let Some(min) = min_filter {
            // Skip the events less than the requested filter.
            if game_loop < min {
                continue;
            }
        }
        if let Some(max) = max_filter {
            // Skip the events greater than the requested filter.
            if game_loop > max {
                break;
            }
        }
        if let Some(user_id) = user_id_filter {
            // Skip the events greater than the requested filter.
            if game_step.user_id != user_id {
                continue;
            }
        }
        if let Some(max) = max_events {
            if idx > max {
                break;
            }
        }
        match &game_step.event {
            ReplayGameEvent::CameraUpdate(camera_update) => {
                total_events +=
                    register_camera_update(&sc2_rerun, game_loop, game_step.user_id, camera_update)?
            }
            ReplayGameEvent::Cmd(game_cmd) => {
                total_events +=
                    register_cmd(&mut sc2_rerun, game_loop, game_step.user_id, game_cmd)?
            }
            ReplayGameEvent::CmdUpdateTargetPoint(target_point) => {
                total_events += register_update_target_point(
                    &mut sc2_rerun,
                    game_loop,
                    game_step.user_id,
                    target_point,
                )?
            }
            ReplayGameEvent::CmdUpdateTargetUnit(target_unit) => {
                total_events += register_update_target_unit(
                    &mut sc2_rerun,
                    game_loop,
                    game_step.user_id,
                    &target_unit.m_target,
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
