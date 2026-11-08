//! Whether this is the browser version of swarmy and its administrator, see
//! `public/web_bridge.js` and `src-tauri/src/web.rs`. The views of the administrator (Scan,
//! Optimize, the personal notes) are not shown to the others, and the server refuses them anyway.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = swarmyWeb, js_name = isWeb)]
    pub fn is_web() -> bool;
    #[wasm_bindgen(js_namespace = swarmyWeb, js_name = refreshSession)]
    async fn refresh_session() -> JsValue;
    #[wasm_bindgen(js_namespace = swarmyWeb, js_name = setAdminKey)]
    async fn set_admin_key(key: &str) -> JsValue;
}

/// Whether this window can use the views of the administrator: always in the app.
#[derive(Clone, Copy)]
pub struct WebSession {
    pub admin: RwSignal<bool>,
}

/// Provides [`WebSession`] and asks the server about it.
pub fn provide_web_session() {
    let session = WebSession {
        admin: RwSignal::new(!is_web()),
    };
    provide_context(session);
    if is_web() {
        spawn_local(async move {
            session.admin.set(refresh_session().await.is_truthy());
        });
    }
}

/// The session of the window, a signal to read in the views.
pub fn use_web_session() -> WebSession {
    expect_context::<WebSession>()
}

/// Asks for the administrator key in the Config of the browser version. The server keeps only its
/// sha512, the browser keeps the key until the tab is closed.
#[component]
pub fn AdminKeyInput() -> impl IntoView {
    let session = use_web_session();
    let key = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let apply = move |value: String| {
        spawn_local(async move {
            let admin = set_admin_key(&value).await.is_truthy();
            session.admin.set(admin);
            status.set(
                match (admin, value.is_empty()) {
                    (true, _) => "Administrator: Scan and Optimize are in the menu",
                    (false, true) => "Signed out",
                    (false, false) => "That key is not the one of this server",
                }
                .to_string(),
            );
        });
    };
    // A row of the grid of Config: the label, the field, its help.
    view! {
        <div class="col-span-1">
            <label class="block text-sm/6 font-medium text-base-content sm:pt-1.5" for="swarmy-admin-key">
                "Administrator key"
            </label>
        </div>
        <div class="col-span-2 flex flex-col gap-2 max-w-md">
            <form
                class="flex gap-2"
                on:submit=move |ev| {
                    ev.prevent_default();
                    apply(key.get_untracked());
                    key.set(String::new());
                }
            >
                <input
                    id="swarmy-admin-key"
                    type="password"
                    autocomplete="current-password"
                    class="input input-sm grow bg-base-200 text-base-content border border-base-content/20"
                    prop:value=move || key.get()
                    on:input=move |ev| key.set(event_target_value(&ev))
                />
                <button type="submit" class="btn btn-primary btn-sm">"Sign in"</button>
                <button
                    type="button"
                    class="btn btn-ghost btn-sm"
                    on:click=move |_| apply(String::new())
                >
                    "Sign out"
                </button>
            </form>
            <span class="text-sm" role="status">{move || status.get()}</span>
        </div>
        <div class="col-span-1 text-primary">
            <span title="The key of the administrator of this server: Scan, Optimize, the personal notes and the Battle.net API need it. It lasts until the tab is closed.">
                <phosphor_leptos::Icon icon=phosphor_leptos::QUESTION weight=phosphor_leptos::IconWeight::Fill prop:class="stroke-current" />
            </span>
        </div>
    }
}
