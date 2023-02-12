use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use colored::*;
use nom_mpq::parser;
use nom_mpq::*;
use s2protocol::tracker_events::read_tracker_events;
use s2protocol::versions::protocol87702::{ReplayTrackerEEventId, ReplayTrackerSUnitBornEvent};
use std::str;

#[derive(Component)]
struct SC2TrackerEvent;

#[derive(Component)]
struct SC2UnitBorn {
    game_loop: u32,
    unit_name: String,
    evt: ReplayTrackerSUnitBornEvent,
}

#[derive(Resource, Debug)]
struct SC2ReplayTimer {
    last_updated: u32,
    current: Timer,
}

#[derive(Component)]
struct Player {
    name: String,
    id: u32,
}

#[derive(Component)]
struct Upkeep {
    current: usize,
    max: usize,
}

// This resource holds information about the game:
#[derive(Resource, Default)]
struct GameState {
    current_loop: u32,
    timer: Timer,
    total_players: usize,
    player_units: Vec<SC2UnitBorn>,
}

pub const GAME_LOOP_SPEED: f32 = 2240.0f32;

fn game_loop(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: ResMut<SC2ReplayTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&SC2UnitBorn, With<SC2TrackerEvent>>,
) {
    if timer.current.tick(time.delta()).just_finished() {
        for tracker_event in query.iter() {
            if timer.last_updated - 1
                == (tracker_event.game_loop as f32 / GAME_LOOP_SPEED).floor() as u32
            {
                commands.spawn(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                    material: materials.add(bevy::prelude::Color::rgb(0.1, 0.9, 0.1).into()),
                    transform: Transform::from_xyz(
                        tracker_event.evt.m_x as f32,
                        tracker_event.evt.m_y as f32,
                        tracker_event.game_loop as f32 / GAME_LOOP_SPEED / 10.,
                    ),
                    ..default()
                });
                let player = match tracker_event.evt.m_control_player_id {
                    1 => "1".blue(),
                    2 => "2".yellow(),
                    _ => "_".white(),
                };
                let m_creat_tag_idx = match tracker_event.evt.m_creator_unit_tag_index {
                    Some(val) => format!("{:>4}", val),
                    None => String::from("    "),
                };
                let m_creat_tag_rcycl = match tracker_event.evt.m_creator_unit_tag_recycle {
                    Some(val) => format!("{}", val),
                    None => String::from("    "),
                };
                println!(
                    "{}[{:>3}={:>6}] Upkeep: {:>4} unit(idx: {:>4}, rcycl: {:}), pos: ({:>4},{:>4}), mcreat: (unit(idx:{}, rcycl: {})) Born [{:>32}]",
                    player,
                    time.elapsed_seconds().floor(),
                    tracker_event.game_loop,
                    tracker_event.evt.m_upkeep_player_id,
                    tracker_event.evt.m_unit_tag_index.to_string().green(),
                    tracker_event.evt.m_unit_tag_recycle,
                    tracker_event.evt.m_x,
                    tracker_event.evt.m_y,
                    m_creat_tag_idx.green(),
                    m_creat_tag_rcycl,
                    tracker_event.unit_name,
                );
            }
        }
    }
    timer.last_updated = time.elapsed_seconds() as u32;
}

pub struct SC2ReplayPlugin;

impl Plugin for SC2ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SC2ReplayTimer {
            last_updated: 0u32,
            current: Timer::from_seconds(1.0, TimerMode::Repeating),
        })
        .add_startup_system(add_tracker_events)
        .add_system(game_loop);
    }
}

// Try to get a printable representation of the _name
fn printable(input: &[u8]) -> String {
    match str::from_utf8(&input) {
        Ok(val) => val.to_string(),
        Err(_) => parser::peek_hex(&input),
    }
}

fn add_tracker_events(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(200., 200.)))),
            material: materials.add(StandardMaterial {
                base_color: bevy::prelude::Color::rgb(0.88, 0.89, 0.72),
                metallic: 0.,
                ..default()
            }),
            transform: Transform::from_translation(Vec3::new(125., 100., -5.)),
            ..default()
        })
        .insert(bevy::core::Name::new("Plane"));
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(100.0, 100.0, 20.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(100., 100., 250.0)
            .looking_at(Vec3::new(100., 100., 0.), Vec3::Y),
        ..default()
    });
    let file_contents =
        parser::read_file("/home/seb/git/nom-mpq/assets/SC2-Patch_4.12-2v2AI.SC2Replay");
    let (_input, mpq) = parser::parse(&file_contents).unwrap();
    let tracker_events = read_tracker_events(&mpq, &file_contents);
    let mut max_items = 500usize;
    for read_evt in tracker_events {
        if let ReplayTrackerEEventId::EUnitPosition(unit_position) = read_evt.event {
            tracing::info!("Position: {:?}", unit_position);
        } else if let ReplayTrackerEEventId::EUnitBorn(unit_born_event) = read_evt.event {
            let unit_type_name = printable(&unit_born_event.m_unit_type_name);
            let unit_name_with_creator_ability = match &unit_born_event.m_creator_ability_name {
                Some(val) => {
                    let mut m_creator = printable(val);
                    // Add some context to what ability created this unit.
                    if !m_creator.is_empty() && m_creator != unit_type_name {
                        m_creator.push_str(">");
                        m_creator.push_str(&unit_type_name);
                    }
                    m_creator
                }
                None => unit_type_name,
            };
            let mut unit_size = 0.75;
            let unit_color = match unit_name_with_creator_ability.as_ref() {
                "VespeneGeyser" => bevy::prelude::Color::LIME_GREEN,
                "SpacePlatformGeyser" => bevy::prelude::Color::GREEN,
                "LabMineralField" => {
                    unit_size = 0.4;
                    bevy::prelude::Color::TEAL
                }
                "LabMineralField750" => {
                    unit_size = 0.6;
                    bevy::prelude::Color::TEAL
                }
                "MineralField" => {
                    unit_size = 0.8;
                    bevy::prelude::Color::TEAL
                }
                "MineralField450" => {
                    unit_size = 1.0;
                    bevy::prelude::Color::TEAL
                }
                "MineralField750" => {
                    unit_size = 1.2;
                    bevy::prelude::Color::TEAL
                }
                "RichMineralField" => bevy::prelude::Color::GOLD,
                "RichMineralField750" => bevy::prelude::Color::ORANGE_RED,
                "DestructibleDebris6x6" => {
                    unit_size = 3.;
                    bevy::prelude::Color::GRAY
                }
                "UnbuildablePlatesDestructible" => {
                    unit_size = 1.0;
                    bevy::prelude::Color::GRAY
                }
                _ => {
                    tracing::error!("Unknown unit name: '{}'", unit_name_with_creator_ability);
                    bevy::prelude::Color::WHITE
                }
            };
            let unit_material = StandardMaterial {
                emissive: unit_color.into(),
                ..default()
            };
            if read_evt.game_loop == 0 {
                // Circle
                commands
                    .spawn(PbrBundle {
                        mesh: meshes.add(shape::Circle::new(unit_size).into()).into(),
                        material: materials.add(unit_material),
                        transform: Transform::from_translation(Vec3::new(
                            unit_born_event.m_x.into(),
                            unit_born_event.m_y.into(),
                            0.,
                        )),
                        ..default()
                    })
                    .insert(bevy::core::Name::new(
                        unit_name_with_creator_ability.clone(),
                    ));
            }
            if unit_born_event.m_control_player_id == 1 {
                commands.spawn((
                    SC2TrackerEvent,
                    SC2UnitBorn {
                        game_loop: read_evt.game_loop,
                        evt: unit_born_event,
                        unit_name: unit_name_with_creator_ability,
                    },
                ));
                max_items -= 1;
            }
        }
        if max_items < 1 {
            break;
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(WorldInspectorPlugin)
        .add_plugin(SC2ReplayPlugin)
        .run();
}
