//! Tracker Event registration.
use super::*;
use convert_case::{Case, Casing};
use rerun::{
    components::{Point3D, Radius, Scalar, TextEntry},
    MsgSender,
};
use s2protocol::tracker_events::*;

pub fn register_unit_init(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    unit_init: &UnitInitEvent,
) -> Result<usize, SwarmyError> {
    let unit_name_filter = &sc2_rerun.filters.unit_name;
    if let Some(unit_name_filter) = unit_name_filter {
        if unit_name_filter != &unit_init.unit_type_name {
            return Ok(0usize);
        }
    }
    let sc2_unit = SC2Unit {
        last_game_loop: game_loop,
        user_id: Some(unit_init.control_player_id),
        name: unit_init.unit_type_name.clone(),
        pos: Vec3D::new(unit_init.x as f32 / 6f32, unit_init.y as f32 / 6f32, 0.),
        init_game_loop: game_loop,
        ..Default::default()
    };
    if unit_init.unit_tag_index == 159 && unit_init.unit_tag_recycle == 1 {
        tracing::error!("INIT: {:?}", unit_init);
    }
    tracing::info!("Initializing unit: {:?}", sc2_unit);
    let game_unit_tag =
        s2protocol::tracker_events::unit_tag(unit_init.unit_tag_index, unit_init.unit_tag_recycle);
    if let Some(unit) = sc2_rerun.units.get(&game_unit_tag) {
        // Hmm no idea if this is normal.
        tracing::warn!("Re-initializing unit: {:?}", unit);
    }
    sc2_rerun.units.insert(game_unit_tag, sc2_unit);
    MsgSender::new(format!("Unit/{}/Init", game_unit_tag))
        .with_time(sc2_rerun.timeline, game_loop)
        .with_splat(Point3D::new(
            unit_init.x as f32 / 6f32,
            -1. * unit_init.y as f32 / 6f32,
            0.,
        ))?
        .with_splat(FREYA_PINK)?
        .with_splat(TextEntry::new(&unit_init.unit_type_name, None))?
        .with_splat(Radius(0.125))?
        .send(&sc2_rerun.rerun_session)?;
    Ok(1usize)
}

pub fn register_unit_born(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    unit_born: &UnitBornEvent,
) -> Result<(), SwarmyError> {
    if unit_born.unit_tag_index == 159 && unit_born.unit_tag_recycle == 1 {
        tracing::error!("BORN: {:?}", unit_born);
    }
    let unit_name_filter = &sc2_rerun.filters.unit_name;
    if let Some(unit_name_filter) = unit_name_filter {
        if unit_name_filter != &unit_born.unit_type_name {
            return Ok(());
        }
    }
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
    let game_unit_tag =
        s2protocol::tracker_events::unit_tag(unit_born.unit_tag_index, unit_born.unit_tag_recycle);
    if let Some(ref mut sc2_unit) = sc2_rerun.units.get_mut(&game_unit_tag) {
        sc2_unit.creator_ability_name = unit_born.creator_ability_name.clone();
        sc2_unit.last_game_loop = game_loop;
    } else {
        let sc2_unit = SC2Unit {
            last_game_loop: game_loop,
            user_id: Some(unit_born.control_player_id),
            name: unit_born.unit_type_name.clone(),
            pos: Vec3D::new(unit_born.x as f32 / 6f32, unit_born.y as f32 / 6f32, 0.),
            init_game_loop: game_loop,
            ..Default::default()
        };
        sc2_rerun.units.insert(game_unit_tag, sc2_unit);
    }
    MsgSender::new(format!("Unit/{}/Born", game_unit_tag,))
        .with_time(sc2_rerun.timeline, game_loop)
        .with_splat(Point3D::new(
            unit_born.x as f32 / 6f32,
            -1. * unit_born.y as f32 / 6f32,
            0.,
        ))?
        .with_splat(unit_color)?
        .with_splat(TextEntry::new(&unit_born.unit_type_name, None))?
        .with_splat(Radius(unit_size))?
        .send(&sc2_rerun.rerun_session)?;
    Ok(())
}

pub fn register_unit_died(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    unit_dead: &UnitDiedEvent,
) -> Result<(), SwarmyError> {
    if unit_dead.unit_tag_index == 159 && unit_dead.unit_tag_recycle == 1 {
        tracing::error!("DEAD: {:?}", unit_dead);
    }
    let game_unit_tag =
        s2protocol::tracker_events::unit_tag(unit_dead.unit_tag_index, unit_dead.unit_tag_recycle);
    MsgSender::new(format!("Unit/{}/Died", game_unit_tag))
        .with_time(sc2_rerun.timeline, game_loop)
        .with_splat(Point3D::new(
            unit_dead.x as f32 / 6f32,
            -1. * unit_dead.y as f32 / 6f32,
            0.,
        ))?
        .with_splat(FREYA_DARK_RED)?
        .with_splat(Radius(0.125))?
        .send(&sc2_rerun.rerun_session)?;
    if let None = sc2_rerun.units.remove(&game_unit_tag) {
        // This may happen when a larva is transformed to a unit in zerg. so this is normal.
        tracing::debug!(
            "Unit {} reported dead but was not registered before.",
            game_unit_tag
        );
    }
    // Create a Path for Death so that it can be drawn on its separate pane.
    // TODO: Create a "triangle soup", maybe something with low resolution to show regions of high
    // activity.
    MsgSender::new(format!("Death/{}/{}", game_unit_tag, game_loop))
        .with_time(sc2_rerun.timeline, game_loop)
        .with_splat(Point3D::new(
            unit_dead.x as f32 / 6f32,
            -1. * unit_dead.y as f32 / 6f32,
            game_loop as f32 / 100.,
        ))?
        .with_splat(FREYA_DARK_RED)?
        .with_splat(Radius(0.125))?
        .send(&sc2_rerun.rerun_session)?;
    Ok(())
}

pub fn register_unit_position(
    sc2_rerun: &mut SC2Rerun,
    game_loop: i64,
    unit_pos: UnitPositionsEvent,
) -> Result<(), SwarmyError> {
    tracing::error!("POSITION: {:?}", unit_pos.first_unit_index);
    let unit_positions = unit_pos.to_unit_positions_vec();
    for unit_pos_item in unit_positions {
        MsgSender::new(format!("Unit/{}/Position", unit_pos_item.tag))
            .with_time(sc2_rerun.timeline, game_loop)
            .with_splat(Point3D::new(
                unit_pos_item.x as f32 / 24.,
                -1. * unit_pos_item.y as f32 / 24.,
                0.,
            ))?
            .send(&sc2_rerun.rerun_session)?;
        if let Some(ref mut sc2_unit) = sc2_rerun.units.get_mut(&(unit_pos_item.tag as i64)) {
            sc2_unit.pos = Vec3D::new(
                unit_pos_item.x as f32 / 24.,
                -1. * unit_pos_item.y as f32 / 24.,
                0.,
            );
            sc2_unit.last_game_loop = game_loop;
        } else {
            tracing::error!(
                "Unit {} did not exist but position registered.",
                unit_pos_item.tag
            );
        }
    }
    Ok(())
}

pub fn register_player_stats(
    sc2_rerun: &SC2Rerun,
    game_loop: i64,
    player_stats: &PlayerStatsEvent,
) -> Result<(), SwarmyError> {
    if !sc2_rerun.include_stats {
        return Ok(());
    }
    for stat_entity_value in player_stats.stats.as_prop_name_value_vec() {
        println!("Stat: {}", stat_entity_value.0);
        let entity_path = stat_entity_value.0.replace("/", "_").to_case(Case::Pascal);
        MsgSender::new(format!("{}/{}", entity_path, player_stats.player_id,))
            .with_time(sc2_rerun.timeline, game_loop)
            .with_splat(Scalar::from(stat_entity_value.1 as f64))?
            .with_splat(user_color(player_stats.player_id as i64))?
            .send(&sc2_rerun.rerun_session)?;
    }
    Ok(())
}

/// Registers the tracker events to Rerun.
pub fn add_tracker_event(
    mut sc2_rerun: &mut SC2Rerun,
    tracker_loop: i64,
    evt: &ReplayTrackerEvent,
) -> Result<(), SwarmyError> {
    match &evt {
        ReplayTrackerEvent::UnitInit(unit_init) => {
            register_unit_init(&mut sc2_rerun, tracker_loop, unit_init)?;
        }
        ReplayTrackerEvent::UnitBorn(unit_born) => {
            register_unit_born(&mut sc2_rerun, tracker_loop, unit_born)?;
        }
        ReplayTrackerEvent::UnitDied(unit_died) => {
            register_unit_died(&mut sc2_rerun, tracker_loop, unit_died)?;
        }
        ReplayTrackerEvent::UnitPosition(unit_pos) => {
            register_unit_position(&mut sc2_rerun, tracker_loop, unit_pos.clone())?;
        }
        ReplayTrackerEvent::PlayerStats(player_stats) => {
            register_player_stats(sc2_rerun, tracker_loop, player_stats)?;
        }
        _ => {}
    }
    Ok(())
}