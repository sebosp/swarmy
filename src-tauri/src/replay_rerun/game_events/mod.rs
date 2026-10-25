//! Game Events drawing

use super::*;
use s2protocol::game_events::*;
use s2protocol::state::SC2UnitCmdData;
use s2protocol::UnitChangeHint;

impl ReplayRerunLogger {
    fn register_camera_update(
        &self,
        user_id: i64,
        player_name: &str,
        camera_update: &CameraUpdateEvent,
    ) -> Result<(), SwarmyError> {
        if let Some(target) = &camera_update.m_target {
            let (x, y) = (target.x as f32 / 250f32, target.y as f32 / 250f32);
            self.recording_stream.log(
                entity_path!("cameras", player_name),
                &rerun::Boxes3D::from_centers_and_half_sizes(
                    [(x, y, self.ground_z(x, y) + OVERLAY_HEIGHT)],
                    [(5.0, 5.0, 0.025)],
                )
                .with_radii([0.025])
                .with_fill_mode(rerun::FillMode::MajorWireframe)
                .with_labels([player_name])
                .with_colors([self
                    .players
                    .color(player_name)
                    .unwrap_or(user_color(user_id))]),
            )?;
        }
        Ok(())
    }

    /// Adds an arrow from each unit to its command target, they are batched per game loop.
    fn register_command_targets(
        &mut self,
        user_id: i64,
        player_name: &str,
        change_hint: &UnitChangeHint,
    ) {
        let units = match change_hint {
            UnitChangeHint::TargetPoints(units) => units,
            UnitChangeHint::TargetUnits { units, .. } => units,
            _ => return,
        };
        let mut arrows = vec![];
        for unit in units {
            let target = match &unit.cmd.data {
                SC2UnitCmdData::TargetPoint(target_point) => target_point,
                SC2UnitCmdData::TargetUnit(target_unit) => &target_unit.snapshot_point,
                _ => continue,
            };
            // s2protocol negates the y of game event coordinates, unit positions come from
            // tracker events which are not negated.
            let (target_x, target_y) = (target.x(), -target.y());
            let (origin_x, origin_y) = (unit.pos.x(), unit.pos.y());
            let origin_z = self.ground_z(origin_x, origin_y) + OVERLAY_HEIGHT;
            arrows.push((
                [origin_x, origin_y, origin_z],
                [
                    target_x - origin_x,
                    target_y - origin_y,
                    self.ground_z(target_x, target_y) + OVERLAY_HEIGHT - origin_z,
                ],
            ));
        }
        let color = self
            .players
            .color(player_name)
            .unwrap_or(user_color(user_id));
        self.commands
            .entry(player_name.to_string())
            .or_insert_with(|| PendingOverlays {
                color,
                items: vec![],
            })
            .items
            .extend(arrows);
    }

    /// Logs the abilities used, grouped by unit name to keep the log short.
    fn register_abilities(
        &self,
        player_name: &str,
        change_hint: &UnitChangeHint,
    ) -> Result<(), SwarmyError> {
        if let UnitChangeHint::Abilities {
            units,
            event,
            target,
        } = change_hint
        {
            // Commands without units or a known ability would log a lonely ":".
            let Some(abil) = event
                .m_abil
                .as_ref()
                .filter(|abil| !abil.ability.is_empty())
            else {
                return Ok(());
            };
            if units.is_empty() {
                return Ok(());
            }
            let mut unit_counts: BTreeMap<&str, usize> = BTreeMap::new();
            for unit in units {
                *unit_counts.entry(unit.name.as_str()).or_default() += 1;
            }
            let unit_str = unit_counts
                .iter()
                .map(|(name, count)| format!("{count}x{name}"))
                .collect::<Vec<_>>()
                .join(",");
            let target_str = match target {
                Some(target) => format!(" -> {}", target.name),
                None => String::new(),
            };
            self.log_event(
                player_name,
                format!("{unit_str}: {}{target_str}", abil.ability),
                rerun::TextLogLevel::DEBUG,
            )?;
        }
        Ok(())
    }

    /// Registers the game events to Rerun.
    pub fn add_game_event(
        &mut self,
        user_id: i64,
        player_name: &str,
        evt: &ReplayGameEvent,
        change_hint: &UnitChangeHint,
    ) -> Result<(), SwarmyError> {
        self.mark_dirty(change_hint);
        match evt {
            ReplayGameEvent::CameraUpdate(camera_update) => {
                self.register_camera_update(user_id, player_name, camera_update)?;
            }
            ReplayGameEvent::Cmd(_)
            | ReplayGameEvent::CmdUpdateTargetPoint(_)
            | ReplayGameEvent::CmdUpdateTargetUnit(_) => {
                self.register_command_targets(user_id, player_name, change_hint);
                self.register_abilities(player_name, change_hint)?;
            }
            ReplayGameEvent::TriggerChatMessage(chat_message) => {
                self.log_event(
                    player_name,
                    format!("Chat: {}", chat_message.m_chat_message),
                    rerun::TextLogLevel::INFO,
                )?;
            }
            _ => {}
        }
        Ok(())
    }
}
