//! Game Events drawing

use super::*;
use rerun::components::Arrow3D;
use rerun::{
    components::{Box3D, Point3D, Quaternion, Radius, Rigid3, Transform, Vec3D},
    MsgSender,
};
use s2protocol::game_events::*;

pub fn register_camera_update(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    user_id: i64,
    camera_update: &CameraUpdateEvent,
) -> Result<(), SwarmyError> {
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
            register_update_target_unit(sc2_rerun, game_loop, target_unit)?;
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
    target_point: &GameSCmdUpdateTargetPointEvent,
) -> Result<(), SwarmyError> {
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
    Ok(())
}

pub fn register_update_target_unit(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    target_unit: &GameSCmdDataTargetUnit,
) -> Result<(), SwarmyError> {
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
        println!("m_tag not found for unit: {:?}", target_unit.m_tag);
        for (key, val) in sc2_rerun.units.iter() {
            if !val.name.contains("Geyser")
                && !val.name.contains("Mineral")
                && !val.name.contains("XelNagaTower")
            {
                println!("{}: {:?}", key, val);
            }
        }
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
            register_update_target_unit(&mut sc2_rerun, game_loop, &target_unit.m_target)?;
        }
        _ => {}
    }
    Ok(())
}
