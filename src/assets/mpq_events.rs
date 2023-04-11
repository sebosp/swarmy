//! Reading of events an spawing an bevy components.

use crate::*;
use nom_mpq::parser;
use s2protocol::versions::read_game_events;
use s2protocol::versions::read_tracker_events;

/// Loads the Tracker Events from an file MPQ. This is hardcoded for now.
pub fn add_events(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(2., 2.)))),
            material: materials.add(StandardMaterial {
                base_color: bevy::prelude::Color::rgba(0., 0., 0., 0.8),
                metallic: 1.,
                ..default()
            }),
            transform: Transform::from_translation(Vec3::new(1.25, 1., -0.4)),
            ..default()
        })
        .insert(bevy::core::Name::new("Plane"));
    // light
    commands.insert_resource(AmbientLight {
        color: bevy::prelude::Color::WHITE,
        brightness: 1.0,
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(1., 1., 2.5).looking_at(Vec3::new(1., 1., 0.), Vec3::Y),
        ..default()
    });
    let file_contents =
        parser::read_file("/home/seb/git/nom-mpq/assets/2023-04-08-2v2AI.SC2Replay");
    let (_input, mpq) = parser::parse(&file_contents).unwrap();
    let tracker_events = read_tracker_events(&mpq, &file_contents);
    let mut game_loop = 0u64;
    let mut max_items = 5000usize;
    for evt in tracker_events {
        game_loop += evt.delta as u64;
        commands
            .spawn(GameLoopEvent {
                game_loop,
                evt: GameEventType::Tracker(evt),
                processed: false,
            })
            .insert(bevy::core::Name::new(format!("Trck::{}", game_loop)));
        max_items -= 1;
        if max_items < 2 {
            break;
        }
    }
    let game_events = read_game_events(&mpq, &file_contents);
    let mut game_loop = 0u64;
    let mut max_items = 5000usize;
    for evt in game_events {
        game_loop += evt.delta as u64;
        if let ReplayGameEvent::CameraUpdate(ref evt) = evt.event {
            tracing::info!("Loading Game Event; {:?}", evt);
        }
        commands
            .spawn(GameLoopEvent {
                game_loop,
                evt: GameEventType::Game(evt),
                processed: false,
            })
            .insert(bevy::core::Name::new(format!("Game::{}", game_loop)));
        max_items -= 1;
        if max_items < 2 {
            break;
        }
    }
}
