# Development

## Dependencies
Tauri requirement plus tailwindcss and daisyui
```
$ curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/download/v4.2.4/tailwindcss-linux-x64
$ vim README.md
$ chmod a+x tailwindcss-linux-x64
$ mv tailwindcss-linux-x64 tailwindcss
$ mv tailwindcss ~/local/bin/
$ curl -sLO https://github.com/dobicinaitis/tailwind-cli-extra/releases/download/v2.8.3/tailwindcss-extra-linux-x64
$ chmod a+x tailwindcss-extra-linux-x64
$ mv tailwindcss-extra-linux-x64 tailwindcss-extra
$ mv tailwindcss-extra ~/local/bin/
$ npm install daisyui
```

There's a bit of a strange bug when installing tailwindcss / tailwindcss-extra in Trunk.toml, for some reason installing one tries to
I think I had to use something like
```toml
[tool]
tailwindcss = "2.7.0" # This is actually tailwindcss-extra version
```
Then `trunk build` downloads the tailwindcss-extra plugin version 2.7.0 :shrug:
And then **sometimes** it works?

### Charts
The charts are drawn with [ApexCharts](https://apexcharts.com) 5.3.6 the way the
[rust-ui charts](https://leptos.rust-ui.com) do it: the Leptos components in
`src/components/charts.rs` write the data as `data-chart-*` attributes and
`public/charts/chart_init.js` (adapted from rust-ui, MIT) renders them. `apexcharts.5.3.6.min.js` is
vendored in `public/charts`, ApexCharts 5 is dual licensed, its community license
(`public/charts/APEXCHARTS-LICENSE.md`) is free for individuals and organizations under 2M USD of
annual revenue, with attribution.

### Sortable tables
`public/table_sort.js` sorts any table by clicking on the headers of its `<thead>`, a table sorted by
the backend has `data-server-sort`, a header that should not sort has `data-no-sort`.

## Walkthrough video and screenshots

The README video and the screenshots are recorded with `scripts/walkthrough/`, see its README.

## Releases
`.github/workflows/ci.yaml` checks the formatting, runs the tests and checks the frontend.
`.github/workflows/release.yaml` builds the bundles of Linux (x86_64, aarch64), Windows (x86_64) and
macOS (Apple Silicon) for a `v*` tag, the platforms of the rerun-cli releases, with the sidecars
downloaded by `scripts/download-sidecars.sh`, so the swarmy-bevy release must exist first. The
updater artifacts are signed with the `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets, without them the bundles have no updater
artifacts.

## Rerun web viewer

The timeline must be switched to the "game_timeline" in the dropdown.
The gamespeed seems to be 22 FPS for "faster" game speed. This kindof matches replays.

![Screenshot for 2025EWC Grand Finals](https://github.com/user-attachments/assets/bf6ca086-db19-4054-94d9-0f1ae9a74b11)

### Example recorded replays for the web

Rerun 0.26.0:

[2025EWCClassicSerralGame8GrandFinals](https://sebosp.github.io/swarmy/public/0.26.0/index.html)
