//! The theme and the zoom of the interface: applied when the window opens, from the settings
//! saved on this computer, and changed with Ctrl+=, Ctrl+- and Ctrl+0 or in Config.

use crate::invoke;
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;
use wasm_bindgen::JsCast;

/// The zoom shown for a moment after a change, the percent.
#[derive(Clone, Copy)]
pub struct ZoomToast(pub RwSignal<Option<u32>>);

/// Whether the theme follows the light or dark preference of the desktop, see [`apply_theme`].
static FOLLOWS_SYSTEM: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The dark preference of the desktop, and its changes while the app runs.
fn system_prefers_dark() -> Option<web_sys::MediaQueryList> {
    window()
        .match_media("(prefers-color-scheme: dark)")
        .ok()
        .flatten()
}

/// Sets the theme of the document, the daisyUI `data-theme` of `<html>`. The system theme picks
/// [`DEFAULT_THEME`] when the desktop prefers dark and `light` otherwise, and follows the changes
/// of the preference.
pub fn apply_theme(theme: &str) {
    let follows_system = theme == SYSTEM_THEME || theme.is_empty();
    FOLLOWS_SYSTEM.store(follows_system, std::sync::atomic::Ordering::Relaxed);
    let theme = if follows_system {
        match system_prefers_dark() {
            Some(query) if !query.matches() => "light".to_string(),
            _ => DEFAULT_THEME.to_string(),
        }
    } else {
        theme.to_string()
    };
    let Some(html) = document().document_element() else {
        return;
    };
    if let Err(err) = html.set_attribute("data-theme", &theme) {
        leptos::logging::warn!("Unable to apply the theme {theme}: {err:?}");
    }
}

/// Sets the accessibility attributes of `<html>` that styles.css reads: `data-a11y-motion`
/// (`system`, `on`, `off`), `data-a11y-links` and `data-a11y-focus` (`on`, `off`).
pub fn apply_accessibility(reduce_motion: &str, underline_links: bool, focus_outline: bool) {
    let Some(html) = document().document_element() else {
        return;
    };
    let on_off = |flag: bool| if flag { "on" } else { "off" };
    for (name, value) in [
        ("data-a11y-motion", reduce_motion),
        ("data-a11y-links", on_off(underline_links)),
        ("data-a11y-focus", on_off(focus_outline)),
    ] {
        if let Err(err) = html.set_attribute(name, value) {
            leptos::logging::warn!("Unable to set {name}: {err:?}");
        }
    }
}

/// Follows the light or dark preference of the desktop while the system theme is picked.
fn follow_system_theme() {
    let Some(query) = system_prefers_dark() else {
        return;
    };
    let on_change = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(|| {
        if FOLLOWS_SYSTEM.load(std::sync::atomic::Ordering::Relaxed) {
            apply_theme(SYSTEM_THEME);
        }
    });
    query.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget();
}

#[derive(serde::Serialize)]
struct ZoomStepArgs {
    steps: i32,
}

/// Changes the zoom by steps, 0 resets it, and shows the new zoom for a moment.
pub fn zoom_step(steps: i32, toast: ZoomToast) {
    spawn_local(async move {
        match crate::common::invoke_response("zoom_step", &ZoomStepArgs { steps }).await {
            Ok(response) => {
                let percent = response.message.parse().unwrap_or(100);
                toast.0.set(Some(percent));
                set_timeout(
                    move || {
                        if toast.0.get_untracked() == Some(percent) {
                            toast.0.set(None);
                        }
                    },
                    std::time::Duration::from_millis(1200),
                );
            }
            Err(err) => leptos::logging::warn!("Unable to zoom: {err}"),
        }
    });
}

/// Applies the saved theme as soon as the window opens, and the zoom shortcuts: Ctrl+= or
/// Ctrl++ zooms in, Ctrl+- out, Ctrl+0 resets (Cmd on macOS). The zoom itself is applied by the
/// backend when the window opens.
pub fn install_ui_settings() -> ZoomToast {
    let toast = ZoomToast(RwSignal::new(None));
    provide_context(toast);
    spawn_local(async move {
        match serde_wasm_bindgen::from_value::<UiSettings>(
            invoke("get_ui_settings", wasm_bindgen::JsValue::UNDEFINED).await,
        ) {
            Ok(settings) => {
                apply_theme(&settings.theme);
                apply_accessibility(
                    &settings.reduce_motion,
                    settings.underline_links,
                    settings.focus_outline,
                );
            }
            Err(err) => leptos::logging::warn!("Unable to read the UI settings: {err:?}"),
        }
    });
    follow_system_theme();
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
        zoom_step(steps, toast);
    });
    toast
}

/// The zoom for a moment after Ctrl+= or Ctrl+-, in the corner of the window.
#[component]
pub fn ZoomToastView() -> impl IntoView {
    let toast = use_context::<ZoomToast>().expect("the zoom toast is provided by Main");
    view! {
        <Show when=move || toast.0.get().is_some()>
            <div
                role="status"
                aria-live="polite"
                class="fixed bottom-4 right-4 z-50 rounded-md bg-neutral px-3 py-2 text-sm text-neutral-content shadow-lg"
            >
                {move || format!("Zoom {}%", toast.0.get().unwrap_or(100))}
            </div>
        </Show>
    }
}
