//! Starcraft 2 - Replay visualizer.
//!

use nom_mpq::{parser, MPQ};
use rerun::Session;
use rerun::{
    components::{ColorRGBA, Point3D, Radius, Scalar, TextEntry},
    time::Timeline,
    MsgSender,
};
use s2protocol::tracker_events::ReplayTrackerEvent;
use s2protocol::versions::read_tracker_events;

pub mod unit_colors;
pub use unit_colors::*;

pub mod game_events;
pub use game_events::*;

/// The game events seem to be at this ratio when compared to Tracker Events.
pub const GAME_EVENT_POS_RATIO: f32 = 27_000f32;

// Some colors I really liked from a Freya Holmer presentation:
// https://www.youtube.com/watch?v=kfM-yu0iQBk
pub const FREYA_ORANGE: ColorRGBA = ColorRGBA(0xeb790700);
pub const FREYA_GOLD: ColorRGBA = ColorRGBA(0xea9e3600);
pub const FREYA_RED: ColorRGBA = ColorRGBA(0xf8105300);
pub const FREYA_BLUE: ColorRGBA = ColorRGBA(0x30b5f700);
pub const FREYA_GREEN: ColorRGBA = ColorRGBA(0x0aeb9f00);
pub const FREYA_LIGHT_BLUE: ColorRGBA = ColorRGBA(0x72c5dd00);
pub const FREYA_GRAY: ColorRGBA = ColorRGBA(0xb2c5c500);
pub const FREYA_PINK: ColorRGBA = ColorRGBA(0xeaa48300);
pub const FREYA_LIGHT_GRAY: ColorRGBA = ColorRGBA(0xf4f5f800);
pub const FREYA_DARK_BLUE: ColorRGBA = ColorRGBA(0x4da7c200);
pub const FREYA_DARK_GREEN: ColorRGBA = ColorRGBA(0x37bda900);
pub const FREYA_DARK_RED: ColorRGBA = ColorRGBA(0xae204400);
pub const FREYA_VIOLET: ColorRGBA = ColorRGBA(0xa401ed00);
pub const FREYA_WHITE: ColorRGBA = ColorRGBA(0xfaf8fb00);
pub const FREYA_YELLOW: ColorRGBA = ColorRGBA(0xf7d45400);
pub const FREYA_LIGHT_YELLOW: ColorRGBA = ColorRGBA(0xead8ad00);
pub const FREYA_LIGHT_GREEN: ColorRGBA = ColorRGBA(0x6ec29c00);

// This was observed in a game with max game_loop = 13735 and a duration of 15:42 = 942 seconds.
// nanos 942000000000 / 13735 game_loops = 68583909 nanoseconds per game_loop
pub const GAME_LOOP_SPEED_NANOS: i64 = 68_583_909;

pub const TRACKER_SPEED_RATIO: f32 = 0.70996;

pub fn read_mpq(path: &str) -> (MPQ, Vec<u8>) {
    tracing::info!("Processing MPQ file {}", path);
    let file_contents = parser::read_file(path);
    let (_, mpq) = parser::parse(&file_contents).unwrap();
    (mpq, file_contents)
}

pub fn add_tracker_events(
    mpq: &MPQ,
    file_contents: &[u8],
    rerun_session: &Session,
    game_timeline: &Timeline,
) -> Result<(), Box<dyn std::error::Error>> {
    let tracker_events = read_tracker_events(&mpq, &file_contents);
    let mut game_loop = 0i64;
    for game_step in tracker_events {
        game_loop += game_step.delta as i64;
        if let ReplayTrackerEvent::UnitInit(ref unit_init) = game_step.event {
            MsgSender::new(format!(
                "Unit/{}/Init",
                s2protocol::tracker_events::unit_tag(
                    unit_init.unit_tag_index,
                    unit_init.unit_tag_recycle
                )
            ))
            .with_time(
                *game_timeline,
                (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
            )
            .with_splat(Point3D::new(
                unit_init.x as f32 / 6f32,
                -1. * unit_init.y as f32 / 6f32,
                0.,
            ))?
            .with_splat(FREYA_PINK)? // Find the user_id related to this m_tag
            .with_splat(TextEntry::new(&unit_init.unit_type_name, None))? // Find the user_id related to this m_tag
            .with_splat(Radius(0.125))?
            .send(rerun_session)?;
        } else if let ReplayTrackerEvent::UnitDied(ref unit_dead) = game_step.event {
            MsgSender::new(format!(
                "Unit/{}/Died",
                s2protocol::tracker_events::unit_tag(
                    unit_dead.unit_tag_index,
                    unit_dead.unit_tag_recycle
                )
            ))
            .with_time(
                *game_timeline,
                (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
            )
            .with_splat(Point3D::new(
                unit_dead.x as f32 / 6f32,
                -1. * unit_dead.y as f32 / 6f32,
                0.,
            ))?
            .with_splat(FREYA_DARK_RED)? // Find the user_id related to this m_tag
            .with_splat(Radius(0.125))?
            .send(rerun_session)?;
            MsgSender::new(format!(
                "Death/{}/{}",
                s2protocol::tracker_events::unit_tag(
                    unit_dead.unit_tag_index,
                    unit_dead.unit_tag_recycle
                ),
                game_loop
            ))
            .with_time(
                *game_timeline,
                (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
            )
            .with_splat(Point3D::new(
                unit_dead.x as f32 / 6f32,
                -1. * unit_dead.y as f32 / 6f32,
                game_loop as f32 / 100.,
            ))?
            .with_splat(FREYA_DARK_RED)? // Find the user_id related to this m_tag
            .with_splat(Radius(0.125))?
            .send(rerun_session)?;
        } else if let ReplayTrackerEvent::UnitBorn(ref unit_born) = game_step.event {
            let unit_type_name = &unit_born.unit_type_name;
            let unit_name_with_creator_ability = match &unit_born.creator_ability_name {
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
            let (unit_size, unit_color) = get_unit_sized_color(
                &unit_name_with_creator_ability,
                unit_born.control_player_id as i64,
            );
            MsgSender::new(format!(
                "Unit/{}/Born",
                s2protocol::tracker_events::unit_tag(
                    unit_born.unit_tag_index,
                    unit_born.unit_tag_recycle
                )
            ))
            .with_time(
                *game_timeline,
                (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
            )
            .with_splat(Point3D::new(
                unit_born.x as f32 / 6f32,
                -1. * unit_born.y as f32 / 6f32,
                0.,
            ))?
            .with_splat(unit_color)? // Find the user_id related to this m_tag
            .with_splat(TextEntry::new(&unit_born.unit_type_name, None))? // Find the user_id related to this m_tag
            .with_splat(Radius(unit_size))?
            .send(rerun_session)?;
        } else if let ReplayTrackerEvent::UnitPosition(unit_pos) = game_step.event {
            for unit_pos_item in unit_pos.to_unit_positions_vec() {
                MsgSender::new(format!("Unit/{}/Position", unit_pos_item.tag,))
                    .with_time(
                        *game_timeline,
                        (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
                    )
                    .with_splat(Point3D::new(
                        unit_pos_item.x as f32 / 24.,
                        -1. * unit_pos_item.y as f32 / 24.,
                        0.,
                    ))?
                    .send(rerun_session)?;
            }
        } else if let ReplayTrackerEvent::PlayerStats(ref player_stats) = game_step.event {
            for stat_entity_value in player_stats.stats.as_prop_name_value_vec() {
                MsgSender::new(format!(
                    "{}/Stats/{}",
                    stat_entity_value.0, player_stats.player_id,
                ))
                .with_time(
                    *game_timeline,
                    (game_loop as f32 * TRACKER_SPEED_RATIO) as i64,
                )
                .with_splat(Scalar::from(stat_entity_value.1 as f64))?
                .with_splat(FREYA_LIGHT_YELLOW)? // Find the user_id related to this m_tag
                .send(rerun_session)?;
            }
        }
    }
    println!("Final Tracker loop: {}", game_loop);
    Ok(())
}
