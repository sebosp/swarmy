# Roadmap

## Initial focus
- Colored fog of war, which player has scouted which area.
- Hint on next region of interest: Since this is for replays, we know when important events are going to happen.
  we can guide the caster to position the window to specific regions where in the next X gameloops, an important
  activity is happening, for example massive damage or massive death on an area, i.e. splash from widow mines, disruptors, tanks.
  This could be similar to how a warning is seen for a Nydus or Nukes.
- Different visualizations on events, for example, we could draw the amount of deaths per regions of the map, like piling up bodies
  and showing what regions of a map have been more active than others.
- We can add many data visualizations.

## Done

- "caches" should be stored in a global location to avoid double downloading for different snapshots.
- cache_handles are analized to be able to process the MapInfo uniqueness. However the version of the map hasn't been found on the MapInfo contents.

## To do

- Currently only one version can be running at a time because of the settings.json that is written on "scan" tab.
- In the current queries, I only use one-player, however it should be easy to add multi-user queries,
  either for 2v2 or for specific player vs specific player.
- [x] Watch the replay directories: Config has a watcher of the replay path (the `notify`
  crate), a banner tells how many replays are not in the snapshot and runs Optimize.

## Ideas for casters

Inspired by what the Dota 2 broadcasts use: stat sites with matchup win rates and an API (Dotabuff,
Stratz, OpenDota), the "stat bibles" that statisticians prepare for casters and analysts before a
tournament (datDota), win prediction models that explain a game as it happens (presented at the MIT
Sloan Sports Analytics Conference), and HUD managers that put live comparisons and timers on the
stream (Lexogrine HUD Manager).

- [ ] **Stat bible**: one page per player of a tournament for the casters, exported as HTML/PDF:
  matchup and map win rates, signature openings and builds, head-to-head records, recent form,
  records (fastest third base, most workers at 6:00) and fun facts.
- [ ] **Win probability over the game**: a model trained on the snapshot from the leads at each
  minute (workers, army value, income, bases, upgrades), the curve of a replay shows the turning
  points. functime/polars in s2-polars-data-analysis is a place to prototype it.
- [ ] **Map veto helper**: each player's win rate per map and matchup in the current pool, which
  maps each one would veto, for best-of series.
- [ ] **Turning points and highlights**: the engagements with the largest swing in army value or win
  probability, with the replay time to jump to in Rerun or swarmy-bevy.
- [ ] **Live overlay**: a browser source for OBS with the army, income and worker comparisons of
  the game being casted, from the SC2 client API (`localhost:6119`) while it is played.
- [ ] **Fun facts generator**: streaks, records per map and patch, first time a build was played,
  "the last time player A beat player B on this map".
- [ ] **Build order popularity per patch**: the "pick/ban rates" of StarCraft II, which openings are
  rising or falling after a balance patch.
- [ ] **Balance dashboard**: the matchup win rates per patch with confidence intervals and the
  number of games, so that a 60% from 20 games is not read as imbalance.
- [ ] **An API or export of the aggregates** (like OpenDota) so that other tools and overlays can
  use them.
- [ ] Normalize the localized race names (테란, Терраны, 神族...) in s2protocol-rs, swarmy and the
  notebooks map them to T, Z and P for now.

## Rerun preview
- Some units generate too many actions to be displayed in the current way as separate events in rerun,  for example a Lurker may create a temporary InvisibleTargetUnit for every cycle the spines shoots, this needs to be somehow shortened.
- Sometimes the state management in s2protocol-rs may lose sync, at this point rerun is very useful for debugging and seeing how the state transists through events and what abilities may be missing.
- Some events seem a bit redundanct, i.e. a Larva "dies" to become a "drone" or an "Adept" has an "AdeptPhaseShift" temporary unit that "dies" and this also causes a lot of events.
- The Target Position arrows (where each unit is pointing to) are batched in a single entity per player and cleared after a couple of seconds, they may still be too busy in big fights.
