//! The theme, the zoom and the accessibility options of the interface: applied when the window
//! opens, from the settings saved on this computer, and changed in Config or, the zoom, with
//! Ctrl+=, Ctrl+- and Ctrl+0.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;
use wasm_bindgen::JsCast;

/// The appearance of the interface, provided by [`install_ui_settings`]: Config edits the
/// signals, the shortcuts change the zoom.
#[derive(Clone, Copy)]
pub struct Appearance {
    /// The theme id of [`THEMES`], [`SYSTEM_THEME`] follows `prefers_dark`.
    pub theme: RwSignal<String>,
    /// Whether the desktop prefers dark, updated when the preference changes.
    pub prefers_dark: RwSignal<bool>,
    /// The zoom in percent, as saved by the backend.
    pub zoom_percent: RwSignal<u32>,
    /// Whether the zoom is shown in the corner, for a moment after a change.
    pub zoom_toast: RwSignal<bool>,
}

/// The dark preference of the desktop, and its changes while the app runs.
fn system_prefers_dark() -> Option<web_sys::MediaQueryList> {
    window()
        .match_media("(prefers-color-scheme: dark)")
        .ok()
        .flatten()
}

/// The daisyUI `data-theme` of a theme id: the system theme is [`DEFAULT_THEME`] when the
/// desktop prefers dark and `light` otherwise.
fn resolve_theme(theme: &str, prefers_dark: bool) -> &str {
    match theme {
        SYSTEM_THEME if prefers_dark => DEFAULT_THEME,
        SYSTEM_THEME => "light",
        theme => theme,
    }
}

/// Sets an attribute of `<html>`, the ones styles.css reads; None removes it.
fn set_html_attribute(name: &str, value: Option<&str>) {
    let Some(html) = document().document_element() else {
        return;
    };
    let result = match value {
        Some(value) => html.set_attribute(name, value),
        None => html.remove_attribute(name),
    };
    if let Err(err) = result {
        leptos::logging::warn!("Unable to set {name}: {err:?}");
    }
}

/// Sets the accessibility attributes of `<html>` that styles.css reads: `data-a11y-motion`
/// (`system` or `on`), `data-a11y-links` and `data-a11y-focus` (`on`); off is no attribute, so
/// that their rules are not matched.
pub fn apply_accessibility(reduce_motion: &str, underline_links: bool, focus_outline: bool) {
    set_html_attribute(
        "data-a11y-motion",
        Some(reduce_motion).filter(|value| *value != "off"),
    );
    set_html_attribute("data-a11y-links", underline_links.then_some("on"));
    set_html_attribute("data-a11y-focus", focus_outline.then_some("on"));
}

#[derive(serde::Serialize)]
struct ZoomStepArgs {
    steps: i32,
}

/// Changes the zoom by steps, 0 resets it, and shows the new zoom for a moment.
pub fn zoom_step(steps: i32, appearance: Appearance) {
    spawn_local(async move {
        match crate::common::invoke_response("zoom_step", &ZoomStepArgs { steps }).await {
            Ok(response) => {
                let percent = response.message.parse().unwrap_or(default_zoom_percent());
                appearance.zoom_percent.set(percent);
                appearance.zoom_toast.set(true);
                set_timeout(
                    move || {
                        if appearance.zoom_percent.get_untracked() == percent {
                            appearance.zoom_toast.set(false);
                        }
                    },
                    std::time::Duration::from_millis(1200),
                );
            }
            Err(err) => leptos::logging::warn!("Unable to zoom: {err}"),
        }
    });
}

/// Provides [`Appearance`], applies the saved theme and accessibility options as soon as the
/// window opens, and installs the zoom shortcuts: Ctrl+= or Ctrl++ zooms in, Ctrl+- out, Ctrl+0
/// resets (Cmd on macOS). The zoom itself is applied by the backend when the window opens.
pub fn install_ui_settings() {
    let prefers_dark = system_prefers_dark();
    let appearance = Appearance {
        theme: RwSignal::new(default_theme()),
        prefers_dark: RwSignal::new(prefers_dark.as_ref().is_some_and(|query| query.matches())),
        zoom_percent: RwSignal::new(default_zoom_percent()),
        zoom_toast: RwSignal::new(false),
    };
    provide_context(appearance);
    Effect::new(move |_| {
        let theme = appearance.theme.get();
        set_html_attribute(
            "data-theme",
            Some(resolve_theme(&theme, appearance.prefers_dark.get())),
        );
    });
    if let Some(query) = prefers_dark {
        let on_change =
            wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::MediaQueryListEvent)>::new(
                move |event: web_sys::MediaQueryListEvent| {
                    appearance.prefers_dark.set(event.matches())
                },
            );
        query.set_onchange(Some(on_change.as_ref().unchecked_ref()));
        on_change.forget();
    }
    spawn_local(async move {
        match serde_wasm_bindgen::from_value::<AppSettings>(
            crate::invoke_without_args("get_ui_settings").await,
        ) {
            Ok(settings) => {
                appearance.theme.set(settings.theme);
                appearance.zoom_percent.set(settings.zoom_percent);
                apply_accessibility(
                    &settings.reduce_motion,
                    settings.underline_links,
                    settings.focus_outline,
                );
            }
            Err(err) => leptos::logging::warn!("Unable to read the UI settings: {err:?}"),
        }
    });
    window_event_listener(ev::keydown, move |event| {
        if !(event.ctrl_key() || event.meta_key()) || event.alt_key() {
            return;
        }
        // The key for the layouts where + needs Shift, the code for the numpad.
        let steps = match (event.key().as_str(), event.code().as_str()) {
            ("=" | "+", _) | (_, "NumpadAdd") => 1,
            ("-" | "_", _) | (_, "NumpadSubtract") => -1,
            ("0", _) | (_, "Numpad0") => 0,
            _ => return,
        };
        event.prevent_default();
        zoom_step(steps, appearance);
    });
}

/// The zoom for a moment after Ctrl+= or Ctrl+-, in the corner of the window.
#[component]
pub fn ZoomToastView() -> impl IntoView {
    let appearance = expect_context::<Appearance>();
    view! {
        <Show when=move || appearance.zoom_toast.get()>
            <div class="toast toast-end toast-bottom z-50">
                <div role="status" aria-live="polite" class="alert alert-info text-sm">
                    {move || format!("Zoom {}%", appearance.zoom_percent.get())}
                </div>
            </div>
        </Show>
    }
}
