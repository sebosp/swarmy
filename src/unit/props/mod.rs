//! Unit Properties

// Returns the expected size of units depending on their type
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
