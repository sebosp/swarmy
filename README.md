# swarmy 1.2.0

![Starcraft 2 Replay in Rerun](https://user-images.githubusercontent.com/873436/231281746-40fde3f1-fec6-49fe-8cf1-5fbd197589b7.png)

## Web UI Setup

The timeline must be switched to the "game_timeline" in the dropdown.
The gamespeed seems to be 22 FPS for "faster" game speed. This kindof matches replays.

![Screenshot for 2025EWC Grand Finals](https://github.com/user-attachments/assets/bf6ca086-db19-4054-94d9-0f1ae9a74b11)
Created with:
```
$ cargo run -- --verbosity-level info --source /home/seb/SCReplaysOnNVMe/2025EWCReplayPack/3\ -\ Playoffs/8\ -\ Grand\ Finals\ -\ Serral\ vs\ Classic/20250725\ -\ Game7\ -\ Classic\ vs\ Serral\ -\ Magannatha.SC2Replay --json-balance-data-dir=/home/seb/git/s2protocol-rs/assets/BalanceData/ --connect rerun+http://localhost:9876/proxy --filter-max-events 15000
```

## Example Recorded/Processed Replays

Rerun 0.26.0:

[2025EWCClassicSerralGame8GrandFinals](https://sebosp.github.io/swarmy/public/0.26.0/index.html)


## Running:

For now this has been tested on Linux.

[Install Rust](https://www.rust-lang.org/tools/install)
Clone this repo.

```shell
# Clone this repository.
$ cargo run -r -- --source <FILE>
# To run the example file provided in this repo:
$ cargo run -- --source assets/2023-04-08-2v2AI.SC2Replay --connect rerun+http://localhost:9876/proxy --filter-max-events 1000 --json-balance-data-dir $HOME/SC2Replays/BalanceData/
# The first time the code is compiled it will take a few minutes.
# Subsequent runs should not need compilation.
```
Then open a browser to http://localhost:9101/?url=rerun%2Bhttp%3A%2F%2Flocalhost%3A9876%2Fproxy

## Status
Working:
- The minerals are recognized and drawn.
- Units Created
- Units Died
- Camera Positions
- The Unit targets (either points or other units)
- Active Units are highlighted with increased radius.
- Unit abilities are recognized, so as their commands


## Motivation:

Appreciate a fantastic game at a lower level, learn how people use and learn the game.

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
- Some units generate too many actions to be displayed in the current way as separate events in rerun,  for example a Lurker may create a temporary InvisibleTargetUnit for every cycle the spines shoots, this needs to be somehow shortened.
- Sometimes the state management in s2protocol-rs may lose sync, at this point rerun is very useful for debugging and seeing how the state transists through events and what abilities may be missing.
- Some events seem a bit redundanct, i.e. a Larva "dies" to become a "drone" or an "Adept" has an "AdeptPhaseShift" temporary unit that "dies" and this also causes a lot of events.
- The Target Position (arrows signaling where each unit is pointing to) are not shown anymore as it just creates way too many objects in the map and makes it impossible to use.
