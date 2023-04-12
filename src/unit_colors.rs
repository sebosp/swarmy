use super::*;
// Returns the expected size of units depending on their type
pub fn get_unit_sized_color(unit_name: &str, user_id: i64) -> (f32, ColorRGBA) {
    let mut unit_size = 0.075;
    let color = match unit_name {
        "VespeneEDyser" => FREYA_LIGHT_GREEN,
        "SpacePlatformGeyser" => FREYA_LIGHT_GREEN,
        "LabMineralField" => {
            unit_size = 0.04;
            FREYA_LIGHT_BLUE
        }
        "LabMineralField750" => {
            unit_size = 0.06;
            FREYA_LIGHT_BLUE
        }
        "MineralField" => {
            unit_size = 0.08;
            FREYA_LIGHT_BLUE
        }
        "MineralField450" => {
            unit_size = 0.1;
            FREYA_LIGHT_BLUE
        }
        "MineralField750" => {
            unit_size = 0.12;
            FREYA_LIGHT_BLUE
        }
        "RichMineralField" => FREYA_GOLD,
        "RichMineralField750" => FREYA_ORANGE,
        "DestructibleDebris6x6" => {
            unit_size = 0.3;
            FREYA_GRAY
        }
        "UnbuildablePlatesDestructible" => {
            unit_size = 0.1;
            FREYA_LIGHT_GRAY
        }
        "Overlord" => {
            unit_size = 0.0;
            FREYA_YELLOW
        }
        "SCV" | "Drone" | "Probe" => {
            unit_size = 0.05;
            FREYA_LIGHT_GRAY
        }
        "Hatchery" | "CommandCenter" | "Nexus" => {
            unit_size = 0.2;
            FREYA_PINK
        }
        "Broodling" => {
            unit_size = 0.01;
            FREYA_LIGHT_GRAY
        }
        _ => {
            println!("Unknown unit name: '{}'", unit_name);
            // Fallback to user color
            user_color(user_id)
        }
    };
    (unit_size, color)
}

pub fn user_color(user_id: i64) -> ColorRGBA {
    match user_id {
        0 => FREYA_LIGHT_GREEN,
        1 => FREYA_LIGHT_BLUE,
        2 => FREYA_LIGHT_GRAY,
        _ => FREYA_WHITE,
    }
}
