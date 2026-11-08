//! Leptos view for app configuration.

use leptos::{ev::MouseEvent, prelude::*};
use phosphor_leptos::{Icon, IconWeight, FOLDER_OPEN, QUESTION};
use swarmy_common::*;

use crate::components::loading::Loading;
use crate::{
    settings::actions::trigger_set_current_app_config, DisplayBackendStatus, SELECT_CLASS,
};

/// Config: in the browser version only what is of this browser and the administrator key, the
/// paths and the options of the server are set in the server.
#[component]
pub fn Config() -> impl IntoView {
    if crate::components::web_session::is_web() {
        view! { <WebConfig /> }.into_any()
    } else {
        view! { <DesktopConfig /> }.into_any()
    }
}

#[component]
fn WebConfig() -> impl IntoView {
    // The rows in the grid of the app's Config: the label, the field, its help.
    view! {
        <div class="flex flex-col gap-6 p-4 w-full">
            <div class="grid grid-cols-4 items-start gap-4">
                <crate::components::web_session::AdminKeyInput />
                <ConfigAppearance />
                <ConfigBattleNet />
            </div>
        </div>
    }
}

#[component]
fn DesktopConfig() -> impl IntoView {
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
    let remote_backend_url = RwSignal::new("".to_string());
    let remote_backend_token = RwSignal::new("".to_string());
    let appearance = expect_context::<crate::components::appearance::Appearance>();
    let reduce_motion = RwSignal::new(default_reduce_motion());
    let underline_links = RwSignal::new(false);
    let focus_outline = RwSignal::new(false);
    let max_table_rows = RwSignal::new(swarmy_common::DEFAULT_MAX_TABLE_ROWS);
    let (backend_response, set_backend_response) = signal(ApiResponse {
        meta: ResponseMeta::incomplete(),
        message: String::new(),
    });
    // The form is shown once the stored settings are read, not with the defaults first.
    let loaded = RwSignal::new(false);
    // Whether to show what the last Save did.
    let show_saved = RwSignal::new(false);
    crate::settings::with_fetch_stored_app_config(
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
            *remote_backend_url.write() = settings.remote_backend_url.clone();
            *remote_backend_token.write() = settings.remote_backend_token.clone();
            appearance.theme.set(settings.theme.clone());
            appearance.zoom_percent.set(settings.zoom_percent);
            *reduce_motion.write() = settings.reduce_motion.clone();
            *underline_links.write() = settings.underline_links;
            *focus_outline.write() = settings.focus_outline;
            *max_table_rows.write() = if settings.max_table_rows == 0 {
                swarmy_common::DEFAULT_MAX_TABLE_ROWS
            } else {
                settings.max_table_rows
            };
            set_backend_response.update(|val| {
                val.meta.success = true;
                val.meta.is_complete = true;
            });
            loaded.set(true);
        },
    );
    view! {
        <div class="ml-4 mt-4">
            <h2 class="text-base/7 font-semibold text-base-content">"Swarmy Settings"</h2>
            <DisplayBackendStatus backend_response />
            <Show
                when=move || loaded.get()
                fallback=|| view! { <Loading message="Reading the settings..." /> }
            >
            <div class="mt-6 space-y-10 border-b border-base-content/10 pb-12 sm:space-y-0 sm:divide-y sm:divide-base-content/10 sm:border-t sm:border-t-white/10 sm:pb-0">
                <Show when=move || remote_backend_url.get().trim().is_empty()>
                <fieldset>
                    <legend class="sr-only">"Parallelism"</legend>
                    <div class="grid grid-cols-4 gap-4 py-6">
                        <div aria-hidden="true" class="text-sm/6 font-semibold text-base-content">
                            <p class="text-sm/6 font-semibold text-base-content">"Parallelism"</p>
                            <p class="text-sm/6 font-semibold text-base-content">
                                " Status: "
                                <span class=move || {
                                    if disable_parallelism.get() {
                                        "inline-flex items-center rounded-full bg-warning/10 px-1.5 py-0.5 text-xs font-medium text-warning"
                                    } else {
                                        "inline-flex items-center rounded-full bg-success/10 px-1.5 py-0.5 text-xs font-medium text-success"
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
                                                class="col-start-1 row-start-1 appearance-none rounded-sm border border-base-content/10 bg-base-content/5 checked:border-primary checked:bg-primary indeterminate:border-primary indeterminate:bg-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:border-base-content/5 disabled:bg-base-content/10 disabled:checked:bg-base-content/10 forced-colors:appearance-auto"
                                            />
                                            <svg
                                                viewBox="0 0 14 14"
                                                fill="none"
                                                class="pointer-events-none col-start-1 row-start-1 size-3.5 self-center justify-self-center stroke-base-content group-has-disabled:stroke-base-content/25"
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
                                        <label for="parallelism" class="font-medium text-base-content">
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
                </Show>
                <fieldset>
                    <legend class="sr-only">"New replays"</legend>
                    <div class="grid grid-cols-4 gap-4 py-6">
                        <p class="text-sm/6 font-semibold text-base-content">"New replays"</p>
                        <label class="col-span-2 flex items-center gap-3 text-sm/6 font-medium text-base-content">
                            <input
                                id="watch_new_replays"
                                type="checkbox"
                                class="size-4 accent-primary"
                                prop:checked=move || watch_new_replays.get()
                                on:change=move |ev| watch_new_replays.set(event_target_checked(&ev))
                            />
                            {move || {
                                if remote_backend_url.get().trim().is_empty() {
                                    "Watch the replay path for new replays"
                                } else {
                                    "Receive notifications of the new replays of the remote backend"
                                }
                            }}
                        </label>
                        <div class="col-span-1 text-primary">
                            <span title=move || {
                                if remote_backend_url.get().trim().is_empty() {
                                    "Watches the replay path in the background (inotify on Linux) and tells you when there are replays that are not in the snapshot, Optimize then only parses the new replays and the ones parsed by another version of s2protocol."
                                } else {
                                    "The remote backend watches its Replay Path: tells you here when it has replays that are not in its snapshot, i.e. a game just played on that computer, and Refresh adds them."
                                }
                            }>
                                <Icon
                                    icon=QUESTION
                                    weight=IconWeight::Fill
                                    prop:class="stroke-current"
                                />
                            </span>
                        </div>
                        <div></div>
                        <label
                            class="col-span-2 flex items-center gap-3 text-sm/6 font-medium text-base-content"
                            title="Shows a message with the name of each new replay the watcher finds, i.e. when a game ends"
                        >
                            <input
                                id="notify_new_replays"
                                type="checkbox"
                                class="size-4 accent-primary"
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
                    name="remote_backend_url"
                    label="Remote Backend"
                    description="A swarmy that runs the Scan, Optimize and the queries of its snapshot for this one, \
                    for a computer that cannot handle them, i.e. http://wormgat-zt:7878. \
                    Start it on the other computer with: swarmy --serve 0.0.0.0:7878 \
                    The snapshot is then the one of the Replay Path of the Config of the other computer, \
                    its Map Caches, Duplicate Replays Folder and Scan limits are set there too: \
                    the Replay Path of this Config is only for the watcher and to open the replays here. \
                    Leave it empty to run everything in this computer."
                    value=remote_backend_url
                    optional=true
                />
                <ConfigStringInput
                    name="remote_backend_token"
                    label="Remote Backend Token"
                    description="The --token of the remote backend, or its SWARMY_SERVER_TOKEN. Empty when it has none."
                    value=remote_backend_token
                    optional=true
                />
                <CheckRemoteBackend remote_backend_url remote_backend_token />
                <Show when=move || !remote_backend_url.get().trim().is_empty()>
                    <div class="col-span-1"></div>
                    <p class="col-span-2 text-sm/6 text-muted">
                        "The Map Caches Path, the Duplicate Replays Folder, the parallelism and the limits of Scan and Optimize are the ones of the Config of the remote backend, set them on that computer."
                    </p>
                    <div class="col-span-1"></div>
                </Show>
                <Show when=move || remote_backend_url.get().trim().is_empty()>
                <ConfigStringInput
                    name="cache_path"
                    directory=true
                    label="Map Caches Path"
                    description="The replays only refer to their maps, to analyse the maps the Map Info caches (terrain, start locations, resources) and the maps are downloaded from Blizzard's depot servers into this directory, like StarCraft II does. swarmy-caches in your home directory by default."
                    value=cache_path
                />
                </Show>
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
                    description="The parent path for the replay collection to analyse. With a remote backend, the replays of this computer: the watcher follows it and the replays open from it, the snapshot is the one of the remote backend."
                    value=replay_path
                />
                <Show when=move || remote_backend_url.get().trim().is_empty()>
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
                </Show>
                <ConfigAppearance />
                <ConfigAccessibility reduce_motion underline_links focus_outline />
                <ConfigTableRows max_table_rows />
                <ConfigBattleNet />
                <Show when=move || remote_backend_url.get().trim().is_empty()>
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
                </Show>
                <div class="col-span-1"></div>
                <div class="col-span-2">
                    <button
                        class="btn btn-primary w-full"
                        on:click=move |ev: MouseEvent| {
                            show_saved.set(true);
                            // The confirmation goes away by itself, an error stays.
                            leptos::prelude::set_timeout(
                                move || {
                                    if backend_response.get_untracked().meta.success {
                                        show_saved.set(false);
                                    }
                                },
                                std::time::Duration::from_secs(4),
                            );
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
                                    remote_backend_url: remote_backend_url.get(),
                                    remote_backend_token: remote_backend_token.get(),
                                    zoom_percent: appearance.zoom_percent.get(),
                                    theme: appearance.theme.get(),
                                    reduce_motion: reduce_motion.get(),
                                    underline_links: underline_links.get(),
                                    focus_outline: focus_outline.get(),
                                    max_table_rows: max_table_rows.get(),
                                    ..Default::default()
                                },
                                set_backend_response,
                            )
                        }
                    >
                        "Save"
                    </button>
                    // What Save did, beside it: the status at the top of the view is out of sight.
                    <p class="mt-2 text-sm" role="status" aria-live="polite">
                        {move || {
                            let response = backend_response.get();
                            (show_saved.get() && !response.message.is_empty())
                                .then(|| {
                                    if response.meta.success {
                                        view! { <span class="text-success">"✓ Saved"</span> }.into_any()
                                    } else {
                                        view! { <span class="text-error">{response.message}</span> }.into_any()
                                    }
                                })
                        }}
                    </p>
                </div>
                <div class="col-span-1"></div>
            </div>
            </Show>
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
            <label class="block text-sm/6 font-medium text-base-content sm:pt-1.5">{label}</label>
        </div>
        <div class="mt-2 col-span-2 sm:mt-0">
            <input
                type="text"
                aria-invalid=move || if is_invalid_number.get() { "true" } else { "false" }
                aria-describedby=move || format!("{}-description", name)
                class=move || {
                    if is_invalid_number.get() {
                        "block w-full rounded-md bg-base-content/5 px-3 py-1.5 text-error outline-1 -outline-offset-1 outline-error/50 placeholder:text-error/70 focus:outline-2 focus:-outline-offset-2 focus:outline-error sm:pr-9 sm:max-w-xs sm:text-sm/6"
                    } else {
                        "block w-full rounded-md bg-base-content/5 px-3 py-1.5 text-base text-base-content outline-1 -outline-offset-1 outline-base-content/10 placeholder:text-faint focus:outline-2 focus:-outline-offset-2 focus:outline-primary sm:max-w-xs sm:text-sm/6"
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
                        "mt-2 text-sm text-error"
                    } else {
                        "mt-2 text-sm/6 text-muted"
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
            <label class="block text-sm/6 font-medium text-base-content sm:pt-1.5">{label}</label>
        </div>
        <div class="mt-2 col-span-2 sm:mt-0">
            <div class="flex gap-2">
                <input
                    type="text"
                    aria-invalid=move || if is_missing() { "true" } else { "false" }
                    aria-describedby=move || format!("{}-description", name)
                    class=move || {
                        if is_missing() {
                            "block w-full rounded-md bg-base-content/5 px-3 py-1.5 text-error outline-1 -outline-offset-1 outline-error/50 placeholder:text-error/70 focus:outline-2 focus:-outline-offset-2 focus:outline-error sm:pr-9 sm:text-sm/6"
                        } else {
                            "block w-full rounded-md bg-base-content/5 px-3 py-1.5 text-base text-base-content outline-1 -outline-offset-1 outline-base-content/10 placeholder:text-faint focus:outline-2 focus:-outline-offset-2 focus:outline-primary sm:text-sm/6"
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
                        "mt-2 text-sm/6 text-error"
                    } else {
                        "mt-2 text-sm/6 text-muted"
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
                <div class="col-span-1 text-warning">
                    <span class="group relative inline-block" tabindex="0" aria-label=description>
                        <Icon icon=QUESTION weight=IconWeight::Fill prop:class="stroke-current" />
                        <span
                            role="tooltip"
                            class="invisible absolute right-0 top-7 z-20 w-96 max-w-[80vw] whitespace-pre-line rounded-md border border-warning/60 bg-base-200 px-3 py-2 text-sm text-warning shadow-lg group-hover:visible group-focus:visible"
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

/// A row of the Config grid: the label, the control and the help icon with its explanation.
#[component]
fn ConfigRow(label: &'static str, help: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="col-span-1">
            <label class="block text-sm/6 font-medium text-base-content sm:pt-1.5">{label}</label>
        </div>
        <div class="col-span-2">{children()}</div>
        <div class="col-span-1 text-primary">
            <span title=help>
                <Icon icon=QUESTION weight=IconWeight::Fill prop:class="stroke-current" />
            </span>
        </div>
    }
}

/// A `<select>` of `(id, label)` options bound to the signal of the id, `on_pick` runs with each
/// change, i.e. to apply the option at once.
#[component]
fn OptionSelect(
    options: &'static [(&'static str, &'static str)],
    value: RwSignal<String>,
    on_pick: impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <select
            class=format!("{SELECT_CLASS} max-w-xs")
            on:change=move |ev| {
                value.set(event_target_value(&ev));
                on_pick();
            }
        >
            {options
                .iter()
                .map(|(id, label)| {
                    view! {
                        <option value=*id selected=move || value.get() == *id>
                            {*label}
                        </option>
                    }
                })
                .collect_view()}
        </select>
    }
}

/// The theme and the zoom: the theme is applied as it is picked and saved with Save, the zoom is
/// saved as it changes, as Ctrl+= and Ctrl+- do.
#[component]
fn ConfigAppearance() -> impl IntoView {
    let appearance = expect_context::<crate::components::appearance::Appearance>();
    let zoom = move |steps: i32| crate::components::appearance::zoom_step(steps, appearance);
    view! {
        <ConfigRow
            label="Theme"
            help="The colors of the interface, applied as you pick them and kept with Save. Follow the system picks Swarmy or Light from the light or dark preference of your desktop."
        >
            <OptionSelect options=THEMES value=appearance.theme on_pick=|| () />
        </ConfigRow>
        // The browser zooms the page itself (Ctrl+= and Ctrl+-).
        {(!crate::components::web_session::is_web()).then(|| view! {
        <ConfigRow
            label="Zoom"
            help="The size of the whole interface, text, charts and icons, from 50% to 250%. Ctrl+= and Ctrl+- change it in any view, Ctrl+0 resets it; it is saved as it changes."
        >
            <div class="flex items-center gap-2">
                <button type="button" class="btn btn-outline btn-sm" title="Zoom out, Ctrl+-" on:click=move |_| zoom(-1)>
                    "−"
                </button>
                <span class="w-14 text-center text-sm text-base-content">
                    {move || format!("{}%", appearance.zoom_percent.get())}
                </span>
                <button type="button" class="btn btn-outline btn-sm" title="Zoom in, Ctrl+=" on:click=move |_| zoom(1)>
                    "+"
                </button>
                <button type="button" class="btn btn-ghost btn-sm" title="Back to 100%, Ctrl+0" on:click=move |_| zoom(0)>
                    "Reset"
                </button>
            </div>
        </ConfigRow>
        })}
    }
}

/// The accessibility options, applied as they are picked and kept with Save. The chart colors are
/// safe for color vision deficiencies in every theme, the zoom is the text size, see above.
#[component]
fn ConfigAccessibility(
    reduce_motion: RwSignal<String>,
    underline_links: RwSignal<bool>,
    focus_outline: RwSignal<bool>,
) -> impl IntoView {
    let apply = move || {
        crate::components::appearance::apply_accessibility(
            &reduce_motion.get_untracked(),
            underline_links.get_untracked(),
            focus_outline.get_untracked(),
        )
    };
    let toggle = move |label: &'static str, flag: RwSignal<bool>| {
        view! {
            <label class="flex items-center gap-3 text-sm/6 font-medium text-base-content">
                <input
                    type="checkbox"
                    class="toggle toggle-sm"
                    prop:checked=move || flag.get()
                    on:change=move |ev| {
                        flag.set(event_target_checked(&ev));
                        apply();
                    }
                />
                {label}
            </label>
        }
    };
    view! {
        <ConfigRow
            label="Motion"
            help="Reduce stops the animations and transitions, i.e. the spinners and the sliding panels. Follow the system uses the Reduce animations setting of your desktop."
        >
            <OptionSelect options=REDUCE_MOTION_OPTIONS value=reduce_motion on_pick=apply />
        </ConfigRow>
        <ConfigRow
            label="Links and focus"
            help="The names that open another view are underlined with dots; underlined links also show them as solid underlines. The focus outline shows which element the keyboard is on, also after a click."
        >
            <div class="flex flex-col gap-2">
                {toggle("Underline the links, not only on hover", underline_links)}
                {toggle("Always show the focus outline", focus_outline)}
            </div>
        </ConfigRow>
    }
}

/// The rows a table shows at once: the longer tables have pages, a long table blocks the window
/// while it is drawn.
#[component]
fn ConfigTableRows(max_table_rows: RwSignal<u32>) -> impl IntoView {
    view! {
        <ConfigRow
            label="Rows per table"
            help="The tables with more rows than this show them in pages, with Previous and Next under the table. Their headers sort all the rows. Between 10 and 10000."
        >
            <input
                type="number"
                min="10"
                max="10000"
                step="10"
                class="input input-sm w-28 bg-base-200 text-base-content border border-base-content/20 tabular-nums"
                prop:value=move || max_table_rows.get().to_string()
                on:change=move |ev| {
                    if let Ok(rows) = event_target_value(&ev).trim().parse::<u32>() {
                        let rows = rows.clamp(10, 10_000);
                        max_table_rows.set(rows);
                        crate::components::paged::set_table_rows(rows);
                    }
                }
            />
        </ConfigRow>
    }
}

/// The Client ID and Secret of a Battle.net application, for the profiles of the accounts in the
/// Player view. The backend keeps them (with a remote backend, the server) and never sends the
/// secret back; Save checks them with Battle.net first.
#[component]
fn ConfigBattleNet() -> impl IntoView {
    // The credentials are of the backend, set by its administrator: in the browser, after the
    // administrator key above.
    let admin = crate::components::web_session::use_web_session().admin;
    let client_id = RwSignal::new(String::new());
    let client_secret = RwSignal::new(String::new());
    let status = RwSignal::new(None::<BattleNetStatus>);
    let message = RwSignal::new(String::new());
    let refresh = move || {
        leptos::task::spawn_local(async move {
            // A refused or failed request says why instead of checking forever.
            status.set(Some(
                crate::invoke_json::<BattleNetStatus>("get_battlenet_status", &())
                    .await
                    .unwrap_or_else(|error| BattleNetStatus {
                        error,
                        ..Default::default()
                    }),
            ));
        })
    };
    // Again once signed in: the status is of the administrator.
    Effect::new(move |_| {
        if admin.get() {
            refresh();
        }
    });
    let save = move |client_id: String, client_secret: String| {
        #[derive(serde::Serialize)]
        struct Args {
            client_id: String,
            client_secret: String,
        }
        message.set("Checking with Battle.net...".to_string());
        leptos::task::spawn_local(async move {
            match crate::common::invoke_response(
                "set_battlenet_credentials",
                &Args {
                    client_id,
                    client_secret,
                },
            )
            .await
            {
                Ok(response) => message.set(response.message.trim_matches('"').to_string()),
                Err(err) => message.set(err),
            }
            refresh();
        });
    };
    view! {
        <ConfigRow
            label="Battle.net API"
            help="The Client ID and Client Secret of an application created on https://community.developer.battle.net: the Player view then shows the Battle.net profiles of the accounts (best 1v1 finish, ladders and MMR of the season). They are kept by the backend, readable by its user only, the secret is never shown again."
        >
            <div class="flex flex-col gap-2 max-w-md">
                <p class="text-sm" role="status">
                    {move || match status.get() {
                        _ if !admin.get() => view! { <span>"Sign in with the administrator key to set them"</span> }.into_any(),
                        None => view! { <span>"Checking with Battle.net..."</span> }.into_any(),
                        Some(status) if status.configured && status.working => view! {
                            <span class="text-success">"✓ Enabled: "</span>
                            <span>
                                {if status.from_environment { "set by the environment of the backend, " } else { "" }}
                                "Battle.net accepts them, the Player view shows the profiles"
                            </span>
                        }
                        .into_any(),
                        Some(status) if status.configured => view! {
                            <span class="text-error">"✗ Not working: "</span>
                            <span>{status.error}</span>
                        }
                        .into_any(),
                        Some(status) if !status.error.is_empty() => view! {
                            <span class="text-error">"✗ "</span>
                            <span>{status.error}</span>
                        }
                        .into_any(),
                        Some(_) => view! { <span>"Not set: the Player view shows no Battle.net profiles"</span> }.into_any(),
                    }}
                </p>
                <p class="text-sm text-muted">
                    "Create an application, its Client ID and Secret, on "
                    <a
                        class="nav-link text-base-content"
                        href="https://community.developer.battle.net/access/clients"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        "community.developer.battle.net"
                    </a>
                    "; the profiles come from its "
                    <a
                        class="nav-link text-base-content"
                        href="https://community.developer.battle.net/documentation/starcraft-2/community-apis"
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        "StarCraft II community API"
                    </a>
                    "."
                </p>
                <form
                    class="flex flex-col gap-2"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        save(client_id.get_untracked(), client_secret.get_untracked());
                        client_secret.set(String::new());
                    }
                >
                    <input
                        disabled=move || !admin.get()
                        type="text"
                        autocomplete="off"
                        class="input input-sm bg-base-200 text-base-content border border-base-content/20"
                        placeholder="Client ID"
                        aria-label="Battle.net Client ID"
                        prop:value=move || client_id.get()
                        on:input=move |ev| client_id.set(event_target_value(&ev))
                    />
                    <input
                        disabled=move || !admin.get()
                        type="password"
                        autocomplete="off"
                        class="input input-sm bg-base-200 text-base-content border border-base-content/20"
                        placeholder="Client Secret"
                        aria-label="Battle.net Client Secret"
                        prop:value=move || client_secret.get()
                        on:input=move |ev| client_secret.set(event_target_value(&ev))
                    />
                    <div class="flex gap-2">
                        <button type="submit" class="btn btn-primary btn-sm" disabled=move || !admin.get()>"Save"</button>
                        <button
                            type="button"
                            class="btn btn-outline btn-sm"
                            disabled=move || !admin.get() || !status.get().is_some_and(|status| status.configured)
                            title="Ask Battle.net again whether it accepts the Client ID and Secret saved"
                            on:click=move |_| {
                                status.set(None);
                                refresh();
                            }
                        >
                            "Test"
                        </button>
                        <button
                            type="button"
                            class="btn btn-ghost btn-sm"
                            disabled=move || !admin.get()
                            title="Forget the Client ID and Secret saved by the backend"
                            on:click=move |_| save(String::new(), String::new())
                        >
                            "Remove"
                        </button>
                    </div>
                </form>
                <p class="text-sm" role="status" aria-live="polite">{move || message.get()}</p>
            </div>
        </ConfigRow>
    }
}

/// Checks that the remote backend answers, with its versions.
#[component]
fn CheckRemoteBackend(
    remote_backend_url: RwSignal<String>,
    remote_backend_token: RwSignal<String>,
) -> impl IntoView {
    let status = RwSignal::new(None::<Result<String, String>>);
    let check = move |_: MouseEvent| {
        let args = CheckRemoteBackendArgs {
            remote_backend_url: remote_backend_url.get_untracked(),
            remote_backend_token: remote_backend_token.get_untracked(),
        };
        status.set(Some(Ok("Checking...".to_string())));
        leptos::task::spawn_local(async move {
            let checked = crate::common::invoke_json::<AppVersions>("check_remote_backend", &args)
                .await
                .map(|versions| {
                    let mut message = format!("Connected to swarmy {}", versions.swarmy);
                    if versions.swarmy != env!("CARGO_PKG_VERSION") {
                        message.push_str(&format!(
                            ", this one is {}: the views may fail until both are the same version",
                            env!("CARGO_PKG_VERSION")
                        ));
                    }
                    message
                });
            status.set(Some(checked));
        });
    };
    view! {
        <div class="col-span-1"></div>
        <div class="col-span-2 mb-4 flex items-center gap-3">
            <button
                type="button"
                class="btn btn-outline btn-sm shrink-0"
                prop:disabled=move || remote_backend_url.get().trim().is_empty()
                on:click=check
            >
                "Check the Remote Backend"
            </button>
            {move || match status.get() {
                Some(Ok(message)) => {
                    view! { <span class="text-sm/6 text-success">{message}</span> }.into_any()
                }
                Some(Err(err)) => {
                    view! { <span class="text-sm/6 text-error">{err}</span> }.into_any()
                }
                None => ().into_any(),
            }}
        </div>
        <div class="col-span-1"></div>
    }
}

/// The arguments of the check_remote_backend command.
#[derive(serde::Serialize)]
struct CheckRemoteBackendArgs {
    remote_backend_url: String,
    remote_backend_token: String,
}

/// The arguments of the pick_folder command.
#[derive(serde::Serialize)]
struct PickFolderArgs {
    directory: String,
    title: String,
}
