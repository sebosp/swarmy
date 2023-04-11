//! Handling of unit birth to Bevy Command spawns

use super::*;
use s2protocol::tracker_events::ReplayTrackerEvent;

pub fn unit_born(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: ResMut<ReplayTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    game_assets: Res<GameAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut game_loop_query: Query<(Entity, &mut GameLoopEvent)>,
) {
    if !timer.current.tick(time.delta()).just_finished() {
        return;
    }
    for (entity, mut replay_event) in game_loop_query.iter_mut() {
        let game_loop_time = (replay_event.game_loop as f32 / GAME_LOOP_SPEED).floor() as i64;
        if (!replay_event.processed && timer.last_updated > game_loop_time)
            || timer.last_updated - 1 == game_loop_time
        {
            let tracker = match &replay_event.evt {
                GameEventType::Tracker(evt) => evt,
                _ => continue,
            };
            let event = match &tracker.event {
                ReplayTrackerEvent::UnitBorn(event) => event.clone(),
                _ => continue,
            };
            tracing::info!("UnitBorn: {:?}", event);
            let unit_type_name = &event.unit_type_name;
            let unit_name_with_creator_ability = match &event.creator_ability_name {
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
            let (unit_size, unit_color) = get_unit_sized_color(&unit_name_with_creator_ability);
            let unit_material = StandardMaterial {
                emissive: unit_color.into(),
                ..default()
            };
            let evt_x = event.x as f32 / GAME_SCALE;
            let evt_y = event.y as f32 / GAME_SCALE;
            let mut unit = Unit::from(event);
            let unit_name = bevy::core::Name::new(unit.name.clone());
            unit.size = unit_size;
            if unit.name == "Drone" || unit.name == "Probe" || unit.name == "SCV" {
                commands
                    .spawn(SceneBundle {
                        scene: game_assets.ike_scene.clone(),
                        transform: Transform::from_translation(Vec3::new(evt_x, evt_y, 0.))
                            .with_scale(Vec3::new(0.0005, 0.0005, 0.0005)),
                        ..default()
                    })
                    .insert(unit)
                    .insert(unit_name);
            } else if unit.name == "Overlord" {
                commands
                    .spawn(SceneBundle {
                        scene: game_assets.wendy_scene.clone(),
                        transform: Transform::from_translation(Vec3::new(evt_x, evt_y, 0.))
                            .with_scale(Vec3::new(0.02, 0.02, 0.02)),
                        ..default()
                    })
                    .insert(unit)
                    .insert(unit_name);
            } else if unit.name == "SiegeTank" {
                commands
                    .spawn(SceneBundle {
                        scene: game_assets.cartman_scene.clone(),
                        transform: Transform::from_translation(Vec3::new(evt_x, evt_y, 0.))
                            .with_scale(Vec3::new(0.0005, 0.0005, 0.0005)),
                        ..default()
                    })
                    .insert(unit)
                    .insert(unit_name);
            } else if unit.name == "Marine" {
                commands
                    .spawn(SceneBundle {
                        scene: game_assets.kyle_scene.clone(),
                        transform: Transform::from_translation(Vec3::new(evt_x, evt_y, 0.))
                            .with_scale(Vec3::new(0.0005, 0.0005, 0.0005)),
                        ..default()
                    })
                    .insert(unit)
                    .insert(unit_name);
            } else if unit.name == "Zergling" {
                commands
                    .spawn(SceneBundle {
                        scene: game_assets.tweek_scene.clone(),
                        transform: Transform::from_translation(Vec3::new(evt_x, evt_y, 0.))
                            .with_scale(Vec3::new(0.1, 0.1, 0.1)),
                        ..default()
                    })
                    .insert(unit)
                    .insert(unit_name);
            } else if unit.name == "Hatchery"
                || unit.name == "Nexus"
                || unit.name == "CommandCenter"
            {
                commands
                    .spawn(SceneBundle {
                        scene: game_assets.sp_church_scene.clone(),
                        transform: Transform::from_translation(Vec3::new(evt_x, evt_y, 0.))
                            .with_scale(Vec3::new(0.005, 0.005, 0.005)),
                        ..default()
                    })
                    .insert(unit)
                    .insert(unit_name);
            } else if unit.name == "Baneling" {
                commands
                    .spawn(SceneBundle {
                        scene: game_assets.kenny_scene.clone(),
                        transform: Transform::from_translation(Vec3::new(evt_x, evt_y, 0.))
                            .with_scale(Vec3::new(0.0005, 0.0005, 0.0005)),
                        ..default()
                    })
                    .insert(unit)
                    .insert(unit_name);
            } else {
                commands
                    .spawn(PbrBundle {
                        mesh: meshes
                            .add(shape::Circle::new(unit_size / 100.).into())
                            .into(),
                        material: materials.add(unit_material),
                        transform: Transform::from_translation(Vec3::new(evt_x, evt_y, 0.)),
                        ..default()
                    })
                    .insert(unit)
                    .insert(unit_name);
            }
            /*.insert(bevy::core::Name::new(format!(
                "{}:{}",
                event.unit_tag_index, unit_name_with_creator_ability
            )));*/
            commands.entity(entity).despawn_recursive();
            replay_event.processed = true;
        }
    }
    timer.last_updated = time.elapsed_seconds() as i64;
}
