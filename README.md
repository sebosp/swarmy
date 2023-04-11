# swarmy
Starcraft 2 Replay in Rerun

## Running on native.

For now this has been tested on Linux.

[Install Rust](https://www.rust-lang.org/tools/install)
Clone this repo.

```shell
# Clone this repository.
$ cargo run -r -- --source <FILE>
# The first time the code is compiled it will take a few minutes.
# Subsequent runs should not need compilation.
```

## Running in browser.

Running in browser requires exporting the `.rrd` file from the previous step.
Basically load the Rerun viewer, in the menu Export the RRD and download.

```shell
$ cargo install rerun
$ rerun --web-viewer my-downloaded-file.rrd
```

## Status
Very basic initial setup.

The minerals are recognized and drawn.

The drones are visible in their initial position.

![Initial preview](https://user-images.githubusercontent.com/873436/231281746-40fde3f1-fec6-49fe-8cf1-5fbd197589b7.png)

## Motivation:

Appreciate a fantastic game at a different level, learn how people use and learn the game.

## Uses:

One of the first uses I'll add is a helper for Casters.

### Initial Focus
- Colored fog of war, which player has scouted which area.
- Hint on next region of interest: Since this is for replays, we know when important events are going to happen.
  we can guide the caster to position the window to specific regions where in the next X gameloops, an important
  activity is happening, for example massive damage or massive death on an area, i.e. splash from widow mines, disruptors, tanks.
  This could be similar to how a warning is seen for a Nydus or Nukes.
- Different visualizations on events, for example, we could draw the amount of deaths per regions of the map, like piling up bodies
  and showing what regions of a map have been more active than others.
- We can add many data visualizations.

### Future focus
- It is possible to capture a replay for training purposes, so that one has to follow the same actions a Pro-player does.
  for example, a pro-player may execute a build order and export it, then it can be imported into swarmy,
  you would have to follow the same operations/steps the Pro-player and you can be graded in timing/accuracy/etc.

## TODO:
Everything, this is super early state, all suggestions are welcome.
