# The epicenters of the engagements of a replay

## Purpose

Where the fights of a replay happened: for each engagement of the Replay view (units dying close
in time, `replay_detail::data::engagements`), the center of the deaths of its seconds and how far
around it they spread, drawn on the map of the view; hovering the position in the Engagements
table shows the circle on the map. Computed when the replay is opened.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `unit_died` | `ext_fs_id`, `unit_died_name`, `cause`, `x`, `y`, `ext_replay_loop` | one per unit died |
| `engagements` | `engagement`, `start_seconds`, `end_seconds` | one per engagement of the replay; a frame swarmy builds from the analysis of the replay file, `epicenters::engagement_windows` |
| `valued_units` | `variant` | the units and structures that cost resources, with their variants; a frame swarmy builds, `replay_detail::valued_units::valued_units` |

The id of the replay is the parameter, `1` below.

## Output

One row per engagement with at least one death in the snapshot, by engagement.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `engagement` | u32 | the position of the engagement in the list of the replay, from 0 | never |
| `x` | f32 | the mean x of the deaths of the engagement, in map units | never |
| `y` | f32 | their mean y | never |
| `radius` | f32 | the root mean square distance of the deaths to (x, y): `sqrt(mean((x - mean x)² + (y - mean y)²))` | never |
| `deaths` | u32 | the deaths counted | never |

## Query

```sql
WITH deaths AS (
    SELECT CAST(d.x AS REAL) AS x, CAST(d.y AS REAL) AS y,
           CAST(CAST(d.ext_replay_loop AS DOUBLE) / 22.4 AS UINT) AS game_seconds
    FROM unit_died d
    JOIN valued_units v ON d.unit_died_name = v.variant
    WHERE d.ext_fs_id = 1
      AND d.cause IN ('killed', 'own_player')
),
in_window AS (
    SELECT e.engagement, d.x, d.y
    FROM engagements e
    CROSS JOIN deaths d
    WHERE d.game_seconds >= e.start_seconds AND d.game_seconds <= e.end_seconds
),
centers AS (
    SELECT engagement AS center_of, AVG(x) AS center_x, AVG(y) AS center_y, COUNT(*) AS deaths
    FROM in_window
    GROUP BY engagement
)
SELECT w.engagement,
       CAST(c.center_x AS REAL) AS x,
       CAST(c.center_y AS REAL) AS y,
       CAST(SQRT(AVG(POWER(w.x - c.center_x, 2) + POWER(w.y - c.center_y, 2))) AS REAL) AS radius,
       CAST(c.deaths AS UINT) AS deaths
FROM in_window w
JOIN centers c ON w.engagement = c.center_of
GROUP BY w.engagement, c.center_x, c.center_y, c.deaths
ORDER BY w.engagement
```

## Edge cases

- The deaths are the ones of the engagement's seconds, from its first to its last death: the
  deaths the analysis of the replay file counted, as the snapshot has them.
- A unit without a cost (a Larva, a Broodling, an Interceptor) is not a death; a death whose
  `cause` is not a loss (killed or own_player) is not one either: a Drone that became a
  building, templars merged into an Archon, a MULE or an AutoTurret that expired, a cancelled
  structure, a hallucination.
- An engagement with a single death has radius 0. Two fights at once in different places (a
  drop during a battle) give one center between them and a large radius: the view shows the
  radius, so that a center in the middle of nowhere is read as that.
- An engagement without deaths in the snapshot (an older snapshot of the replay) has no row, the
  view shows no position for it.

## Performance

A filter of the `unit_died` table of one game and a cross join with a few dozen engagements:
milliseconds.

## Example

The game of the test (`tests.rs`, `replay_detail::test_games::fight`): a fight from 300 s to
310 s in which Alice loses two Marines at (48, 50) and (52, 50) and Bob two Zerglings at (50, 48)
and (50, 52), while a Larva of Bob dies, a Drone and a Zergling of Bob morph and Alice cancels
a Supply Depot; at 500 s an Overlord dies alone at
(10, 90). Expected: engagement 0 at (50, 50) with radius 2 and 4 deaths, engagement 1 at
(10, 90) with radius 0 and 1 death.
