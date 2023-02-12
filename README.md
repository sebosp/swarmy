# swarmy
Starcraft 2 Replay in Bevy


## Status
Very basic initial setup.

A 2v2 game on Heavy Artillery on build 87702.

The minerals are recognized and drawn.

The units being borned are shown in dark green.

The drones are visible in their initial position.

![Initial preview](https://user-images.githubusercontent.com/873436/218337062-083b71d1-4a5a-45bc-883f-14da06f0f840.png)

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
Everything, this is super early state, all input is welcome.
