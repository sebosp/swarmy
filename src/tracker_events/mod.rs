//! Tracker Event registration.
use super::*;
use convert_case::{Case, Casing};
use nom_mpq::MPQ;
use rerun::{
    components::{Point3D, Radius, Scalar, TextEntry},
    time::Timeline,
    MsgSender, Session,
};
use s2protocol::tracker_events::*;
use s2protocol::versions::read_tracker_events;

pub fn register_unit_init(
    unit_init: &UnitInitEvent,
    rerun_session: &Session,
    game_timeline: &Timeline,
    game_loop: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    MsgSender::new(format!(
        "Unit/{}/Init",
        s2protocol::tracker_events::unit_tag(unit_init.unit_tag_index, unit_init.unit_tag_recycle)
    ))
    .with_time(
        *game_timeline,
        (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
    )
    .with_splat(Point3D::new(
        unit_init.x as f32 / 6f32,
        -1. * unit_init.y as f32 / 6f32,
        0.,
    ))?
    .with_splat(FREYA_PINK)?
    .with_splat(TextEntry::new(&unit_init.unit_type_name, None))?
    .with_splat(Radius(0.125))?
    .send(rerun_session)?;
    Ok(())
}

pub fn register_unit_born(
    unit_born: &UnitBornEvent,
    rerun_session: &Session,
    game_timeline: &Timeline,
    game_loop: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let unit_type_name = &unit_born.unit_type_name;
    let unit_name_with_creator_ability = match &unit_born.creator_ability_name {
        Some(val) => {
            let mut creator = val.clone();
            // Add some context to what ability created this unit.
            if !creator.is_empty() && val != unit_type_name {
                creator.push_str(">");
                creator.push_str(&unit_type_name);
            }
            creator
        }
        None => unit_type_name.clone(),
    };
    let (unit_size, unit_color) = get_unit_sized_color(
        &unit_name_with_creator_ability,
        unit_born.control_player_id as i64,
    );
    MsgSender::new(format!(
        "Unit/{}/Born",
        s2protocol::tracker_events::unit_tag(unit_born.unit_tag_index, unit_born.unit_tag_recycle)
    ))
    .with_time(
        *game_timeline,
        (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
    )
    .with_splat(Point3D::new(
        unit_born.x as f32 / 6f32,
        -1. * unit_born.y as f32 / 6f32,
        0.,
    ))?
    .with_splat(unit_color)?
    .with_splat(TextEntry::new(&unit_born.unit_type_name, None))?
    .with_splat(Radius(unit_size))?
    .send(rerun_session)?;
    Ok(())
}

pub fn register_unit_died(
    unit_dead: &UnitDiedEvent,
    rerun_session: &Session,
    game_timeline: &Timeline,
    game_loop: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    MsgSender::new(format!(
        "Unit/{}/Died",
        s2protocol::tracker_events::unit_tag(unit_dead.unit_tag_index, unit_dead.unit_tag_recycle)
    ))
    .with_time(
        *game_timeline,
        (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
    )
    .with_splat(Point3D::new(
        unit_dead.x as f32 / 6f32,
        -1. * unit_dead.y as f32 / 6f32,
        0.,
    ))?
    .with_splat(FREYA_DARK_RED)?
    .with_splat(Radius(0.125))?
    .send(rerun_session)?;
    // Create a Path for Death so that it can be drawn on its separate pane.
    // TODO: Create a "triangle soup", maybe something with low resolution to show regions of high
    // activity.
    MsgSender::new(format!(
        "Death/{}/{}",
        s2protocol::tracker_events::unit_tag(unit_dead.unit_tag_index, unit_dead.unit_tag_recycle),
        game_loop
    ))
    .with_time(
        *game_timeline,
        (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
    )
    .with_splat(Point3D::new(
        unit_dead.x as f32 / 6f32,
        -1. * unit_dead.y as f32 / 6f32,
        game_loop as f32 / 100.,
    ))?
    .with_splat(FREYA_DARK_RED)?
    .with_splat(Radius(0.125))?
    .send(rerun_session)?;
    Ok(())
}

pub fn register_unit_position(
    unit_pos: UnitPositionsEvent,
    rerun_session: &Session,
    game_timeline: &Timeline,
    game_loop: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    for unit_pos_item in unit_pos.to_unit_positions_vec() {
        MsgSender::new(format!("Unit/{}/Position", unit_pos_item.tag,))
            .with_time(
                *game_timeline,
                (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
            )
            .with_splat(Point3D::new(
                unit_pos_item.x as f32 / 24.,
                -1. * unit_pos_item.y as f32 / 24.,
                0.,
            ))?
            .send(rerun_session)?;
    }
    Ok(())
}

pub fn register_player_stats(
    player_stats: &PlayerStatsEvent,
    rerun_session: &Session,
    game_timeline: &Timeline,
    game_loop: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    for stat_entity_value in player_stats.stats.as_prop_name_value_vec() {
        println!("Stat: {}", stat_entity_value.0);
        let entity_path = stat_entity_value.0.replace("/", "_").to_case(Case::Pascal);
        MsgSender::new(format!("{}/{}", entity_path, player_stats.player_id,))
            .with_time(
                *game_timeline,
                (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
            )
            .with_splat(Scalar::from(stat_entity_value.1 as f64))?
            .with_splat(user_color(player_stats.player_id as i64))?
            .send(rerun_session)?;
    }
    Ok(())
}

/// Registers the tracker events to Rerun.
pub fn add_tracker_events(
    mpq: &MPQ,
    file_contents: &[u8],
    rerun_session: &Session,
    game_timeline: &Timeline,
) -> Result<(), Box<dyn std::error::Error>> {
    let tracker_events = read_tracker_events(&mpq, &file_contents);
    let mut game_loop = 0i64;
    for game_step in tracker_events {
        game_loop += game_step.delta as i64;
        match game_step.event {
            ReplayTrackerEvent::UnitInit(ref unit_init) => {
                register_unit_init(unit_init, rerun_session, game_timeline, game_loop)?
            }
            ReplayTrackerEvent::UnitBorn(ref unit_born) => {
                register_unit_born(unit_born, rerun_session, game_timeline, game_loop)?
            }
            ReplayTrackerEvent::UnitDied(ref unit_died) => {
                register_unit_died(unit_died, rerun_session, game_timeline, game_loop)?
            }
            ReplayTrackerEvent::UnitPosition(unit_pos) => {
                register_unit_position(unit_pos, rerun_session, game_timeline, game_loop)?
            }
            //ReplayTrackerEvent::PlayerStats(ref player_stats) => register_player_stats(player_stats, rerun_session game_timeline, game_loop)?,
            _ => {}
        }
    }
    println!("Final Tracker loop: {}", game_loop);
    Ok(())
}
