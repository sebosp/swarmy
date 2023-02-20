//! Tracker Event visualization
//! NOTE:
//! - tag "recycle" is unused for now. Seems to be important for GameEvents, unsupported for now.

use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use colored::*;
use nom_mpq::parser;
use s2protocol::tracker_events::ReplayTrackerEvent;
use s2protocol::tracker_events::TrackerEvent;
use s2protocol::tracker_events::UnitBornEvent;
use s2protocol::versions::read_tracker_events;
use std::convert::From;
use std::time::Duration;

pub const GAME_SCALE: f32 = 100.;

/// A unit that can spawn other units, the units
#[derive(Reflect, Component, Default, Clone)]
#[reflect(Component)]
pub struct UnitCreator {
    /// The unit tag index.
    idx: u32,
    //recycle: u32, // Unused for now.
}

/// A unit state, TODO: add burrowed
#[derive(Reflect, Component, Default, PartialEq, Clone)]
#[reflect(Component)]
pub enum UnitState {
    /// The unit is alive
    #[default]
    Alive,
    /// The unit has died.
    Dead,
}

/// The Players from the replay.
#[derive(Reflect, Component, Clone)]
#[reflect(Component)]
pub struct Player {
    id: u8,
    name: String,
    color: bevy::prelude::Color,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            id: 99,
            name: String::from("Unknown"),
            color: bevy::prelude::Color::WHITE,
        }
    }
}

/// A unit with state and ownership.
#[derive(Reflect, Component, Clone)]
#[reflect(Component)]
struct Unit {
    /// The unit tag index
    idx: u32,
    //recycle: u32, // Unused for now.
    /// The name of the unit
    name: String,
    /// The current unit position in x axis
    x: f32,
    /// The current unit position in y axis
    y: f32,
    /// The player in control of the unit.
    /// This can change ownership but the event is not handled yet.
    player: Player,
    /// The unit creator, for example a hatchery can spawn larva.
    /// TODO: If no creator of unit, we have a custom "System" unit with ID 9999
    creator: UnitCreator,
    /// The las ttime the unit was updated.
    last_game_loop: u32,
    /// The unit may have an activity counter where its size/color/etc may change to visualize
    /// there are changes on it.
    activity_counter: u32,
    /// The color of the Unit
    size: f32,
    /// The unit state
    state: UnitState,
    /// Last updated second
    last_updated: u32,
}

impl Unit {
    pub fn colored_term(&self, time: &Time) {
        let player = match self.player.id {
            1 => "1".blue(),
            2 => "2".yellow(),
            3 => "3".red(),
            4 => "3".purple(),
            _ => "_".white(),
        };
        tracing::error!(
            "{}[{:>3}] idx: {:>4}, pos: ({:>4},{:>4}), creat_idx:{} Name [{:>32}]",
            player,
            time.elapsed_seconds().floor(),
            self.idx.to_string().green(),
            self.x,
            self.y,
            self.creator.idx.to_string().green(),
            self.name,
        );
    }
}

impl From<UnitBornEvent> for Unit {
    fn from(evt: UnitBornEvent) -> Self {
        Unit {
            idx: evt.unit_tag_index,
            name: evt.unit_type_name,
            x: evt.x as f32 / GAME_SCALE,
            y: evt.y as f32 / GAME_SCALE,
            player: Player {
                id: evt.control_player_id,
                ..Default::default()
            },
            creator: UnitCreator {
                // A unit that has no creator would default to this ID.
                // Probably a bad idea. sounds like an soon-to-be footgun.
                idx: evt.creator_unit_tag_index.unwrap_or(9999),
            },
            last_game_loop: 0u32,
            activity_counter: 0u32,
            size: 1f32,
            state: UnitState::Alive,
            last_updated: 0u32,
        }
    }
}

impl Default for Unit {
    fn default() -> Self {
        Self {
            idx: 0,
            name: String::from("Unknown"),
            ..default()
        }
    }
}

#[derive(Resource, Debug)]
struct ReplayTimer {
    last_updated: i32,
    current: Timer,
}

#[derive(Component)]
struct Upkeep {
    current: usize,
    max: usize,
}

#[derive(Resource, Component)]
pub struct GameLoopEvent {
    processed: bool,
    game_loop: u32,
    evt: TrackerEvent,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct GameAssets {
    ike_scene: Handle<Scene>,
    cartman_scene: Handle<Scene>,
    kyle_scene: Handle<Scene>,
    kenny_scene: Handle<Scene>,
    tweek_scene: Handle<Scene>,
    sp_church_scene: Handle<Scene>,
    wendy_scene: Handle<Scene>,
}

fn asset_loading(mut commands: Commands, assets: ResMut<AssetServer>) {
    let ike_asset: Handle<Scene> = assets.load("south_park_canada_ike.glb#Scene0");
    let cartman_asset: Handle<Scene> = assets.load("cartman.glb#Scene0");
    let kyle_asset: Handle<Scene> = assets.load("south_park_kyle_broflovski.glb#Scene0");
    let kenny_asset: Handle<Scene> = assets.load("kenny.glb#Scene0");
    let tweek_asset: Handle<Scene> =
        assets.load("nintendo_64_-_south_park_rally_-_tweek.glb#Scene0");
    let sp_church_asset: Handle<Scene> = assets.load("sp_church.glb#Scene0");
    let wendy_asset: Handle<Scene> = assets.load("wendy_testaburger.glb#Scene0");
    commands.insert_resource(GameAssets {
        ike_scene: ike_asset,
        cartman_scene: cartman_asset,
        kyle_scene: kyle_asset,
        kenny_scene: kenny_asset,
        tweek_scene: tweek_asset,
        sp_church_scene: sp_church_asset,
        wendy_scene: wendy_asset,
    });
}

/// It seems the gamespeed is 22.4 loops per second.
pub const GAME_LOOP_SPEED: f32 = 112.0f32;

pub struct SC2ReplayPlugin;

impl Plugin for SC2ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReplayTimer {
            last_updated: 0i32,
            current: Timer::new(Duration::from_millis(1_000), TimerMode::Repeating),
        })
        .add_startup_system(asset_loading)
        .add_startup_system(add_tracker_events)
        .add_system(unit_born)
        .add_system(unit_dead)
        .add_system(unit_move);
    }
}

pub fn get_unit_sized_color(unit_name: &str) -> (f32, bevy::prelude::Color) {
    let mut unit_size = 0.75;
    let color = match unit_name {
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
            // tracing::warn!("Unknown unit name: '{}'", unit_name);
            bevy::prelude::Color::WHITE
        }
    };
    (unit_size, color)
}

fn unit_born(
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
    for (entity, mut tracker_event) in game_loop_query.iter_mut() {
        let game_loop_time = (tracker_event.game_loop as f32 / GAME_LOOP_SPEED).floor() as i32;
        if (!tracker_event.processed && timer.last_updated > game_loop_time)
            || timer.last_updated - 1 == game_loop_time
        {
            let event = match &tracker_event.evt.event {
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
            tracker_event.processed = true;
        }
    }
    timer.last_updated = time.elapsed_seconds() as i32;
}

fn unit_dead(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: ResMut<ReplayTimer>,
    mut game_loop_query: Query<(Entity, &mut GameLoopEvent, &mut Unit)>,
) {
    if !timer.current.tick(time.delta()).just_finished() {
        return;
    }
    for (entity, mut tracker_event, mut unit) in game_loop_query.iter_mut() {
        let game_loop_time = (tracker_event.game_loop as f32 / GAME_LOOP_SPEED).floor() as i32;
        if (!tracker_event.processed && timer.last_updated > game_loop_time)
            || timer.last_updated - 1 == game_loop_time
        {
            match &tracker_event.evt.event {
                ReplayTrackerEvent::UnitDied(event) => {
                    unit.state = UnitState::Dead;
                    commands.entity(entity).despawn_recursive();
                    tracing::info!("UnitDied: {:?}", event);
                }
                _ => continue,
            }
            tracker_event.processed = true;
        }
    }
    timer.last_updated = time.elapsed_seconds() as i32;
}

fn unit_move(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: ResMut<ReplayTimer>,
    mut game_loop_query: Query<(Entity, &mut GameLoopEvent)>,
    mut unit_query: Query<(&mut Unit, &mut Transform)>,
) {
    if !timer.current.tick(time.delta()).just_finished() {
        return;
    }
    let mut new_units_positions: HashMap<u32, (f32, f32)> = HashMap::new();
    for (entity, mut tracker_event) in game_loop_query.iter_mut() {
        let game_loop_time = (tracker_event.game_loop as f32 / GAME_LOOP_SPEED).floor() as i32;
        if (!tracker_event.processed && timer.last_updated > game_loop_time)
            || timer.last_updated - 1 == game_loop_time
        {
            let event = match &tracker_event.evt.event {
                ReplayTrackerEvent::UnitPosition(event) => event.clone(),
                _ => continue,
            };
            let unit_positions = event.to_unit_positions_vec();
            for unit_pos in unit_positions {
                new_units_positions.insert(
                    unit_pos.tag,
                    (
                        unit_pos.x as f32 / GAME_SCALE,
                        unit_pos.y as f32 / GAME_SCALE,
                    ),
                );
            }
            commands.entity(entity).despawn_recursive();
            tracker_event.processed = true;
        }
    }
    for (mut unit, mut transform) in &mut unit_query.iter_mut() {
        if let Some((x, y)) = new_units_positions.remove(&unit.idx) {
            unit.colored_term(&time);
            transform.translation = Vec3::new(x, y, 1.);
            //transform.scale = Vec3::new(10., 10., 10.);
            unit.x = x;
            unit.y = y;
        }
    }
    timer.last_updated = time.elapsed_seconds() as i32;
}

fn add_tracker_events(
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
        parser::read_file("/home/seb/git/nom-mpq/assets/SC2-Patch_4.12-2v2AI.SC2Replay");
    let (_input, mpq) = parser::parse(&file_contents).unwrap();
    let tracker_events = read_tracker_events(&mpq, &file_contents);
    let mut game_loop = 0u32;
    let mut max_items = 5000usize;
    for evt in tracker_events {
        game_loop += evt.delta;
        commands
            .spawn(GameLoopEvent {
                game_loop,
                evt,
                processed: false,
            })
            .insert(bevy::core::Name::new(format!("{}", game_loop)));
        max_items -= 1;
        if max_items < 2 {
            break;
        }
    }
}

fn main() {
    App::new()
        // Window Setup
        .insert_resource(ClearColor(bevy::prelude::Color::rgb(0.2, 0.2, 0.2)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Swarmy".to_string(),
                resizable: false,
                ..Default::default()
            },
            ..default()
        }))
        .add_plugin(WorldInspectorPlugin)
        // Inspector Setup
        .register_type::<Unit>()
        .add_plugin(SC2ReplayPlugin)
        .run();
}
