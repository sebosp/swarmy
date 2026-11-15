//! Show Polars Debug Information (Config): a line above a table, a field or a panel names the
//! query behind it and opens it in the repository, i.e.
//! `https://github.com/sebosp/swarmy/blob/<commit>/src-tauri/src/map_details/recent_games.rs#L7`,
//! to find and read the query (its README is next to it). The queries come with the answers
//! (`ResponseMeta::sources`, recorded by [`crate::invoke_response`]); the commit is the one the
//! backend was built from (`AppVersions::git_sha`).

use crate::invoke_json;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{BUG, GITHUB_LOGO, Icon, IconWeight};
use std::collections::HashMap;
use swarmy_common::*;

/// The repository of swarmy, the links point into it.
pub const REPOSITORY_URL: &str = "https://github.com/sebosp/swarmy";

/// The branch the links point to when the backend was built from uncommitted changes: a commit
/// does not have them yet.
pub const DEVELOPMENT_BRANCH: &str = "develop";

/// The debug information of the window, provided by [`provide_debug_context`].
#[derive(Clone, Copy)]
pub struct DebugContext {
    /// The flag of Config, applied as it is toggled.
    pub enabled: RwSignal<bool>,
    /// The queries behind the answers received so far, by their id (`map_details::matchups`).
    pub sources: RwSignal<HashMap<String, QuerySource>>,
    /// The versions of the backend, for the commit of the links.
    pub versions: RwSignal<Option<AppVersions>>,
}

thread_local! {
    /// The context of the window, also for the code that runs outside the component tree
    /// (`invoke_response` in a spawned task, the event handlers): Leptos' `use_context` sees
    /// nothing there.
    static CONTEXT: std::cell::Cell<Option<DebugContext>> = const { std::cell::Cell::new(None) };
}

pub fn provide_debug_context() {
    let context = DebugContext {
        enabled: RwSignal::new(false),
        sources: RwSignal::new(HashMap::new()),
        versions: RwSignal::new(None),
    };
    CONTEXT.set(Some(context));
    provide_context(context);
    // The commit of the backend, once the flag is on.
    Effect::new(move |_| {
        if context.enabled.get() && context.versions.get_untracked().is_none() {
            spawn_local(async move {
                if let Ok(versions) = invoke_json::<AppVersions>("get_app_versions", &()).await {
                    context.versions.set(Some(versions));
                }
            });
        }
    });
}

/// Turns Show Polars Debug Information on or off and saves it with the settings of this
/// computer (the browser version keeps it in the browser).
pub fn set_debug_enabled(enabled: bool) {
    let debug = use_debug_context();
    debug.enabled.set(enabled);
    spawn_local(async move {
        let Ok(mut settings) = serde_wasm_bindgen::from_value::<AppSettings>(
            crate::invoke_without_args("get_ui_settings").await,
        ) else {
            return;
        };
        settings.show_polars_debug = enabled;
        #[derive(serde::Serialize)]
        struct Args {
            app_settings: AppSettings,
        }
        if let Err(err) = crate::invoke_response(
            "set_current_app_config",
            &Args {
                app_settings: settings,
            },
        )
        .await
        {
            leptos::logging::warn!("Unable to save Show Polars Debug Information: {err}");
        }
    });
}

pub fn use_debug_context() -> DebugContext {
    CONTEXT
        .get()
        .expect("provide_debug_context is called by Main before the views")
}

/// Keeps the queries of an answer, called by `invoke_response` for every answer.
pub fn record_sources(sources: &[QuerySource]) {
    if sources.is_empty() {
        return;
    }
    if let Some(context) = CONTEXT.get() {
        context.sources.update(|known| {
            for source in sources {
                known.insert(source.id.clone(), source.clone());
            }
        });
    }
}

/// The address of a query file in the repository: at the commit of the backend, or on the
/// development branch when the backend was built from uncommitted changes.
pub fn source_url(source: &QuerySource, versions: Option<&AppVersions>) -> String {
    let revision = match versions {
        Some(versions) if !versions.git_dirty && versions.git_sha.len() >= 7 => {
            versions.git_sha.as_str()
        }
        _ => DEVELOPMENT_BRANCH,
    };
    format!(
        "{REPOSITORY_URL}/blob/{revision}/{}#L{}",
        source.file, source.line
    )
}

/// Wraps a table, a field or a panel: with Show Polars Debug Information on, a line above it
/// names the query `query` (the id of its `META`, i.e. `map_details::recent_games`) and its file
/// in `<code>`, with a button that copies the address of the file in the repository. A line, not
/// a button over the content: it never covers a sort arrow or a header. `view_source` is the
/// file and line of the view that shows it (`source!()`), in the title of the line.
#[component]
pub fn Debuggable(
    /// The id of the query, `<view>::<query>`; empty for a panel that is not a query (the
    /// children are shown alone).
    query: &'static str,
    /// The file and line of the view, [`source!`].
    #[prop(optional)]
    view_source: &'static str,
    children: Children,
) -> impl IntoView {
    let debug = use_debug_context();
    let source = Memo::new(move |_| debug.sources.with(|sources| sources.get(query).cloned()));
    let url = move || {
        source
            .get()
            .map(|source| source_url(&source, debug.versions.get().as_ref()))
    };
    // In the default browser (the app) or a new tab (the browser version); the backend only
    // opens the links of the project.
    let open = move |_| {
        let Some(url) = url() else {
            return;
        };
        #[derive(serde::Serialize)]
        struct Args {
            url: String,
        }
        spawn_local(async move {
            if let Err(err) = crate::invoke_response("open_external_url", &Args { url }).await {
                leptos::logging::warn!("Unable to open the query on GitHub: {err}");
            }
        });
    };
    view! {
        <div>
            <Show when=move || debug.enabled.get() && !query.is_empty()>
                <div
                    class="flex flex-wrap items-center gap-2 mb-1 text-xs text-warning"
                    title=move || {
                        let view_line = if view_source.is_empty() {
                            String::new()
                        } else {
                            format!("\nShown by {view_source}")
                        };
                        format!("Polars debug: the query that computes this, its README and SQL are next to its file{view_line}")
                    }
                >
                    <Icon icon=BUG weight=IconWeight::Fill />
                    <code class="font-mono">{query}</code>
                    {move || match source.get() {
                        Some(source) => view! {
                            <code class="font-mono text-faint">{format!("{}#L{}", source.file, source.line)}</code>
                            <button
                                type="button"
                                class="btn btn-xs btn-ghost btn-square text-warning"
                                title="Open in GitHub"
                                on:click=open
                            >
                                <Icon icon=GITHUB_LOGO weight=IconWeight::Bold />
                                <span class="sr-only">"Open in GitHub"</span>
                            </button>
                        }
                            .into_any(),
                        None => view! {
                            <span class="text-faint">"the answer of the backend did not name this query"</span>
                        }
                            .into_any(),
                    }}
                </div>
            </Show>
            {children()}
        </div>
    }
}

/// The file and line of the view that uses it, for the `view_source` of [`Debuggable`]:
/// `src/map_details/view.rs#L180`.
#[macro_export]
macro_rules! source {
    () => {
        concat!(file!(), "#L", line!())
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> QuerySource {
        QuerySource {
            id: "map_details::matchups".to_string(),
            file: "src-tauri/src/map_details/matchups.rs".to_string(),
            line: 7,
        }
    }

    #[test]
    fn a_clean_build_links_to_its_commit() {
        let versions = AppVersions {
            git_sha: "0123456789abcdef".to_string(),
            git_dirty: false,
            ..Default::default()
        };
        assert_eq!(
            source_url(&source(), Some(&versions)),
            "https://github.com/sebosp/swarmy/blob/0123456789abcdef/src-tauri/src/map_details/matchups.rs#L7"
        );
    }

    #[test]
    fn a_dirty_or_unknown_build_links_to_the_development_branch() {
        let dirty = AppVersions {
            git_sha: "0123456789abcdef".to_string(),
            git_dirty: true,
            ..Default::default()
        };
        let expected = "https://github.com/sebosp/swarmy/blob/develop/src-tauri/src/map_details/matchups.rs#L7";
        assert_eq!(source_url(&source(), Some(&dirty)), expected);
        assert_eq!(source_url(&source(), None), expected);
    }
}
