//! Unit Move handling

use crate::*;
use bevy::utils::HashMap;
use s2protocol::tracker_events::ReplayTrackerEvent;

pub fn unit_move(
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
                ReplayTrackerEvent::UnitPosition(event) => event.clone(),
                _ => continue,
            };
            let unit_positions = event.to_unit_positions_vec();
            for unit_pos in unit_positions {
                new_units_positions.insert(
                    unit_pos.tag,
                    (
                        unit_pos.x as f32 / GAME_SCALE / 4.,
                        unit_pos.y as f32 / GAME_SCALE / 4.,
                    ),
                );
            }
            commands.entity(entity).despawn_recursive();
            replay_event.processed = true;
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
    timer.last_updated = time.elapsed_seconds() as i64;
}
