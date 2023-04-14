//! Game Events drawing

use super::*;
use nom_mpq::MPQ;
use rerun::Session;
use rerun::{
    components::{Box3D, Point3D, Quaternion, Radius, Rigid3, Transform, Vec3D},
    time::Timeline,
    MsgSender,
};
use s2protocol::game_events::GameSCmdData;
use s2protocol::game_events::ReplayGameEvent;
use s2protocol::versions::read_game_events;

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
        if let ReplayGameEvent::CameraUpdate(ref camera_update) = game_step.event {
            if let Some(target) = &camera_update.m_target {
                MsgSender::new(format!("Unit/999{}/Player", game_step.user_id))
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
                    .with_splat(user_color(game_step.user_id))?
                    .send(rerun_session)?;
            }
        } else if let ReplayGameEvent::Cmd(ref game_cmd) = game_step.event {
            if let GameSCmdData::TargetPoint(target) = &game_cmd.m_data {
                MsgSender::new(format!("Target/{}/Camera", game_step.user_id))
                    .with_time(*game_timeline, game_loop)
                    .with_splat(Point3D::new(
                        target.x as f32 / GAME_EVENT_POS_RATIO,
                        -1. * target.y as f32 / GAME_EVENT_POS_RATIO,
                        target.z as f32 / GAME_EVENT_POS_RATIO,
                    ))?
                    .with_splat(user_color(game_step.user_id))?
                    .with_splat(Radius(0.5))?
                    .send(rerun_session)?;
            } else if let GameSCmdData::TargetUnit(target) = &game_cmd.m_data {
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
        } else if let ReplayGameEvent::CmdUpdateTargetPoint(ref target_point) = game_step.event {
            MsgSender::new(format!("Target/{}", game_step.user_id))
                .with_time(*game_timeline, game_loop)
                .with_splat(Point3D::new(
                    target_point.m_target.x as f32 / GAME_EVENT_POS_RATIO,
                    -1. * target_point.m_target.y as f32 / GAME_EVENT_POS_RATIO,
                    target_point.m_target.z as f32 / GAME_EVENT_POS_RATIO,
                ))?
                .with_splat(user_color(game_step.user_id))?
                .with_splat(Radius(0.5))?
                .send(rerun_session)?;
        } else if let ReplayGameEvent::CmdUpdateTargetUnit(ref target_unit) = game_step.event {
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
        }
    }
    println!("Final Game loop: {}", game_loop);
    Ok(())
}
