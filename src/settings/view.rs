//! Leptos view for app configuration.

use leptos::{ev::MouseEvent, prelude::*};
use phosphor_leptos::{Icon, IconWeight, QUESTION};
use swarmy_common::*;

use crate::{DisplayBackendStatus, settings::actions::trigger_set_current_app_config};

#[component]
pub fn Config() -> impl IntoView {
    let (_app_settings, set_app_settings) = signal(AppSettings::default());
    let replay_path = RwSignal::new("".to_string());
    let cache_path = RwSignal::new("".to_string());
    let disable_parallelism = RwSignal::new(DEFAULT_DISABLE_PARALLELISM);
    let traverse_max_depth = RwSignal::new(DEFAULT_TRAVERSE_MAX_DEPTH);
    let scan_max_files = RwSignal::new(DEFAULT_SCAN_MAX_FILES);
    let process_max_files = RwSignal::new(DEFAULT_PROCESS_MAX_FILES);
    let min_version = RwSignal::new(DEFAULT_MIN_VERSION);
    let max_version = RwSignal::new(DEFAULT_MAX_VERSION);
    let (backend_response, set_backend_response) = signal(ApiResponse {
        meta: ResponseMeta::incomplete(),
        message: String::new(),
    });
    crate::settings::with_fetch_current_app_config(
        set_app_settings,
        move |settings: AppSettings| {
            *replay_path.write() = settings.replay_path.clone();
            *cache_path.write() = settings.cache_path.clone();
            *disable_parallelism.write() = settings.disable_parallelism;
            *traverse_max_depth.write() = settings.traverse_max_depth;
            *scan_max_files.write() = settings.scan_max_files;
            *process_max_files.write() = settings.process_max_files;
            *min_version.write() = settings.min_version;
            *max_version.write() = settings.max_version;
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
            </div>
            <div class="grid grid-cols-4 sm:items-start sm:gap-4 sm:py-6">
                <ConfigStringInput
                    name="cache_path"
                    label="Map Caches Path"
                    description="The location to store the replay caches, containing map data, tournament logos, etc"
                    value=cache_path
                />
                <ConfigStringInput
                    name="replay_path"
                    label="Replay Path"
                    description="The parent path for the replay collection to analyse."
                    value=replay_path
                />
                <ConfigNumberInput
                    name="scan_max_files"
                    label="Max number of files to Scan"
                    description="Limit of the number of files to read, before validating if they are valid/supported."
                    value=scan_max_files
                />
                <ConfigNumberInput
                    name="process_max_files"
                    label="Maxnumber of files to optimize"
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
                                cache_path.get(),
                                replay_path.get(),
                                disable_parallelism.get(),
                                traverse_max_depth.get(),
                                scan_max_files.get(),
                                process_max_files.get(),
                                min_version.get(),
                                max_version.get(),
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
) -> impl IntoView {
    view! {
        <div class="mt-2 col-span-1 sm:mt-0">
            <label class="block text-sm/6 font-medium text-white sm:pt-1.5">{label}</label>
        </div>
        <div class="mt-2 col-span-2 sm:mt-0">
            <input
                type="text"
                aria-invalid=move || if value.get().is_empty() { "true" } else { "false" }
                aria-describedby=move || format!("{}-description", name)
                class=move || {
                    if value.get().is_empty() {
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
            <p
                id=move || format!("{}-description", name)
                class=move || {
                    if value.get().is_empty() {
                        "mt-2 text-sm/6 text-red-400"
                    } else {
                        "mt-2 text-sm/6 text-gray-400"
                    }
                }
            >
                {move || {
                    if value.get().is_empty() {
                        "Please enter a directory".to_string()
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
