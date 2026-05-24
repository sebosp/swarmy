/// Common views.
use leptos::{leptos_dom::logging::console_log, prelude::*};
use phosphor_leptos::{Icon, IconWeight, X_CIRCLE};

pub fn text_input_tailwind_classes() -> Vec<&'static str> {
    vec![
        "block",
        "w-full",
        "rounded-md",
        "px-3",
        "py-1.5",
        "outline-1",
        "-outline-offset-1",
        "sm:text-sm/6",
        "bg-white/5",
        "text-gray-300",
        "outline-white/10",
        "placeholder:text-gray-500",
        "focus:outline-2",
        "focus:-outline-offset-2",
        "focus:outline-indigo-500",
        "disabled:cursor-not-allowed",
        "disabled:bg-gray-50",
        "disabled:text-gray-500",
        "disabled:outline-gray-200",
    ]
}

#[component]
pub fn DisplayBackendStatus(
    backend_response: ReadSignal<swarmy_tauri_common::ApiResponse>,
) -> impl IntoView {
    let error_lines = move || {
        backend_response
            .get()
            .message
            .replace("\\n", "\n")
            .replace("\\t", "\u{2007}\u{2007}\u{2007}\u{2007}")
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<String>>()
    };
    view! {
        <Show when=move || {
            !backend_response.get().meta.success && backend_response.get().meta.is_complete
        }>
            <div class="border-l-4 mt-1 p-2 border-red-500 bg-red-500/10">
                <div class="flex">
                    <div class="shrink-0 text-red-500 size-5">
                        <Icon icon=X_CIRCLE weight=IconWeight::Bold prop:class="stroke-current" />
                    </div>
                    <div class="ml-3">
                        <p class="text-ellipsis overflow-hidden text-sm text-red-300">
                            <p class="text-sm text-red-300">"Directory is optimized."</p>
                            <For
                                each=move || error_lines()
                                key=|line| line.clone()
                                children=move |line: String| {
                                    console_log(&format!("Error line: {:?}", line));
                                    view! { <span>{move || line.clone()}<br /></span> }
                                }
                            />
                        </p>
                    </div>
                </div>
            </div>
        </Show>
    }
}
