// The browser version of swarmy: the frontend calls the backend through window.__TAURI__, here it
// goes to the routes of `swarmy --serve` instead. In the app, Tauri has defined it and only
// window.swarmyWeb is added. See src-tauri/src/web.rs and docs/Installation.md#browser-version.
(function () {
  const isWeb = !window.__TAURI__ && !window.__TAURI_INTERNALS__;
  const KEY_STORE = "swarmy-admin-key";
  const SETTINGS_STORE = "swarmy-web-settings";
  const EVOLUTIONS_STORE = "swarmy-web-evolutions";
  const ZOOM = { default: 100 };
  const session = { admin: !isWeb };

  const store = {
    get(name) {
      try {
        return JSON.parse(window.localStorage.getItem(name));
      } catch (_err) {
        return null;
      }
    },
    set(name, value) {
      try {
        window.localStorage.setItem(name, JSON.stringify(value));
      } catch (_err) {
        // Private windows may have no storage, the settings then last until the page is closed.
      }
    },
  };

  function adminKey() {
    try {
      return window.sessionStorage.getItem(KEY_STORE) || "";
    } catch (_err) {
      return "";
    }
  }

  function headers() {
    const result = { "Content-Type": "application/json" };
    const key = adminKey();
    if (key) {
      result["X-Swarmy-Admin-Key"] = key;
    }
    return result;
  }

  function response(message, success = true) {
    return {
      meta: { success, duration_ms: 0, is_complete: true },
      message: typeof message === "string" ? message : JSON.stringify(message),
    };
  }

  function desktopOnly(what) {
    return async () => response(`${what} is only available in the desktop app of swarmy`, false);
  }

  // A failure is an answer the views can show: a rejected promise would stop them loading forever.
  async function backend(command, args) {
    try {
      const reply = await fetch(`/api/${command}`, {
        method: "POST",
        headers: headers(),
        body: JSON.stringify(args || {}),
      });
      const text = await reply.text();
      let body;
      try {
        body = JSON.parse(text);
      } catch (_err) {
        body = text;
      }
      if (reply.ok) {
        return body;
      }
      return response(body && body.message ? body.message : `${command}: ${reply.status}`, false);
    } catch (err) {
      return response(`The server does not answer: ${err}`, false);
    }
  }

  async function serverSettings() {
    const reply = await fetch("/api/web/settings", { headers: headers() });
    const settings = await reply.json();
    return Object.assign(settings, store.get(SETTINGS_STORE) || {}, {
      remote_backend_url: window.location.origin,
    });
  }

  // The browser zooms the page itself (Ctrl+= and Ctrl+-): the zoom of the app is not used here.
  function applyZoom(_percent) {}

  // The appearance is of this browser, the rest of the settings are of the server.
  function saveAppearance(settings) {
    const { theme, zoom_percent, reduce_motion, underline_links, focus_outline, max_table_rows } =
      settings;
    store.set(SETTINGS_STORE, {
      theme,
      zoom_percent,
      reduce_motion,
      underline_links,
      focus_outline,
      max_table_rows,
    });
  }

  async function exportReport(args) {
    const rendered = await backend("render_replay_report", args);
    if (!rendered.meta || !rendered.meta.success) {
      return rendered;
    }
    const [html, mapTitle] = JSON.parse(rendered.message);
    const link = document.createElement("a");
    link.href = URL.createObjectURL(new Blob([html], { type: "text/html" }));
    link.download = `${mapTitle}.html`;
    link.click();
    URL.revokeObjectURL(link.href);
    return response(link.download);
  }

  // The commands of the app that have no route in the server. A test of src-tauri/src/web.rs checks
  // that every command of the frontend is here or in the server.
  const localCommands = {
    get_current_app_config: serverSettings,
    get_ui_settings: serverSettings,
    set_current_app_config: async (args) => {
      saveAppearance(args.app_settings);
      applyZoom(args.app_settings.zoom_percent);
      return null;
    },
    zoom_step: async () => response(String(ZOOM.default)),
    check_remote_backend: async () => response(await (await fetch("/api/version")).json()),
    get_app_versions: async () => response(await (await fetch("/api/version")).json()),
    open_external_url: async (args) => {
      window.open(args.url, "_blank", "noopener");
      return response("");
    },
    copy_path_to_clipboard: async (args) => {
      await navigator.clipboard.writeText(args.path || "");
      return response("");
    },
    list_saved_evolutions: async () => response(store.get(EVOLUTIONS_STORE) || []),
    save_evolution: async (args) => {
      const others = (store.get(EVOLUTIONS_STORE) || []).filter(
        (saved) => saved.name !== args.saved.name,
      );
      if (args.saved.series && args.saved.series.length > 0) {
        others.push(args.saved);
      }
      store.set(EVOLUTIONS_STORE, others);
      return response(others);
    },
    export_replay_report: exportReport,
    pick_folder: desktopOnly("Choosing a folder"),
    open_folder: desktopOnly("Opening a folder"),
    connect_swarmy_rerun: desktopOnly("The Rerun viewer"),
    exec_swarmy_bevy_map_caches: desktopOnly("swarmy-bevy"),
    exec_swarmy_bevy_replay: desktopOnly("swarmy-bevy"),
    exec_swarmy_bevy_siege_spots: desktopOnly("swarmy-bevy"),
    exec_swarmy_bevy_map_layers: desktopOnly("swarmy-bevy"),
    query_replay_lesson: desktopOnly("The lesson of a new replay"),
  };

  async function refreshSession() {
    try {
      const reply = await fetch("/api/web/session", { headers: headers() });
      const body = await reply.json();
      session.admin = Boolean(body.admin);
    } catch (_err) {
      session.admin = false;
    }
    return session.admin;
  }

  window.swarmyWeb = {
    isWeb: () => isWeb,
    // Resolves to whether this browser is the administrator of the server.
    refreshSession: () => (isWeb ? refreshSession() : Promise.resolve(true)),
    isAdmin: () => session.admin,
    // The key lasts until the tab is closed, it is not kept on the computer.
    setAdminKey: (key) => {
      try {
        if (key) {
          window.sessionStorage.setItem(KEY_STORE, key);
        } else {
          window.sessionStorage.removeItem(KEY_STORE);
        }
      } catch (_err) {
        // Without storage the key cannot be used.
      }
      return refreshSession();
    },
  };

  if (!isWeb) {
    return;
  }

  // The buttons of the app only (Rerun, swarmy-bevy) are dimmed by styles.css and their clicks
  // stop here, before the handlers of the views; their titles say why.
  document.documentElement.dataset.web = "on";
  window.addEventListener(
    "click",
    (event) => {
      if (event.target.closest?.(".native-only")) {
        event.preventDefault();
        event.stopImmediatePropagation();
      }
    },
    true,
  );

  window.__TAURI__ = {
    core: {
      invoke: (command, args) => {
        const local = localCommands[command];
        return local ? local(args || {}) : backend(command, args);
      },
    },
    event: {
      listen: async (event, handler) => {
        const source = new EventSource("/api/events");
        source.addEventListener(event, (message) => {
          handler({ event, payload: JSON.parse(message.data) });
        });
        return () => source.close();
      },
    },
  };

})();
