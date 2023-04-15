//! Game Events drawing

use super::*;
use nom_mpq::MPQ;
use rerun::Session;
use rerun::{
    components::{Box3D, Point3D, Quaternion, Radius, Rigid3, Transform, Vec3D},
    time::Timeline,
    MsgSender,
};
use s2protocol::game_events::*;
use s2protocol::versions::read_game_events;

pub fn register_camera_update(
    camera_update: &CameraUpdateEvent,
    rerun_session: &Session,
    game_timeline: &Timeline,
    game_loop: i64,
    user_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(target) = &camera_update.m_target {
        MsgSender::new(format!("Unit/999{}/Player", user_id))
            .with_time(*game_timeline, game_loop)
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
            .send(rerun_session)?;
    }
    Ok(())
}

pub fn register_cmd(
    game_cmd: &GameSCmdEvent,
    rerun_session: &Session,
    game_timeline: &Timeline,
    game_loop: i64,
    user_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    match &game_cmd.m_data {
        GameSCmdData::TargetPoint(target) => {
            MsgSender::new(format!("Target/{}/Camera", user_id))
                .with_time(*game_timeline, game_loop)
                .with_splat(Point3D::new(
                    target.x as f32 / GAME_EVENT_POS_RATIO,
                    -1. * target.y as f32 / GAME_EVENT_POS_RATIO,
                    target.z as f32 / GAME_EVENT_POS_RATIO,
                ))?
                .with_splat(user_color(user_id))?
                .with_splat(Radius(0.5))?
                .send(rerun_session)?;
        }
        GameSCmdData::TargetUnit(target) => {
            MsgSender::new(format!(
                "Target/{}/Unit/{}",
                target.m_snapshot_control_player_id.unwrap_or_default(),
                target.m_tag,
            ))
            .with_time(*game_timeline, game_loop)
            .with_splat(Point3D::new(
                target.m_snapshot_point.x as f32 / GAME_EVENT_POS_RATIO,
                -1. * target.m_snapshot_point.y as f32 / GAME_EVENT_POS_RATIO,
                target.m_snapshot_point.z as f32 / GAME_EVENT_POS_RATIO,
            ))?
            .with_splat(FREYA_RED)?
            .with_splat(Radius(0.1))?
            .send(rerun_session)?;
        }
        GameSCmdData::Data(data) => {
            tracing::info!("GameSCmdData: {}", data);
        }
        GameSCmdData::None => {}
    }
    Ok(())
}

pub fn register_update_target_point(
    target_point: &GameSCmdUpdateTargetPointEvent,
    rerun_session: &Session,
    game_timeline: &Timeline,
    game_loop: i64,
    user_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    MsgSender::new(format!("Target/{}", user_id))
        .with_time(*game_timeline, game_loop)
        .with_splat(Point3D::new(
            target_point.m_target.x as f32 / GAME_EVENT_POS_RATIO,
            -1. * target_point.m_target.y as f32 / GAME_EVENT_POS_RATIO,
            target_point.m_target.z as f32 / GAME_EVENT_POS_RATIO,
        ))?
        .with_splat(user_color(user_id))?
        .with_splat(Radius(0.5))?
        .send(rerun_session)?;
    Ok(())
}

pub fn register_update_target_unit(
    target_unit: &GameSCmdUpdateTargetUnitEvent,
    rerun_session: &Session,
    game_timeline: &Timeline,
    game_loop: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    MsgSender::new(format!("Unit/{}/UpdateTarget", target_unit.m_target.m_tag))
        .with_time(*game_timeline, game_loop)
        .with_splat(Point3D::new(
            target_unit.m_target.m_snapshot_point.x as f32 / GAME_EVENT_POS_RATIO,
            -1. * target_unit.m_target.m_snapshot_point.y as f32 / GAME_EVENT_POS_RATIO,
            target_unit.m_target.m_snapshot_point.z as f32 / GAME_EVENT_POS_RATIO,
        ))?
        .with_splat(FREYA_WHITE)?
        .with_splat(Radius(0.08))?
        .send(rerun_session)?;
    Ok(())
}
/// Registers the game events to Rerun.
pub fn add_game_events(
    mpq: &MPQ,
    file_contents: &[u8],
    rerun_session: &Session,
    game_timeline: &Timeline,
) -> Result<(), Box<dyn std::error::Error>> {
    let game_events = read_game_events(&mpq, &file_contents);
    let mut game_loop = 0i64;
    for game_step in game_events {
        game_loop += game_step.delta as i64;
        match game_step.event {
            ReplayGameEvent::CameraUpdate(ref camera_update) => register_camera_update(
                camera_update,
                rerun_session,
                game_timeline,
                game_loop,
                game_step.user_id,
            )?,
            ReplayGameEvent::Cmd(ref game_cmd) => register_cmd(
                game_cmd,
                rerun_session,
                game_timeline,
                game_loop,
                game_step.user_id,
            )?,
            ReplayGameEvent::CmdUpdateTargetPoint(ref target_point) => {
                register_update_target_point(
                    target_point,
                    rerun_session,
                    game_timeline,
                    game_loop,
                    game_step.user_id,
                )?
            }
            ReplayGameEvent::CmdUpdateTargetUnit(ref target_unit) => {
                register_update_target_unit(target_unit, rerun_session, game_timeline, game_loop)?
            }
            _ => {}
        }
    }
    println!("Final Game loop: {}", game_loop);
    Ok(())
}
