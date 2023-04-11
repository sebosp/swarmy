//! Handling of Unit Death to bevy despawning.
use crate::*;
use s2protocol::tracker_events::ReplayTrackerEvent;

pub fn unit_dead(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: ResMut<ReplayTimer>,
    mut game_loop_query: Query<(Entity, &mut GameLoopEvent, &mut Unit)>,
) {
    if !timer.current.tick(time.delta()).just_finished() {
        return;
    }
    for (entity, mut replay_event, mut unit) in game_loop_query.iter_mut() {
        let game_loop_time = (replay_event.game_loop as f32 / GAME_LOOP_SPEED).floor() as i64;
        if (!replay_event.processed && timer.last_updated > game_loop_time)
            || timer.last_updated - 1 == game_loop_time
        {
            let tracker = match &replay_event.evt {
                GameEventType::Tracker(evt) => evt,
                _ => continue,
            };
            match &tracker.event {
                ReplayTrackerEvent::UnitDied(event) => {
                    unit.state = UnitState::Dead;
                    commands.entity(entity).despawn_recursive();
                    tracing::info!("UnitDied: {:?}", event);
                }
                _ => continue,
            }
            replay_event.processed = true;
        }
    }
    timer.last_updated = time.elapsed_seconds() as i64;
}
