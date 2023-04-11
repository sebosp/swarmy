//! Tracker Event visualization

use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use swarmy::*;

fn main() {
    App::new()
        // Window Setup
        .insert_resource(ClearColor(bevy::prelude::Color::rgb(0.2, 0.2, 0.2)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(1920., 1080.),
                title: "Swarmy".to_string(),
                resizable: false,
                ..Default::default()
            }),
            ..default()
        }))
        .add_plugin(WorldInspectorPlugin::new())
        // Inspector Setup
        .register_type::<Unit>()
        .add_plugin(SC2ReplayPlugin)
        .run();
}
