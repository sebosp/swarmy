# Swarmy - Starcrat II - Replay collection organizer / analysis

This repo is a frontend to bridge different tools to work together on data analysis.

# Tooling scopes:
- Frontend-backend: tauri with leptos (this repo)
- Decoding/batch-transformation into Apache Arrow via [s2protocol-rs](https://www.github.com/sebosp/s2protocol-rs)
- Map T3 Terrain Height navigation with [swarmy-bevy](https://www.github.com/sebosp/swarmy-bevy)
- Visualization Replay with [rerun](https://github.com/rerun-io/rerun) 
- Pola.rs/DuckDB analysis via [s2-polars-data-analysis](https://github.com/sebosp/s2-polars-data-analysis), these queries will become part of the frontend.

![Starcraft 2 Replay in Rerun](https://user-images.githubusercontent.com/873436/231281746-40fde3f1-fec6-49fe-8cf1-5fbd197589b7.png)

## Web UI Setup

The timeline must be switched to the "game_timeline" in the dropdown.
The gamespeed seems to be 22 FPS for "faster" game speed. This kindof matches replays.

![Screenshot for 2025EWC Grand Finals](https://github.com/user-attachments/assets/bf6ca086-db19-4054-94d9-0f1ae9a74b11)

## Example Recorded/Processed Replays for web.

Rerun 0.26.0:

[2025EWCClassicSerralGame8GrandFinals](https://sebosp.github.io/swarmy/public/0.26.0/index.html)

## Initial Focus
- Colored fog of war, which player has scouted which area.
- Hint on next region of interest: Since this is for replays, we know when important events are going to happen.
  we can guide the caster to position the window to specific regions where in the next X gameloops, an important
  activity is happening, for example massive damage or massive death on an area, i.e. splash from widow mines, disruptors, tanks.
  This could be similar to how a warning is seen for a Nydus or Nukes.
- Different visualizations on events, for example, we could draw the amount of deaths per regions of the map, like piling up bodies
  and showing what regions of a map have been more active than others.
- We can add many data visualizations.

# TODO

## Tauri

- Initially I thought of making maps unique based on their cache_handles
  to compensate for the fact that maps have versions and I wanted to account for that.
  however, it seems that when on tournaments, additional images are added to maps
  and that already changes the caches, maybe I should locate the t3HeightMap and MapInfo
  and use them for deduplication, tho visual element changes wouldn't be account for?
- Currently only one version can be running at a time because of the settings.json that is written on "scan" tab.
- "caches" should be stored in a global location to avoid double downloading for different snapshots.
- In the current queries, I only use one-player, however it should be easy to add multi-user queries,
  either for 2v2 or for specific player vs specific player.

## Rerun
- Some units generate too many actions to be displayed in the current way as separate events in rerun,  for example a Lurker may create a temporary InvisibleTargetUnit for every cycle the spines shoots, this needs to be somehow shortened.
- Sometimes the state management in s2protocol-rs may lose sync, at this point rerun is very useful for debugging and seeing how the state transists through events and what abilities may be missing.
- Some events seem a bit redundanct, i.e. a Larva "dies" to become a "drone" or an "Adept" has an "AdeptPhaseShift" temporary unit that "dies" and this also causes a lot of events.
- The Target Position (arrows signaling where each unit is pointing to) are not shown anymore as it just creates way too many objects in the map and makes it impossible to use.

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
$ cd ~/swarmy-tauri;
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
