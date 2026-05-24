//! Leptos view for app configuration.

use leptos::prelude::*;

#[component]
pub fn Config() -> impl IntoView {
    let (disable_parallel_scans, set_disable_parallel_scans) = signal(false);
    let scan_max_files = RwSignal::new(100000);
    let process_max_files = RwSignal::new(100000);
    let traverse_max_depth = RwSignal::new(8);
    let min_version = RwSignal::new(0);
    let max_version = RwSignal::new(0);
    view! {
        <div class="ml-4 mt-4">
            <h2 class="text-base/7 font-semibold text-white">"Swarmy Settings"</h2>

            <div class="mt-6 space-y-10 border-b border-white/10 pb-12 sm:space-y-0 sm:divide-y sm:divide-white/10 sm:border-t sm:border-t-white/10 sm:pb-0">
                <fieldset>
                    <legend class="sr-only">"Parallel Scanning"</legend>
                    <div class="sm:grid sm:grid-cols-3 sm:gap-4 sm:py-6">
                        <div aria-hidden="true" class="text-sm/6 font-semibold text-white">
                            <p class="text-sm/6 font-semibold text-white">"Parallel Scanning"</p>
                            <p class="text-sm/6 font-semibold text-white">
                                " Status: "
                                <span class=move || {
                                    if disable_parallel_scans.get() {
                                        "inline-flex items-center rounded-full bg-orange-400/10 px-1.5 py-0.5 text-xs font-medium text-orange-400"
                                    } else {
                                        "inline-flex items-center rounded-full bg-green-400/10 px-1.5 py-0.5 text-xs font-medium text-green-400"
                                    }
                                }>
                                    {move || {
                                        if disable_parallel_scans.get() {
                                            "Disabled"
                                        } else {
                                            "Enabled"
                                        }
                                    }}
                                </span>
                            </p>
                        </div>
                        <div class="mt-4 sm:col-span-2 sm:mt-0">
                            <div class="max-w-lg space-y-6">
                                <div class="flex gap-3">
                                    <div class="flex h-6 shrink-0 items-center">
                                        <div class="group grid size-4 grid-cols-1">
                                            <input
                                                id="paralellism"
                                                type="checkbox"
                                                name="paralellism"
                                                checked=move || disable_parallel_scans.get()
                                                on:click=move |_| {
                                                    set_disable_parallel_scans
                                                        .set(!disable_parallel_scans.get())
                                                }
                                                aria-describedby="comments-description"
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
                                        <label for="paralellism" class="font-medium text-white">
                                            "Disable Parallel Processing"
                                        </label>
                                        <p id="paralellism-description" class="text-gray-400">
                                            "Decreases CPU usage. Disable if you are on a laptop and you experience heating issues."
                                        </p>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </fieldset>
            </div>
            <div class="sm:grid sm:grid-cols-3 sm:items-start sm:gap-4 sm:py-6">
                <ConfigNumberInput
                    name="scan_max_files"
                    label="Maximum number of files to Scan"
                    description="Limit of the number of files to read, before validating if they are valid/supported."
                    value=scan_max_files
                />
                <ConfigNumberInput
                    name="process_max_files"
                    label="Maximum number of files to optimize"
                    description="Decreases the size of the optimized snapshot."
                    value=process_max_files
                />
                <ConfigNumberInput
                    name="traverse_max_depth"
                    label="Maximum Directories to traverse"
                    description="Limits how deep the scanner will go into sub-directories."
                    value=traverse_max_depth
                />
                <ConfigNumberInput
                    name="min_version"
                    label="Minimum protocol patch version to process"
                    description="Helps skip very old replays that are not relevant or supported. Set to 0 to disable filter."
                    value=min_version
                />
                <ConfigNumberInput
                    name="max_version"
                    label="Maximum protocol patch version to process"
                    description="Helps skip new replays that may be considered corrupt/unsupported. Set to 0 to disable filter."
                    value=max_version
                />
            </div>
        </div>
    }
}

#[component]
pub fn ConfigNumberInput(
    name: &'static str,
    label: &'static str,
    description: &'static str,
    value: RwSignal<i32>,
) -> impl IntoView {
    let (is_invalid_number, set_is_invalid_number) = signal(false);
    view! {
        <label class="block text-sm/6 font-medium text-white sm:pt-1.5">{label}</label>
        <div class="mt-2 sm:col-span-2 sm:mt-0">
            <input
                type="text"
                aria-invalid=move || if is_invalid_number.get() { "true" } else { "false" }
                aria-describedby=move || format!("{}-description", name)
                class=move || {
                    if is_invalid_number.get() {
                        "col-start-1 row-start-1 block w-full rounded-md bg-white/5 py-1.5 pr-10 pl-3 text-red-400 outline-1 -outline-offset-1 outline-red-500/50 placeholder:text-red-400/70 focus:outline-2 focus:-outline-offset-2 focus:outline-red-400 sm:pr-9 sm:text-sm/6"
                    } else {
                        "block w-full rounded-md bg-white/5 px-3 py-1.5 text-base text-white outline-1 -outline-offset-1 outline-white/10 placeholder:text-gray-500 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-500 sm:max-w-xs sm:text-sm/6"
                    }
                }
                on:input=move |ev| {
                    let input_value = event_target_value(&ev);
                    if let Ok(parsed_value) = input_value.parse::<i32>() {
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
                        description.to_string()
                    }
                }}
            </p>
        </div>
    }
}
