use chrono::DateTime;
use chrono::{NaiveDateTime, Utc};
use leptos::prelude::*;
use phosphor_leptos::{
    BOXING_GLOVE, CALENDAR_DOT, CALENDAR_STAR, CIRCUITRY, HARD_DRIVE, Icon, IconWeight,
    IconWeightData,
};
use si_scale::helpers::bibytes2;
use std::time::UNIX_EPOCH;
use swarmy_tauri_common::*;

pub fn time_ago(date: NaiveDateTime) -> String {
    let now = Utc::now().naive_utc();
    let duration = now.signed_duration_since(date);
    let num_days = duration.num_days();
    let num_hours = duration.num_hours();
    let num_minutes = duration.num_minutes();

    if num_days > 1 {
        match num_days {
            0 => "today".to_string(),
            1 => "yesterday".to_string(),
            2..=6 => format!("{} days ago", duration.num_days()),
            7..=13 => "1 week ago".to_string(),
            14..=20 => "2 weeks ago".to_string(),
            21..=27 => "3 weeks ago".to_string(),
            28..=30 => "1 month ago".to_string(),
            31..=59 => format!("{} months ago", duration.num_days() / 30),
            60..=364 => format!("{} months ago", duration.num_days() / 30),
            365..=729 => "1 year ago".to_string(),
            _ => format!("{} years ago", duration.num_days() / 365),
        }
    } else if num_hours > 1 {
        match num_hours {
            1 => "1 hour ago".to_string(),
            _ => format!("{} hours ago", duration.num_hours()),
        }
    } else if num_minutes > 1 {
        match num_minutes {
            1 => "1 minute ago".to_string(),
            _ => format!("{} minutes ago", duration.num_minutes()),
        }
    } else {
        "just now".to_string()
    }
}

#[component]
pub fn StatDescriptionItem(
    name: String,
    value: ReadSignal<String>,
    description: ReadSignal<String>,
    icon: &'static IconWeightData,
) -> impl IntoView {
    view! {
        <div class="relative overflow-hidden rounded-lg px-4 pt-4 pb-4 shadow-sm sm:px-3 sm:pt-3 bg-gray-800/75 inset-ring inset-ring-white/10">
            <dt>
                <div class="absolute rounded-md bg-indigo-500 p-2">
                    <Icon
                        icon=icon
                        weight=IconWeight::Bold
                        size="24px"
                        prop:fill="none"
                        prop:stroke="currentColor"
                        prop:stroke-width="1.5"
                        prop:data-slot="icon"
                        prop:aria-hidden="true"
                        prop:class="size-6 text-white"
                    />
                </div>
                <p class="ml-16 truncate text-sm font-medium text-gray-400">{name}</p>
            </dt>
            <dd class="ml-16 flex">
                <p class="text-2xl font-semibold text-white" title=description>
                    {value}
                </p>
            </dd>
        </div>
    }
}

#[component]
pub fn ArrowIpcStats(snapshot_stats: ReadSignal<SnapshotStats>) -> impl IntoView {
    let (modif_dt, _set_modif_dt) = signal(
        DateTime::from_timestamp(
            snapshot_stats
                .get()
                .date_modified
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            0,
        )
        .unwrap_or_default(),
    );
    let (rel_modif_dt, _set_rel_modif_dt) = signal(time_ago(modif_dt.get().naive_utc()));
    let (abs_modif_dt, _set_abs_modif_dt) =
        signal(modif_dt.get().format("%Y-%m-%d %H:%M:%S %Z").to_string());
    let (num_games, _set_num_games) = signal(snapshot_stats.get().num_games.to_string());
    let (processed_description, _set_processed_description) = signal("SC2Replay files".to_string());
    let (snapshot_size, _set_snapshot_size) =
        signal(bibytes2(snapshot_stats.get().ipc_dir_size as f64));
    let (snapshot_size_description, _set_snapshot_size_description) =
        signal(format!("{}/ directory", IPC_DIR));
    let (num_maps_value, _set_num_maps_value) = signal(snapshot_stats.get().num_maps.to_string());
    let (num_maps_description, _set_num_maps_description) = signal("unique maps".to_string());
    let (snapshot_min_date, _set_snapshot_min_date) =
        signal(snapshot_stats.get().min_date.format("%Y-%m-%d").to_string());
    let (snapshot_min_date_description, _set_snapshot_min_date_description) =
        signal("Minimum replay date".to_string());
    let (snapshot_max_date, _set_snapshot_max_date) =
        signal(snapshot_stats.get().max_date.format("%Y-%m-%d").to_string());
    let (snapshot_max_date_description, _set_snapshot_max_date_description) =
        signal("Maximum replay date".to_string());
    view! {
        <div>
            <h3 class="text-base font-semibold text-white ml-2">Snapshot Stastics</h3>

            <dl class="px-5 mt-2 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                <StatDescriptionItem
                    name="Optimized".to_string()
                    value=rel_modif_dt
                    description=abs_modif_dt
                    icon=CALENDAR_STAR
                />
                <StatDescriptionItem
                    name="Processed".to_string()
                    value=num_games
                    description=processed_description
                    icon=BOXING_GLOVE
                />
                <StatDescriptionItem
                    name="Snapshot Size".to_string()
                    value=snapshot_size
                    description=snapshot_size_description
                    icon=HARD_DRIVE
                />
                <StatDescriptionItem
                    name="Maps".to_string()
                    value=num_maps_value
                    description=num_maps_description
                    icon=CIRCUITRY
                />
                <StatDescriptionItem
                    name="From:".to_string()
                    value=snapshot_min_date
                    description=snapshot_min_date_description
                    icon=CALENDAR_DOT
                />
                <StatDescriptionItem
                    name="To:".to_string()
                    value=snapshot_max_date
                    description=snapshot_max_date_description
                    icon=CALENDAR_DOT
                />
            </dl>
        </div>
    }
}
