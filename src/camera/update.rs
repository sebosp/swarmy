//! Player Camera Update

use super::*;

pub fn camera_update(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: ResMut<ReplayTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
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
            let camera = match PlayerCamera::try_from_camera_update(&replay_event.evt) {
                Ok(val) => val,
                Err(_err) => continue,
            };
            let camera_material = player_camera_material(camera.player.id);
            tracing::info!("CameraUpdate: {:?}", camera);
            let camera_name = bevy::core::Name::new(format!(
                "Game:CameraUpdate:{}:{}",
                replay_event.game_loop, camera.player.id
            ));
            commands
                .spawn(PbrBundle {
                    mesh: meshes
                        .add(shape::RegularPolygon::new(1., 4usize).into())
                        .into(),
                    material: materials.add(camera_material),
                    transform: Transform::from_translation(Vec3::new(camera.x, camera.y, 0.))
                        .with_scale(Vec3::new(0.02, 0.02, 0.02)),
                    ..default()
                })
                .insert(camera)
                .insert(camera_name);
            commands.entity(entity).despawn_recursive(); // huh^?
            replay_event.processed = true;
        }
    }
    timer.last_updated = time.elapsed_seconds() as i64;
}
