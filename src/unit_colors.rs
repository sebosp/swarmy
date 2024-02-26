use super::*;
// Returns the expected size of units depending on their type
pub fn get_unit_sized_color(unit_name: &str, user_id: i64) -> (f32, [u8; 4]) {
    let mut unit_size = 0.45;
    let color = match unit_name {
        "VespeneEDyser" => FREYA_LIGHT_GREEN,
        "SpacePlatformGeyser" => FREYA_LIGHT_GREEN,
        "LabMineralField" => {
            unit_size = 0.24;
            FREYA_LIGHT_BLUE
        }
        "LabMineralField750" => {
            unit_size = 0.36;
            FREYA_LIGHT_BLUE
        }
        "MineralField" => {
            unit_size = 0.48;
            FREYA_LIGHT_BLUE
        }
        "MineralField450" => {
            unit_size = 0.6;
            FREYA_LIGHT_BLUE
        }
        "MineralField750" => {
            unit_size = 0.72;
            FREYA_LIGHT_BLUE
        }
        "XelNagaTower" => {
            // This should be super transparent
            unit_size = 0.72;
            FREYA_WHITE
        }
        "RichMineralField" => FREYA_GOLD,
        "RichMineralField750" => FREYA_ORANGE,
        "DestructibleDebris6x6" => {
            unit_size = 1.8;
            FREYA_GRAY
        }
        "UnbuildablePlatesDestructible" => {
            unit_size = 0.6;
            FREYA_LIGHT_GRAY
        }
        "Overlord" => {
            unit_size = 0.6;
            FREYA_YELLOW
        }
        "SCV" | "Drone" | "Probe" | "Larva" => {
            unit_size = 0.3;
            FREYA_LIGHT_GRAY
        }
        "Hatchery" | "CommandCenter" | "Nexus" => {
            unit_size = 1.2;
            FREYA_PINK
        }
        "Broodling" => {
            unit_size = 0.06;
            FREYA_LIGHT_GRAY
        }
        _ => {
            // Ignore the Beacons for now.
            if !unit_name.starts_with("Beacon") {
                tracing::warn!("Unknown unit name: '{}'", unit_name);
            }
            // Fallback to user color
            user_color(user_id)
        }
    };
    (unit_size, color)
}

pub fn user_color(user_id: i64) -> [u8; 4] {
    match user_id {
        0 => FREYA_LIGHT_GREEN,
        1 => FREYA_LIGHT_BLUE,
        2 => FREYA_LIGHT_GRAY,
        3 => FREYA_ORANGE,
        _ => FREYA_WHITE,
    }
}
