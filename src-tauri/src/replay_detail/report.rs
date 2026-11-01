//! A game as a single HTML page to share, i.e. with a coach or a friend: the players, the win
//! chance with its turning points, the three things to fix, the economy against the winners of the
//! matchup, the build orders of the first minutes and the biggest fights. The page has no scripts
//! and no files next to it, the chart is an inline SVG.

use swarmy_common::*;

/// The build orders show the first minutes.
const REPORT_BUILD_ORDER_SECONDS: u32 = 300;

/// The fights shown, the costliest first.
const REPORT_ENGAGEMENTS: usize = 5;

const CHART_WIDTH: f32 = 720.;
const CHART_HEIGHT: f32 = 180.;

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The name of a player without the clan escapes, escaped for HTML.
fn name(player: &str) -> String {
    escape(&clean_player_name_clan_escapes(player))
}

fn time(seconds: u32) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn color([r, g, b, _]: [u8; 4]) -> String {
    format!("rgb({r},{g},{b})")
}

/// The chance of the first team as a line, the second team is its mirror, with the 50% line and
/// the turning points as bands.
fn win_chance_svg(chances: &WinChances, first_color: &str, second_color: &str) -> String {
    let Some(last) = chances.points.last() else {
        return String::new();
    };
    let start = chances.points[0].seconds as f32;
    let span = (last.seconds as f32 - start).max(1.);
    let x = |seconds: u32| (seconds as f32 - start) / span * CHART_WIDTH;
    let y = |chance: f32| (1. - chance) * CHART_HEIGHT;
    let line = |chance: fn(&WinChancePoint) -> f32| {
        chances
            .points
            .iter()
            .map(|point| format!("{:.1},{:.1}", x(point.seconds), y(chance(point))))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let bands: String = chances
        .turning_points
        .iter()
        .map(|point| {
            format!(
                r##"<rect x="{:.1}" y="0" width="{:.1}" height="{CHART_HEIGHT}" fill="#facc15" opacity="0.15"/>"##,
                x(point.start_seconds),
                (x(point.end_seconds) - x(point.start_seconds)).max(2.)
            )
        })
        .collect();
    format!(
        r##"<svg viewBox="0 -10 {w} {h}" width="100%" role="img" aria-label="Win chance over the game">
{bands}<line x1="0" y1="{mid}" x2="{CHART_WIDTH}" y2="{mid}" stroke="#9ca3af" stroke-dasharray="4 4"/>
<polyline points="{first}" fill="none" stroke="{first_color}" stroke-width="2"/>
<polyline points="{second}" fill="none" stroke="{second_color}" stroke-width="2" opacity="0.6"/>
<text x="0" y="{label}" font-size="11" fill="#9ca3af">{start_time}</text>
<text x="{CHART_WIDTH}" y="{label}" font-size="11" fill="#9ca3af" text-anchor="end">{end_time}</text>
</svg>"##,
        w = CHART_WIDTH,
        h = CHART_HEIGHT + 30.,
        mid = CHART_HEIGHT / 2.,
        first = line(|point| point.chance),
        second = line(|point| 1. - point.chance),
        label = CHART_HEIGHT + 16.,
        start_time = time(start as u32),
        end_time = time(last.seconds),
    )
}

fn player_section(summary: &ReplayPlayerSummary) -> String {
    let lessons: String = if summary.player.is_computer {
        "<p class=muted>A computer player.</p>".to_string()
    } else if summary.lessons.is_empty() {
        "<p class=muted>Nothing costly.</p>".to_string()
    } else {
        let items: String = summary
            .lessons
            .iter()
            .map(|lesson| {
                format!(
                    "<li><b>{}</b> <span class=muted>~{}</span>: {}</li>",
                    escape(&lesson.title),
                    lesson.cost,
                    escape(&lesson.detail)
                )
            })
            .collect();
        format!("<ol>{items}</ol>")
    };
    let economy: String = summary
        .benchmarks
        .iter()
        .map(|benchmark| {
            let winner = summary
                .winner_benchmarks
                .iter()
                .find(|winner| winner.minute == benchmark.minute);
            let cell = |value: i32, winner_value: Option<i32>| match winner_value {
                Some(winner_value) => format!("{value} <span class=muted>/{winner_value}</span>"),
                None => value.to_string(),
            };
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                time(benchmark.minute * 60),
                cell(benchmark.workers, winner.map(|winner| winner.workers)),
                cell(benchmark.supply, winner.map(|winner| winner.supply)),
                cell(
                    benchmark.minerals_rate + benchmark.vespene_rate,
                    winner.map(|winner| winner.minerals_rate + winner.vespene_rate)
                ),
                cell(benchmark.army_value, winner.map(|winner| winner.army_value)),
            )
        })
        .collect();
    let winners_note = if summary.winner_benchmarks.is_empty() {
        String::new()
    } else {
        format!(
            "<p class=muted>/ the median of the {} winners of the matchup in the patch</p>",
            summary.winner_games
        )
    };
    let build_order: String = summary
        .build_order
        .iter()
        .filter(|entry| entry.seconds <= REPORT_BUILD_ORDER_SECONDS && !entry.is_worker)
        .map(|entry| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                time(entry.seconds),
                entry.supply,
                escape(&entry.name)
            )
        })
        .collect();
    let energy: String = summary
        .ability_efficiency
        .iter()
        .map(|efficiency| {
            format!(
                "<li>{}: {} of ~{:.0} ({:.0}%)</li>",
                escape(&efficiency.ability),
                efficiency.uses,
                efficiency.possible,
                100. * efficiency.uses as f32 / efficiency.possible.max(1.)
            )
        })
        .collect();
    format!(
        r#"<section class=player>
<h2><span class=dot style="background:{color}"></span>{name} <span class=muted>{race}, {result}</span></h2>
<h3>Three things to fix</h3>{lessons}
<h3>Economy</h3>
<table><tr><th>Time</th><th>Workers</th><th>Supply</th><th>Income/min</th><th>Army value</th></tr>{economy}</table>{winners_note}
{energy_section}
<h3>Build order until {build_time}</h3>
<table><tr><th>Time</th><th>Supply</th><th></th></tr>{build_order}</table>
</section>"#,
        color = color(summary.player.color),
        name = name(&summary.player.name),
        race = escape(&summary.player.race),
        result = escape(&summary.player.result),
        energy_section = if energy.is_empty() {
            String::new()
        } else {
            format!("<h3>Energy used until 10:00</h3><ul>{energy}</ul>")
        },
        build_time = time(REPORT_BUILD_ORDER_SECONDS),
    )
}

/// The page of the game.
pub fn render_report(detail: &ReplayDetail) -> String {
    let team_color = |team: &[String]| {
        detail
            .players
            .iter()
            .find(|summary| team.first() == Some(&summary.player.name))
            .map(|summary| color(summary.player.color))
            .unwrap_or_else(|| "#9ca3af".to_string())
    };
    let win_chance = detail
        .win_chances
        .as_ref()
        .filter(|chances| !chances.points.is_empty())
        .map(|chances| {
            let teams = |team: &[String]| {
                team.iter().map(|player| name(player)).collect::<Vec<_>>().join(" &amp; ")
            };
            let turning: String = chances
                .turning_points
                .iter()
                .map(|point| {
                    let favored = if point.change > 0. {
                        teams(&chances.first_team)
                    } else {
                        teams(&chances.second_team)
                    };
                    format!(
                        "<li>{}-{}: {favored} +{:.0}%, {}</li>",
                        time(point.start_seconds),
                        time(point.end_seconds),
                        point.change.abs() * 100.,
                        escape(point.engagement.as_deref().unwrap_or("no fight"))
                    )
                })
                .collect();
            format!(
                r#"<section><h2>Win chance</h2>
<p class=muted><span style="color:{first_color}">{first}</span> against <span style="color:{second_color}">{second}</span>, from the army value, income, workers and resources killed of each side. It does not see the mistakes: a side ahead can still lose.</p>
{svg}<ul>{turning}</ul></section>"#,
                first_color = team_color(&chances.first_team),
                second_color = team_color(&chances.second_team),
                first = teams(&chances.first_team),
                second = teams(&chances.second_team),
                svg = win_chance_svg(
                    chances,
                    &team_color(&chances.first_team),
                    &team_color(&chances.second_team)
                ),
            )
        })
        .unwrap_or_default();
    let mut engagements: Vec<&Engagement> = detail.engagements.iter().collect();
    let lost = |engagement: &Engagement| -> u32 {
        engagement
            .losses
            .iter()
            .map(|loss| loss.resources.minerals + loss.resources.vespene)
            .sum()
    };
    engagements.sort_by_key(|engagement| std::cmp::Reverse(lost(engagement)));
    let fights: String = engagements
        .iter()
        .take(REPORT_ENGAGEMENTS)
        .map(|engagement| {
            let losses: Vec<String> = engagement
                .losses
                .iter()
                .map(|loss| {
                    format!(
                        "{} lost {} ({})",
                        name(&loss.player),
                        loss.resources.minerals + loss.resources.vespene,
                        escape(&loss.units)
                    )
                })
                .collect();
            format!(
                "<li>{}-{}: {}</li>",
                time(engagement.start_seconds),
                time(engagement.end_seconds),
                losses.join(", ")
            )
        })
        .collect();
    let players: String = detail.players.iter().map(player_section).collect();
    // The players by team, i.e. `Sazed vs Cheater 1 (Insane) & Cheater 2 (Insane)`.
    let mut teams: Vec<(u8, Vec<String>)> = vec![];
    for summary in &detail.players {
        let player = clean_player_name_clan_escapes(&summary.player.name);
        match teams
            .iter_mut()
            .find(|(team, _)| *team == summary.player.team_id)
        {
            Some((_, players)) => players.push(player),
            None => teams.push((summary.player.team_id, vec![player])),
        }
    }
    let title = format!(
        "{} on {}",
        teams
            .iter()
            .map(|(_, players)| players.join(" & "))
            .collect::<Vec<_>>()
            .join(" vs "),
        detail.map_title
    );
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
body {{ font-family: system-ui, sans-serif; background: #111827; color: #e5e7eb; margin: 0 auto; max-width: 1100px; padding: 16px; }}
h1 {{ font-size: 1.4rem; }} h2 {{ font-size: 1.1rem; margin: 1rem 0 0.3rem; }} h3 {{ font-size: 0.95rem; margin: 0.8rem 0 0.2rem; }}
.muted {{ color: #9ca3af; font-weight: normal; }}
.players {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 16px; }}
.player {{ border: 1px solid #374151; border-radius: 6px; padding: 8px 12px; }}
.dot {{ display: inline-block; width: 12px; height: 12px; border-radius: 6px; margin-right: 6px; }}
table {{ border-collapse: collapse; font-size: 0.85rem; }} td, th {{ padding: 1px 10px 1px 0; text-align: left; }}
li {{ font-size: 0.9rem; margin: 2px 0; }}
@media (prefers-color-scheme: light) {{ body {{ background: #ffffff; color: #111827; }} .player {{ border-color: #d1d5db; }} }}
</style></head><body>
<h1>{title_html}</h1>
<p class=muted>{date}, {duration}, patch {patch}. Made with swarmy.</p>
{win_chance}
<div class=players>{players}</div>
<section><h2>The biggest fights</h2><ul>{fights}</ul></section>
</body></html>
"#,
        title = escape(&title),
        title_html = escape(&title),
        date = escape(&detail.replay_date),
        duration = time(detail.duration_seconds),
        patch = detail.protocol_version,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_escapes_the_names_and_draws_the_chart() {
        let mut detail = ReplayDetail {
            map_title: "Pylon <LE>".to_string(),
            duration_seconds: 600,
            ..Default::default()
        };
        let mut player = ReplayPlayerSummary::default();
        player.player.name = "&lt;chezs&gt;<sp/>Sazed".to_string();
        player.lessons = vec![Lesson {
            title: "Supply blocked".to_string(),
            detail: "60 seconds".to_string(),
            cost: 1200,
            seconds: None,
        }];
        detail.players = vec![player];
        detail.win_chances = Some(WinChances {
            first_team: vec!["&lt;chezs&gt;<sp/>Sazed".to_string()],
            second_team: vec![],
            points: vec![
                WinChancePoint {
                    seconds: 120,
                    chance: 0.5,
                    ..Default::default()
                },
                WinChancePoint {
                    seconds: 150,
                    chance: 0.8,
                    ..Default::default()
                },
            ],
            turning_points: vec![],
        });
        let html = render_report(&detail);
        assert!(html.contains("Pylon &lt;LE&gt;"));
        assert!(html.contains("Sazed"));
        assert!(!html.contains("<sp/>"));
        assert!(html.contains("<polyline"));
        assert!(html.contains("Supply blocked"));
    }
}

/// Writes the report of a game of a snapshot:
/// SWARMY_TEST_REPLAY_PATH=~/SC2Replays SWARMY_TEST_REPLAY_ID=56576 SWARMY_TEST_OUT=report.html cargo test report_of -- --ignored
#[test]
#[ignore]
fn report_of_a_replay() {
    let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
    let id = std::env::var("SWARMY_TEST_REPLAY_ID")
        .unwrap()
        .parse()
        .unwrap();
    let detail = super::data::try_query_replay_detail(replay_path, id).unwrap();
    let html = render_report(&detail);
    std::fs::write(std::env::var("SWARMY_TEST_OUT").unwrap(), &html).unwrap();
    assert!(html.contains("Three things to fix"));
}
