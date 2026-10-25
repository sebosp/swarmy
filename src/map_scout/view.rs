//! Where the players send their units early in the game, oriented so that the player always
//! starts on the bottom left of the map.

use crate::components::loading::Loading;
use crate::components::terrain::terrain_rects;
use crate::components::unit_icon::UnitIcon;
use crate::navigation::use_navigation;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

/// The color of a command from early (yellow) to late (red) in the time window.
fn time_color(avg_seconds: f32, max_seconds: u32) -> String {
    let ratio = (avg_seconds / max_seconds.max(1) as f32).clamp(0., 1.);
    let green = (220. * (1. - ratio)) as u8;
    format!("rgb(250,{green},40)")
}

/// The units that usually scout, as quick filters.
const SCOUT_UNITS: [&str; 10] = [
    "SCV", "Probe", "Drone", "Overlord", "Reaper", "Hellion", "Adept", "Oracle", "Zergling",
    "Observer",
];

#[component]
pub fn MapScoutView() -> impl IntoView {
    let navigation = use_navigation();
    // Coming from the Map Details view the map is already chosen.
    let map_title = RwSignal::new(navigation.map_title.get_untracked());
    let player_name = RwSignal::new(String::new());
    let unit_name = RwSignal::new(String::new());
    let max_minutes = RwSignal::new(4u32);
    let layer = RwSignal::new(ScoutLayer::Commands);
    let scout = RwSignal::new(None::<MapScout>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    let fetch = move || {
        let query = MapScoutQuery {
            map_title: map_title.get(),
            player_name: player_name.get(),
            unit_name: unit_name.get(),
            max_seconds: max_minutes.get() * 60,
            layer: layer.get(),
        };
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke_json::<MapScout>("query_map_scout", &query).await {
                Ok(response) => scout.set(Some(response)),
                Err(err) => {
                    scout.set(None);
                    error.set(Some(err));
                }
            }
            loading.set(false);
        });
    };
    if !map_title.get_untracked().is_empty() {
        fetch();
    }
    let input = |label: &'static str, value: RwSignal<String>, placeholder: &'static str| {
        view! {
            <div class="col-span-2">
                <label class="block text-sm/6 font-medium text-white">{label}</label>
                <input
                    class=text_input_tailwind_classes().join(" ")
                    bind:value=value
                    placeholder=placeholder
                    type="text"
                />
            </div>
        }
    };
    // The units that usually scout, clicking their icon filters by them.
    let scouts = SCOUT_UNITS
        .iter()
        .map(|scout| {
            view! {
                <button
                    class="btn btn-ghost btn-xs px-1"
                    class:btn-active=move || unit_name.get() == *scout
                    title=format!("Show the {scout}s")
                    on:click=move |_| {
                        unit_name.update(|unit| {
                            *unit = if unit == scout { String::new() } else { scout.to_string() };
                        })
                    }
                >
                    <UnitIcon unit_name=*scout size="w-6 h-6" />
                    {*scout}
                </button>
            }
        })
        .collect_view();
    view! {
        <div class="p-2 space-y-2">
            <div class="flex flex-wrap gap-1 text-xs">{scouts}</div>
            <div class="grid grid-cols-10 gap-2 items-end">
                {input("Map", map_title, "Killswitch")}
                {input("Player", player_name, "Any player")}
                {input("Unit", unit_name, "Any unit, i.e. SCV, Probe, Overlord")}
                <div class="col-span-2">
                    <label class="block text-sm/6 font-medium text-white">"First minutes"</label>
                    <input
                        class=text_input_tailwind_classes().join(" ")
                        type="number"
                        min="1"
                        max="30"
                        prop:value=move || max_minutes.get().to_string()
                        on:input=move |ev| {
                            if let Ok(minutes) = event_target_value(&ev).parse::<u32>() {
                                max_minutes.set(minutes.max(1));
                            }
                        }
                    />
                </div>
                <div class="col-span-1">
                    <label class="block text-sm/6 font-medium text-white">"Show"</label>
                    <select
                        class="select select-sm w-full bg-gray-800 text-gray-200 border border-white/20"
                        on:change=move |ev| {
                            layer.set(match event_target_value(&ev).as_str() {
                                "deaths" => ScoutLayer::Deaths,
                                _ => ScoutLayer::Commands,
                            });
                        }
                    >
                        <option value="commands">"Commands"</option>
                        <option value="deaths">"Deaths"</option>
                    </select>
                </div>
                <div class="col-span-1">
                    <button class="btn btn-primary btn-sm" on:click=move |_ev: MouseEvent| fetch()>
                        "Scout"
                    </button>
                </div>
            </div>
            <p class="text-xs text-gray-500">
                {move || match layer.get() {
                    ScoutLayer::Commands => "The targets of the commands issued by the players",
                    ScoutLayer::Deaths => "Where the units of the players died",
                }}
                ", rotated so that the player always starts on the bottom left. Yellow is early in \
                the time window, red is late, the more opaque the more often it happened in the square."
            </p>
            <Show when=move || loading.get()>
                <Loading message="Collecting the commands..." />
            </Show>
            <ErrorMessage error />
            {move || scout.get().map(|scout| view! {
                <p class="text-sm text-gray-300">
                    {format!(
                        "{}: {} {} in {} games during the first {} minutes.",
                        scout.map_title,
                        scout.commands,
                        match layer.get_untracked() {
                            ScoutLayer::Commands => "commands",
                            ScoutLayer::Deaths => "deaths",
                        },
                        scout.games,
                        max_minutes.get_untracked(),
                    )}
                </p>
                <ScoutMap scout max_seconds=max_minutes.get_untracked() * 60 />
            })}
        </div>
    }
}

#[component]
fn ScoutMap(scout: MapScout, max_seconds: u32) -> impl IntoView {
    let (width, height) = (scout.width as f32, scout.height as f32);
    let terrain = scout.terrain.as_ref().map(terrain_rects);
    let max_count = scout
        .cells
        .iter()
        .map(|cell| cell.count)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let cell_size = scout.cell_size as f32;
    let scout_rects = scout
        .cells
        .iter()
        .map(|scout_cell| {
            let opacity = 0.25 + 0.75 * scout_cell.count as f32 / max_count;
            let title = format!(
                "({}, {}): {} commands, at {} on average",
                scout_cell.x,
                scout_cell.y,
                scout_cell.count,
                format_game_time(scout_cell.avg_seconds as u32),
            );
            view! {
                <rect
                    x=scout_cell.x as f32
                    y=scout_cell.y as f32
                    width=cell_size
                    height=cell_size
                    fill=time_color(scout_cell.avg_seconds, max_seconds)
                    fill-opacity=opacity
                >
                    <title>{title}</title>
                </rect>
            }
        })
        .collect_view();
    let start_marker = |start: Option<(f32, f32)>, color: &'static str, label: &'static str| {
        start.map(|(x, y)| {
            view! {
                <circle cx=x cy=y r="4" fill="none" stroke=color stroke-width="1.5">
                    <title>{label}</title>
                </circle>
            }
        })
    };
    view! {
        <svg
            viewBox=format!("0 0 {width} {height}")
            class="w-full max-w-4xl border border-white/10 bg-gray-950"
        >
            // The map y axis goes up, the SVG one goes down.
            <g transform=format!("translate(0 {height}) scale(1 -1)")>
                {terrain}
                {scout_rects}
                {start_marker(scout.own_start, "#38bdf8", "Own start location")}
                {start_marker(scout.enemy_start, "#f87171", "Enemy start location")}
            </g>
        </svg>
        <p class="text-xs text-gray-500">
            <span class="text-sky-400">"○ "</span>"own start "
            <span class="text-red-400">"○ "</span>"enemy start"
        </p>
    }
}
