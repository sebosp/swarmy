//! Leptos view for app configuration.

use leptos::{ev::MouseEvent, prelude::*};
use phosphor_leptos::{Icon, IconWeight, FOLDER_OPEN, QUESTION};
use swarmy_common::*;

use crate::{settings::actions::trigger_set_current_app_config, DisplayBackendStatus};

#[component]
pub fn Config() -> impl IntoView {
    let (_app_settings, set_app_settings) = signal(AppSettings::default());
    let replay_path = RwSignal::new("".to_string());
    let cache_path = RwSignal::new("".to_string());
    let duplicates_path = RwSignal::new("".to_string());
    let main_player_name = RwSignal::new("".to_string());
    let disable_parallelism = RwSignal::new(DEFAULT_DISABLE_PARALLELISM);
    let traverse_max_depth = RwSignal::new(DEFAULT_TRAVERSE_MAX_DEPTH);
    let scan_max_files = RwSignal::new(DEFAULT_SCAN_MAX_FILES);
    let process_max_files = RwSignal::new(DEFAULT_PROCESS_MAX_FILES);
    let min_version = RwSignal::new(DEFAULT_MIN_VERSION);
    let max_version = RwSignal::new(DEFAULT_MAX_VERSION);
    let watch_new_replays = RwSignal::new(false);
    let notify_new_replays = RwSignal::new(false);
    let (backend_response, set_backend_response) = signal(ApiResponse {
        meta: ResponseMeta::incomplete(),
        message: String::new(),
    });
    crate::settings::with_fetch_current_app_config(
        set_app_settings,
        move |settings: AppSettings| {
            *replay_path.write() = settings.replay_path.clone();
            *cache_path.write() = settings.cache_path.clone();
            *duplicates_path.write() = settings.duplicates_path.clone();
            *main_player_name.write() = settings.main_player_name.clone();
            *disable_parallelism.write() = settings.disable_parallelism;
            *traverse_max_depth.write() = settings.traverse_max_depth;
            *scan_max_files.write() = settings.scan_max_files;
            *process_max_files.write() = settings.process_max_files;
            *min_version.write() = settings.min_version;
            *max_version.write() = settings.max_version;
            *watch_new_replays.write() = settings.watch_new_replays;
            *notify_new_replays.write() = settings.notify_new_replays;
            set_backend_response.update(|val| {
                val.meta.success = true;
                val.meta.is_complete = true;
            })
        },
    );
    view! {
        <div class="ml-4 mt-4">
            <h2 class="text-base/7 font-semibold text-white">"Swarmy Settings"</h2>
            <DisplayBackendStatus backend_response />
            <div class="mt-6 space-y-10 border-b border-white/10 pb-12 sm:space-y-0 sm:divide-y sm:divide-white/10 sm:border-t sm:border-t-white/10 sm:pb-0">
                <fieldset>
                    <legend class="sr-only">"Parallelism"</legend>
                    <div class="grid grid-cols-4 gap-4 py-6">
                        <div aria-hidden="true" class="text-sm/6 font-semibold text-white">
                            <p class="text-sm/6 font-semibold text-white">"Parallelism"</p>
                            <p class="text-sm/6 font-semibold text-white">
                                " Status: "
                                <span class=move || {
                                    if disable_parallelism.get() {
                                        "inline-flex items-center rounded-full bg-orange-400/10 px-1.5 py-0.5 text-xs font-medium text-orange-400"
                                    } else {
                                        "inline-flex items-center rounded-full bg-green-400/10 px-1.5 py-0.5 text-xs font-medium text-green-400"
                                    }
                                }>
                                    {move || {
                                        if disable_parallelism.get() {
                                            "Disabled"
                                        } else {
                                            "Enabled"
                                        }
                                    }}
                                </span>
                            </p>
                        </div>
                        <div class="mt-4 col-span-2 sm:mt-0">
                            <div class="max-w-lg space-y-6">
                                <div class="flex gap-3">
                                    <div class="flex h-6 shrink-0 items-center">
                                        <div class="group grid size-4 grid-cols-1">
                                            <input
                                                id="parallelism"
                                                type="checkbox"
                                                name="parallelism"
                                                checked=move || disable_parallelism.get()
                                                on:click=move |_| {
                                                    disable_parallelism.set(!disable_parallelism.get())
                                                }
                                                class="col-start-1 row-start-1 appearance-none rounded-sm border border-white/10 bg-white/5 checked:border-indigo-500 checked:bg-indigo-500 indeterminate:border-indigo-500 indeterminate:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500 disabled:border-white/5 disabled:bg-white/10 disabled:checked:bg-white/10 forced-colors:appearance-auto"
                                            />
                                            <svg
                                                viewBox="0 0 14 14"
                                                fill="none"
                                                class="pointer-events-none col-start-1 row-start-1 size-3.5 self-center justify-self-center stroke-white group-has-disabled:stroke-white/25"
                                            >
                                                <path
                                                    d="M3 8L6 11L11 3.5"
                                                    stroke-width="2"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    class="opacity-0 group-has-checked:opacity-100"
                                                />
                                                <path
                                                    d="M3 7H11"
                                                    stroke-width="2"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    class="opacity-0 group-has-indeterminate:opacity-100"
                                                />
                                            </svg>
                                        </div>
                                    </div>
                                    <div class="text-sm/6">
                                        <label for="parallelism" class="font-medium text-white">
                                            "Disable Parallelism"
                                        </label>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div class="col-span-1 text-primary">
                            <span title="Decreases CPU usage. Disable if you are on a laptop and you experience heating issues.">
                                <Icon
                                    icon=QUESTION
                                    weight=IconWeight::Fill
                                    prop:class="stroke-current"
                                />
                            </span>
                        </div>
                    </div>
                </fieldset>
                <fieldset>
                    <legend class="sr-only">"New replays"</legend>
                    <div class="grid grid-cols-4 gap-4 py-6">
                        <p class="text-sm/6 font-semibold text-white">"New replays"</p>
                        <label class="col-span-2 flex items-center gap-3 text-sm/6 font-medium text-white">
                            <input
                                id="watch_new_replays"
                                type="checkbox"
                                class="size-4 accent-indigo-500"
                                prop:checked=move || watch_new_replays.get()
                                on:change=move |ev| watch_new_replays.set(event_target_checked(&ev))
                            />
                            "Watch the replay path for new replays"
                        </label>
                        <div class="col-span-1 text-primary">
                            <span title="Watches the replay path in the background (inotify on Linux) and tells you when there are replays that are not in the snapshot, Optimize then only parses the new replays and the ones parsed by another version of s2protocol.">
                                <Icon
                                    icon=QUESTION
                                    weight=IconWeight::Fill
                                    prop:class="stroke-current"
                                />
                            </span>
                        </div>
                        <div></div>
                        <label
                            class="col-span-2 flex items-center gap-3 text-sm/6 font-medium text-white"
                            title="Shows a message with the name of each new replay the watcher finds, i.e. when a game ends"
                        >
                            <input
                                id="notify_new_replays"
                                type="checkbox"
                                class="size-4 accent-indigo-500"
                                prop:disabled=move || !watch_new_replays.get()
                                prop:checked=move || notify_new_replays.get()
                                on:change=move |ev| {
                                    notify_new_replays.set(event_target_checked(&ev))
                                }
                            />
                            "Show a message for each new replay found"
                        </label>
                        <div class="col-span-1"></div>
                    </div>
                </fieldset>
            </div>
            <div class="grid grid-cols-4 sm:items-start sm:gap-4 sm:py-6">
                <ConfigStringInput
                    name="cache_path"
                    directory=true
                    label="Map Caches Path"
                    description="The replays only refer to their maps, to analyse the maps the Map Info caches (terrain, start locations, resources) and the maps are downloaded from Blizzard's depot servers into this directory, like StarCraft II does. swarmy-caches in your home directory by default."
                    value=cache_path
                />
                <ConfigStringInput
                    name="main_player_name"
                    label="Main Player Name"
                    description="Your name in your games, without the clan tag, i.e. Sazed: the player the analyses of your own games are about, such as the s2-polars-data-analysis notebook of the games against the Cheater computers."
                    value=main_player_name
                    optional=true
                />
                <ConfigStringInput
                    name="replay_path"
                    directory=true
                    label="Replay Path"
                    description="The parent path for the replay collection to analyse."
                    value=replay_path
                />
                <ConfigStringInput
                    name="duplicates_path"
                    directory=true
                    label="Duplicate Replays Folder"
                    description="The same game is often saved more than once, i.e. copied from another computer or downloaded again from a tournament pack. \
                    Find duplicates in Scan compares the contents of the replays (their sha256) and Move to the duplicates folder moves the extra copies here, \
                    so that a game is counted once in the statistics. The subdirectories they were in are kept. The copy already in the snapshot, or else the first one by path, stays in the Replay Path.\n\n\
                    Nothing is deleted: check the folder and delete it yourself, or move the files back. It must be outside of the Replay Path, \
                    i.e. ~/SC2Replays-duplicates. Leave it empty to only list the duplicates."
                    value=duplicates_path
                    optional=true
                    warning=true
                />
                <ConfigNumberInput
                    name="scan_max_files"
                    label="Max number of files to Scan"
                    description="Limit of the number of files to read, before validating if they are valid/supported."
                    value=scan_max_files
                />
                <ConfigNumberInput
                    name="process_max_files"
                    label="Max number of files to optimize"
                    description="Decreases the size of the optimized snapshot."
                    value=process_max_files
                />
                <ConfigNumberInput
                    name="traverse_max_depth"
                    label="Max directories to traverse"
                    description="Limits how deep the scanner will go into sub-directories."
                    value=traverse_max_depth
                />
                <ConfigNumberInput
                    name="min_version"
                    label="Minimum patch version to process"
                    description="Helps skip very old replays that are not relevant or supported. Set to 0 to disable filter."
                    value=min_version
                />
                <ConfigNumberInput
                    name="max_version"
                    label="Max patch version to process"
                    description="Helps skip new replays that may be considered corrupt/unsupported. Set to 0 to disable filter."
                    value=max_version
                />
                <div class="col-span-1"></div>
                <div class="col-span-2">
                    <button
                        class="btn btn-primary w-full"
                        on:click=move |ev: MouseEvent| {
                            ev.prevent_default();
                            trigger_set_current_app_config(
                                AppSettings {
                                    cache_path: cache_path.get(),
                                    replay_path: replay_path.get(),
                                    duplicates_path: duplicates_path.get(),
                                    main_player_name: main_player_name.get(),
                                    disable_parallelism: disable_parallelism.get(),
                                    traverse_max_depth: traverse_max_depth.get(),
                                    scan_max_files: scan_max_files.get(),
                                    process_max_files: process_max_files.get(),
                                    min_version: min_version.get(),
                                    max_version: max_version.get(),
                                    watch_new_replays: watch_new_replays.get(),
                                    notify_new_replays: notify_new_replays.get(),
                                    ..Default::default()
                                },
                                set_backend_response,
                            )
                        }
                    >
                        "Save"
                    </button>
                </div>
                <div class="col-span-1"></div>
            </div>
        </div>
    }
}

#[component]
pub fn ConfigNumberInput(
    name: &'static str,
    label: &'static str,
    description: &'static str,
    value: RwSignal<i64>,
) -> impl IntoView {
    let (is_invalid_number, set_is_invalid_number) = signal(false);
    view! {
        <div class="mt-2 col-span-1 sm:mt-0">
            <label class="block text-sm/6 font-medium text-white sm:pt-1.5">{label}</label>
        </div>
        <div class="mt-2 col-span-2 sm:mt-0">
            <input
                type="text"
                aria-invalid=move || if is_invalid_number.get() { "true" } else { "false" }
                aria-describedby=move || format!("{}-description", name)
                class=move || {
                    if is_invalid_number.get() {
                        "block w-full rounded-md bg-white/5 px-3 py-1.5 text-red-400 outline-1 -outline-offset-1 outline-red-500/50 placeholder:text-red-400/70 focus:outline-2 focus:-outline-offset-2 focus:outline-red-400 sm:pr-9 sm:max-w-xs sm:text-sm/6"
                    } else {
                        "block w-full rounded-md bg-white/5 px-3 py-1.5 text-base text-white outline-1 -outline-offset-1 outline-white/10 placeholder:text-gray-500 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-500 sm:max-w-xs sm:text-sm/6"
                    }
                }
                on:input=move |ev| {
                    let input_value = event_target_value(&ev);
                    if let Ok(parsed_value) = input_value.parse::<i64>() {
                        value.set(parsed_value);
                        set_is_invalid_number.set(false);
                    } else {
                        set_is_invalid_number.set(true);
                    }
                }
                value=move || value.get().to_string()
            />

            <p
                id=move || format!("{}-description", name)
                class=move || {
                    if is_invalid_number.get() {
                        "mt-2 text-sm text-red-400"
                    } else {
                        "mt-2 text-sm/6 text-gray-400"
                    }
                }
            >
                {move || {
                    if is_invalid_number.get() {
                        "Please enter a valid number ".to_string()
                    } else {
                        "".to_string()
                    }
                }}
            </p>
        </div>
        <div class="col-span-1 text-primary">
            <span title=move || { description.to_string() }>
                <Icon icon=QUESTION weight=IconWeight::Fill prop:class="stroke-current" />
            </span>
        </div>
    }
}

#[component]
pub fn ConfigStringInput(
    name: &'static str,
    label: &'static str,
    description: &'static str,
    value: RwSignal<String>,
    /// An empty optional value is not marked as invalid.
    #[prop(optional)]
    optional: bool,
    /// The description is a warning, shown in an amber tooltip instead of the browser's one.
    #[prop(optional)]
    warning: bool,
    /// The value is a directory, a Browse button picks it with the folder dialog of the system.
    #[prop(optional)]
    directory: bool,
) -> impl IntoView {
    let is_missing = move || !optional && value.get().is_empty();
    let browse_error = RwSignal::new(None::<String>);
    let browse = move |_: MouseEvent| {
        let args = PickFolderArgs {
            directory: value.get_untracked(),
            title: label.to_string(),
        };
        leptos::task::spawn_local(async move {
            match crate::common::invoke_response("pick_folder", &args).await {
                // Empty when the dialog was cancelled.
                Ok(response) if !response.message.is_empty() => {
                    browse_error.set(None);
                    value.set(response.message);
                }
                Ok(_) => {}
                Err(err) => browse_error.set(Some(err)),
            }
        });
    };
    view! {
        <div class="mt-2 col-span-1 sm:mt-0">
            <label class="block text-sm/6 font-medium text-white sm:pt-1.5">{label}</label>
        </div>
        <div class="mt-2 col-span-2 sm:mt-0">
            <div class="flex gap-2">
                <input
                    type="text"
                    aria-invalid=move || if is_missing() { "true" } else { "false" }
                    aria-describedby=move || format!("{}-description", name)
                    class=move || {
                        if is_missing() {
                            "block w-full rounded-md bg-white/5 px-3 py-1.5 text-red-400 outline-1 -outline-offset-1 outline-red-500/50 placeholder:text-red-400/70 focus:outline-2 focus:-outline-offset-2 focus:outline-red-400 sm:pr-9 sm:text-sm/6"
                        } else {
                            "block w-full rounded-md bg-white/5 px-3 py-1.5 text-base text-white outline-1 -outline-offset-1 outline-white/10 placeholder:text-gray-500 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-500 sm:text-sm/6"
                        }
                    }
                    on:input=move |ev| {
                        let input_value = event_target_value(&ev);
                        value.set(input_value)
                    }
                    value=move || value.get().to_string()
                />
                {directory
                    .then(|| {
                        view! {
                            <button
                                type="button"
                                class="btn btn-outline btn-sm shrink-0"
                                title="Pick the folder"
                                on:click=browse
                            >
                                <Icon
                                    icon=FOLDER_OPEN
                                    weight=IconWeight::Light
                                    prop:class="stroke-current"
                                />
                                "Browse"
                            </button>
                        }
                    })}
            </div>
            <crate::common::ErrorMessage error=browse_error />
            <p
                id=move || format!("{}-description", name)
                class=move || {
                    if is_missing() {
                        "mt-2 text-sm/6 text-red-400"
                    } else {
                        "mt-2 text-sm/6 text-gray-400"
                    }
                }
            >
                {move || {
                    if is_missing() {
                        "Please enter a directory".to_string()
                    } else {
                        "".to_string()
                    }
                }}
            </p>

        </div>
        {if warning {
            view! {
                <div class="col-span-1 text-amber-400">
                    <span class="group relative inline-block" tabindex="0" aria-label=description>
                        <Icon icon=QUESTION weight=IconWeight::Fill prop:class="stroke-current" />
                        <span
                            role="tooltip"
                            class="invisible absolute right-0 top-7 z-20 w-96 max-w-[80vw] whitespace-pre-line rounded-md border border-amber-500/60 bg-amber-950 px-3 py-2 text-sm text-amber-100 shadow-lg group-hover:visible group-focus:visible"
                        >
                            {description}
                        </span>
                    </span>
                </div>
            }
                .into_any()
        } else {
            view! {
                <div class="col-span-1 text-primary">
                    <span title=move || { description.to_string() }>
                        <Icon icon=QUESTION weight=IconWeight::Fill prop:class="stroke-current" />
                    </span>
                </div>
            }
                .into_any()
        }}
    }
}

/// The arguments of the pick_folder command.
#[derive(serde::Serialize)]
struct PickFolderArgs {
    directory: String,
    title: String,
}
