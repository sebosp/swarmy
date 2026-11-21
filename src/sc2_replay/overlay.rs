//! The overlay of a replay played in StarCraft II: a small window of swarmy, always on top of the
//! game, shown once the game plays, moved by dragging it and collapsed to a thin button at its
//! right (» and «). It has the controls of the replay (the game's own do not work while swarmy
//! drives it) and shows the moment the game is at, never what comes after it: the win chance until
//! now (the points of Replay Detail, the same percentages; the first team above, the second
//! below), and for a few seconds an upgrade when it finishes, against the average of the patch,
//! and the first kill of a player when it happens. The game refuses to draw during a replay, so
//! the overlay is a window on top of it, as the overlays of the streamers.
//!
//! It is drawn over the game whatever the theme of swarmy: a dark plate, light text, the status
//! colors of the dataviz palette (good `#0ca30c`, critical `#d03b3b`) with their words.

use super::{send_control, set_overlay_layout, watch_sc2_state};
use crate::common::{format_game_time, invoke_json};
use crate::components::unit_icon::{UnitIcon, provide_unit_icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{CARET_DOUBLE_LEFT, CARET_DOUBLE_RIGHT, Icon, IconWeight, PAUSE, PLAY};
use std::time::Duration;
use swarmy_common::*;

/// The address of the overlay in the window, `main` mounts it instead of the app.
pub const OVERLAY_ROUTE: &str = "#/sc2-overlay";

const PLATE: &str = "background: rgba(14, 16, 20, 0.92); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 10px; color: #f0efec;";
const QUIET: &str = "color: #c3c2b7;";
const GOOD: &str = "color: #0ca30c;";
const CRITICAL: &str = "color: #d03b3b;";
const BUTTON: &str = "background: rgba(255, 255, 255, 0.08); color: #f0efec; border: 1px solid rgba(255, 255, 255, 0.16); border-radius: 6px; width: 28px; height: 28px; display: inline-flex; align-items: center; justify-content: center; cursor: pointer;";

/// How long an upgrade finished or a first kill stays in the overlay, in seconds of the game.
pub const POP_UP_SECONDS: u32 = 20;
/// The size of the win chance chart, in the units of its view box.
const CHART: (f32, f32) = (456., 64.);

fn player_color(player: &ReplayPlayer) -> String {
    let [r, g, b, _] = player.color;
    format!("rgb({r}, {g}, {b})")
}

/// The colors of the players of a team, in its order.
fn team_colors(detail: &ReplayDetail, team: &[String]) -> Vec<String> {
    let colors: Vec<String> = team
        .iter()
        .filter_map(|name| {
            detail
                .players
                .iter()
                .find(|summary| &summary.player.name == name)
        })
        .map(|summary| player_color(&summary.player))
        .collect();
    if colors.is_empty() {
        vec!["rgb(128, 128, 128)".to_string()]
    } else {
        colors
    }
}

/// The points of the win chance until a second, the last one is the chance of that moment (the
/// one Replay Detail shows for it): nothing after it, no spoiler.
pub fn points_until(points: &[WinChancePoint], seconds: u32) -> &[WinChancePoint] {
    let count = points
        .iter()
        .take_while(|point| point.seconds <= seconds)
        .count();
    &points[..count]
}

/// The area of the win chance between its line and the even middle, on a time axis of the whole
/// game: `M` to the middle at the first point, the line, back to the middle at the last one.
pub fn area_path(points: &[WinChancePoint], duration_seconds: u32) -> String {
    let (width, height) = CHART;
    let x = |seconds: u32| seconds as f32 / duration_seconds.max(1) as f32 * width;
    let y = |chance: f32| height / 2. - (chance - 0.5) * height;
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return String::new();
    };
    let mut path = format!("M{:.1},{:.1}", x(first.seconds), height / 2.);
    for point in points {
        path.push_str(&format!(" L{:.1},{:.1}", x(point.seconds), y(point.chance)));
    }
    path.push_str(&format!(" L{:.1},{:.1} Z", x(last.seconds), height / 2.));
    path
}

/// The line of the win chance alone, over the area.
fn line_path(points: &[WinChancePoint], duration_seconds: u32) -> String {
    let (width, height) = CHART;
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            format!(
                "{}{:.1},{:.1}",
                if index == 0 { "M" } else { " L" },
                point.seconds as f32 / duration_seconds.max(1) as f32 * width,
                height / 2. - (point.chance - 0.5) * height
            )
        })
        .collect()
}

/// An upgrade's time against the average of the patch, i.e. "5s faster" in the good color.
pub fn against_average(seconds: u32, average: &PatchAverage) -> (String, &'static str) {
    let difference = seconds as i64 - average.seconds as i64;
    let amount = |seconds: u64| {
        if seconds < 60 {
            format!("{seconds}s")
        } else {
            format_game_time(seconds as u32)
        }
    };
    match difference {
        0 => ("on the average".to_string(), QUIET),
        d if d < 0 => (format!("{} faster", amount(d.unsigned_abs())), GOOD),
        d => (format!("{} slower", amount(d as u64)), CRITICAL),
    }
}

/// Moves the window with the mouse, as its title bar would: the overlay has none.
fn start_dragging() {
    call_current_window("startDragging", None);
}

/// Resizes the window with the mouse from an edge or corner (`"South"`, `"SouthWest"`...), as
/// its borders would: the overlay has none.
fn start_resizing(direction: &str) {
    call_current_window("startResizeDragging", Some(direction));
}

/// Calls a method of Tauri's current window, from the global `__TAURI__` API.
fn call_current_window(method: &str, argument: Option<&str>) {
    use wasm_bindgen::JsCast;
    let get = |target: &wasm_bindgen::JsValue, key: &str| {
        js_sys::Reflect::get(target, &wasm_bindgen::JsValue::from_str(key)).ok()
    };
    let current = get(&window(), "__TAURI__")
        .and_then(|tauri| get(&tauri, "window"))
        .and_then(|module| get(&module, "getCurrentWindow"))
        .and_then(|function| function.dyn_into::<js_sys::Function>().ok())
        .and_then(|function| function.call0(&wasm_bindgen::JsValue::NULL).ok());
    if let Some(current) = current
        && let Some(function) =
            get(&current, method).and_then(|function| function.dyn_into::<js_sys::Function>().ok())
    {
        let _ = match argument {
            Some(argument) => function.call1(&current, &wasm_bindgen::JsValue::from_str(argument)),
            None => function.call0(&current),
        };
    }
}

/// Whether the mouse went down on a button, not on the plate that moves the window.
fn on_button(ev: &leptos::ev::MouseEvent) -> bool {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        .and_then(|element| element.closest("button").ok().flatten())
        .is_some()
}

/// The win chance until now, on the time of the whole game: the area of each team in the colors
/// of its players (one band each), the line, the even middle.
#[component]
fn WinChanceSoFar(detail: ReplayDetail, seconds: u32, duration_seconds: u32) -> impl IntoView {
    let chances = detail.win_chances.clone()?;
    let points = points_until(&chances.points, seconds).to_vec();
    let (width, height) = CHART;
    let middle = height / 2.;
    let area = area_path(&points, duration_seconds);
    let line = line_path(&points, duration_seconds);
    let name = |team: &[String]| team.join(" & ");
    let (first, second) = (name(&chances.first_team), name(&chances.second_team));
    let (first_colors, second_colors) = (
        team_colors(&detail, &chances.first_team),
        team_colors(&detail, &chances.second_team),
    );
    // A side of the middle split in one band per player of its team.
    let bands = move |colors: Vec<String>, from: f32, to: f32, prefix: &'static str| {
        let step = (to - from) / colors.len() as f32;
        let area = area.clone();
        colors
            .into_iter()
            .enumerate()
            .map(move |(index, color)| {
                let id = format!("{prefix}-{index}");
                view! {
                    <clipPath id=id.clone()>
                        <rect x="0" y=from + step * index as f32 width=width height=step />
                    </clipPath>
                    <path d=area.clone() fill=color fill-opacity="0.55" clip-path=format!("url(#{id})") />
                }
            })
            .collect_view()
    };
    let now = points.last().map(|point| point.chance)?;
    let side = |arrow: &'static str, team: String, chance: f32, colors: Vec<String>| {
        view! {
            <div style="display: flex; align-items: center; gap: 4px; font-size: 13px; font-variant-numeric: tabular-nums;">
                <span style=QUIET>{arrow}</span>
                {colors
                    .into_iter()
                    .map(|color| {
                        view! {
                            <span style=format!("display: inline-block; width: 9px; height: 9px; border-radius: 2px; background: {color};")></span>
                        }
                    })
                    .collect_view()}
                <span style="font-weight: 600;">{team}</span>
                <span>{format!("{:.0}%", chance * 100.)}</span>
            </div>
        }
    };
    Some(view! {
        {side("▲", first.clone(), now, first_colors.clone())}
        <svg
            viewBox=format!("0 0 {width} {height}")
            preserveAspectRatio="none"
            style="width: 100%; height: 25vh; min-height: 48px; flex: none; display: block;"
            role="img"
            aria-label="Win chance until now"
        >
            {bands(first_colors.clone(), 0., middle, "above")}
            {bands(second_colors.clone(), middle, height, "below")}
            <line x1="0" x2=width y1=middle y2=middle stroke="rgba(255, 255, 255, 0.3)" stroke-width="1" />
            <path d=line fill="none" stroke="#f0efec" stroke-width="1.5" vector-effect="non-scaling-stroke" />
        </svg>
        {side("▼", second.clone(), 1. - now, second_colors)}
    })
}

/// What happened in the last [`POP_UP_SECONDS`]: the upgrades finished, with their time against
/// the average of the patch, and the first kill of a player. Nothing otherwise.
#[component]
fn PopUps(detail: ReplayDetail, seconds: u32) -> impl IntoView {
    let recent = |at: u32| at <= seconds && seconds - at < POP_UP_SECONDS;
    let mut upgrades: Vec<(BuildOrderEntry, String)> = detail
        .players
        .iter()
        .flat_map(|summary| {
            let color = player_color(&summary.player);
            summary
                .build_order
                .iter()
                .filter(|entry| entry.is_upgrade && recent(entry.seconds))
                .map(move |entry| (entry.clone(), color.clone()))
        })
        .collect();
    upgrades.sort_by_key(|(entry, _)| std::cmp::Reverse(entry.seconds));
    let kills: Vec<(String, String, u32)> = detail
        .players
        .iter()
        .filter_map(|summary| {
            let at = summary.first_kill_seconds.filter(|at| recent(*at))?;
            Some((
                summary.player.name.clone(),
                player_color(&summary.player),
                at,
            ))
        })
        .collect();
    let swatch = |color: String| {
        view! {
            <span style=format!("display: inline-block; width: 4px; height: 18px; border-radius: 2px; background: {color}; flex: none;")></span>
        }
    };
    view! {
        {upgrades
            .into_iter()
            .map(|(entry, color)| {
                let against = entry
                    .patch_average
                    .as_ref()
                    .map(|average| against_average(entry.seconds, average));
                view! {
                    <div style="display: flex; align-items: center; gap: 6px; font-size: 13px; white-space: nowrap;">
                        {swatch(color)}
                        <UnitIcon unit_name=entry.name.clone() size="w-5 h-5" />
                        <span style="overflow: hidden; text-overflow: ellipsis;">{entry.name.clone()}</span>
                        <span style=format!("{QUIET} font-variant-numeric: tabular-nums;")>{format_game_time(entry.seconds)}</span>
                        {against
                            .map(|(text, style)| {
                                view! {
                                    <span
                                        style=format!("{style} margin-left: auto; font-weight: 600;")
                                        title="Against the average time of this upgrade in the games of the same patch"
                                    >
                                        {text}
                                    </span>
                                }
                            })}
                    </div>
                }
            })
            .collect_view()}
        {kills
            .into_iter()
            .map(|(name, color, at)| {
                view! {
                    <div style="display: flex; align-items: center; gap: 6px; font-size: 13px; white-space: nowrap;">
                        {swatch(color)}
                        <span style="font-weight: 600;">"First kill"</span>
                        <span>{name}</span>
                        <span style=format!("{QUIET} font-variant-numeric: tabular-nums;")>{format_game_time(at)}</span>
                    </div>
                }
            })
            .collect_view()}
    }
}

/// Play and pause, the speed (a click goes to the next one) and the time to jump to.
#[component]
fn Controls(state: RwSignal<Sc2ReplayState>, error: RwSignal<Option<String>>) -> impl IntoView {
    let dragged = RwSignal::new(None::<u32>);
    let control = move |control: Sc2ReplayControl| send_control(control, error);
    let speed_label = |speed: f32| {
        if speed < 1. {
            "½×".to_string()
        } else {
            format!("{speed}×")
        }
    };
    view! {
        <button
            style=BUTTON
            title=move || if state.get().playing { "Pause" } else { "Play" }
            on:click=move |_| {
                if state.get_untracked().playing {
                    control(Sc2ReplayControl::Pause)
                } else {
                    control(Sc2ReplayControl::Play)
                }
            }
        >
            {move || {
                if state.get().playing {
                    view! { <Icon icon=PAUSE weight=IconWeight::Fill size="14px" /> }
                } else {
                    view! { <Icon icon=PLAY weight=IconWeight::Fill size="14px" /> }
                }
            }}
        </button>
        <button
            style=format!("{BUTTON} width: 40px; font-size: 13px; font-variant-numeric: tabular-nums;")
            title="The speed, against a ladder game: a click goes to the next one"
            on:click=move |_| {
                let current = state.get_untracked().speed;
                let next = SC2_REPLAY_SPEEDS
                    .iter()
                    .copied()
                    .find(|speed| *speed > current)
                    .unwrap_or(SC2_REPLAY_SPEEDS[0]);
                control(Sc2ReplayControl::Speed { speed: next })
            }
        >
            {move || speed_label(state.get().speed)}
        </button>
        <input
            type="range"
            style="flex: 1; min-width: 0; accent-color: #f0efec;"
            aria-label="Game time"
            title="Jump to a time"
            min="0"
            max=move || state.get().duration_seconds().to_string()
            prop:value=move || dragged.get().unwrap_or(state.get().seconds()).to_string()
            on:input=move |ev| dragged.set(event_target_value(&ev).parse().ok())
            on:change=move |ev| {
                if let Ok(seconds) = event_target_value(&ev).parse() {
                    control(Sc2ReplayControl::Seek { seconds });
                }
                dragged.set(None);
            }
        />
        <span style="font-size: 12px; font-variant-numeric: tabular-nums; white-space: nowrap;">
            {move || format_game_time(dragged.get().unwrap_or(state.get().seconds()))}
            <span style=QUIET>{move || format!(" / {}", format_game_time(state.get().duration_seconds()))}</span>
        </span>
    }
}

/// The overlay, mounted alone in its window: hidden until the game plays, resized by the user, the
/// win chance chart takes the height left.
#[component]
pub fn Sc2Overlay() -> impl IntoView {
    // The window is transparent, the page too: the game shows through the corners.
    if let Some(root) = document().document_element() {
        let _ = root.set_attribute("style", "background: transparent;");
    }
    if let Some(body) = document().body() {
        let _ = body.set_attribute(
            "style",
            "background: transparent; overflow: hidden; margin: 0;",
        );
    }
    provide_unit_icons();
    let state = watch_sc2_state(Duration::from_millis(250));
    let collapsed = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let detail = RwSignal::new(None::<Result<ReplayDetail, String>>);
    // The statistics of the replay, read again when another replay is played.
    Effect::new(move |previous: Option<Option<u64>>| {
        let ext_fs_id = state.with(|state| state.ext_fs_id);
        if previous == Some(ext_fs_id) {
            return ext_fs_id;
        }
        detail.set(None);
        if let Some(ext_fs_id) = ext_fs_id {
            spawn_local(async move {
                let result = invoke_json::<ReplayDetail>(
                    "query_replay_detail",
                    &ReplayDetailQuery { ext_fs_id },
                )
                .await;
                detail.set(Some(result));
            });
        }
        ext_fs_id
    });
    // The window follows the page: hidden without a game, collapsed to its button or open at the
    // size the user gave it.
    let layout = StoredValue::new(None::<(bool, bool)>);
    Effect::new(move |_| {
        let wanted = (state.with(|state| state.active), collapsed.get());
        if layout.get_value() != Some(wanted) {
            layout.set_value(Some(wanted));
            set_overlay_layout(wanted.0, wanted.1);
        }
    });
    let collapse = move |value: bool| collapsed.set(value);
    view! {
        <Show
            when=move || !collapsed.get()
            fallback=move || {
                view! {
                    <button
                        style="width: 100vw; height: 100vh; border: none; border-radius: 6px 0 0 6px; background: rgba(14, 16, 20, 0.92); color: #f0efec; cursor: pointer; padding: 0; display: flex; align-items: center; justify-content: center;"
                        title="Show the overlay"
                        aria-label="Show the overlay"
                        on:click=move |_| collapse(false)
                    >
                        <Icon icon=CARET_DOUBLE_LEFT weight=IconWeight::Bold size="12px" />
                    </button>
                }
            }
        >
            <div
                style=format!("{PLATE} position: relative; box-sizing: border-box; width: 100vw; height: 100vh; overflow: hidden; padding: 6px 10px 10px; display: flex; flex-direction: column; gap: 4px; cursor: move; user-select: none;")
                title="Drag to move the overlay, its edges or the bottom left corner to resize it. The game's own controls do not work while swarmy plays the replay: use these."
                on:mousedown=move |ev| {
                    if ev.button() == 0 && !on_button(&ev) {
                        start_dragging();
                    }
                }
            >
                <div style="display: flex; align-items: center; gap: 6px; cursor: default;">
                    <Controls state error />
                    <button
                        style=BUTTON
                        title="Collapse the overlay to a button"
                        aria-label="Collapse the overlay"
                        on:click=move |_| collapse(true)
                    >
                        <Icon icon=CARET_DOUBLE_RIGHT weight=IconWeight::Bold size="14px" />
                    </button>
                </div>
                {move || {
                    let state = state.get();
                    let message = error
                        .get()
                        .filter(|err| !err.is_empty())
                        .or_else(|| (!state.message.is_empty()).then(|| state.message.clone()))
                        .or_else(|| {
                            state
                                .seeking
                                .map(|target| format!("Jumping to {}", format_game_time(target)))
                        })
                        .or_else(|| match detail.get() {
                            Some(Err(err)) => Some(format!("Unable to read the statistics: {err}")),
                            _ => None,
                        });
                    message.map(|message| view! { <div style=format!("{QUIET} font-size: 12px;")>{message}</div> })
                }}
                {move || {
                    let state = state.get();
                    let Some(Ok(detail)) = detail.get() else {
                        return ().into_any();
                    };
                    let seconds = state.seconds();
                    let duration = state.duration_seconds().max(detail.duration_seconds);
                    view! {
                        <WinChanceSoFar detail=detail.clone() seconds=seconds duration_seconds=duration />
                        <PopUps detail=detail seconds=seconds />
                    }
                        .into_any()
                }}
                <div
                    style="position: absolute; left: 0; right: 0; bottom: 0; height: 6px; cursor: ns-resize;"
                    on:mousedown=move |ev| {
                        ev.stop_propagation();
                        start_resizing("South");
                    }
                ></div>
                <div
                    style="position: absolute; left: 0; top: 0; bottom: 0; width: 6px; cursor: ew-resize;"
                    on:mousedown=move |ev| {
                        ev.stop_propagation();
                        start_resizing("West");
                    }
                ></div>
                <div
                    style="position: absolute; left: 0; bottom: 0; width: 14px; height: 14px; cursor: nesw-resize; border-left: 2px solid rgba(255, 255, 255, 0.35); border-bottom: 2px solid rgba(255, 255, 255, 0.35); border-radius: 0 0 0 10px;"
                    title="Drag to resize the overlay"
                    on:mousedown=move |ev| {
                        ev.stop_propagation();
                        start_resizing("SouthWest");
                    }
                ></div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(seconds: u32, chance: f32) -> WinChancePoint {
        WinChancePoint {
            seconds,
            chance,
            ..Default::default()
        }
    }

    #[test]
    fn it_shows_the_win_chance_until_now_only() {
        let points = vec![point(0, 0.5), point(30, 0.6), point(60, 0.8)];
        assert_eq!(points_until(&points, 45).len(), 2);
        assert_eq!(points_until(&points, 45).last().unwrap().chance, 0.6);
        assert!(points_until(&points, 0).len() == 1);
        assert_eq!(points_until(&points, 600).len(), 3);
    }

    #[test]
    fn it_draws_the_area_from_the_middle() {
        let path = area_path(&[point(0, 0.5), point(100, 1.)], 100);
        assert_eq!(path, "M0.0,32.0 L0.0,32.0 L456.0,0.0 L456.0,32.0 Z");
        assert!(area_path(&[], 100).is_empty());
    }

    #[test]
    fn it_compares_an_upgrade_with_the_patch() {
        let average = PatchAverage {
            seconds: 300,
            samples: 10,
        };
        assert_eq!(
            against_average(295, &average),
            ("5s faster".to_string(), GOOD)
        );
        assert_eq!(
            against_average(372, &average),
            ("01:12 slower".to_string(), CRITICAL)
        );
        assert_eq!(against_average(300, &average).0, "on the average");
    }
}
