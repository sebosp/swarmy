# The deaths of a replay on the map

## Purpose

Where and when the units of a replay died: the points of the density map of the deaths
([`death_density`](../death_density/README.md), the Deaths view of the map of the Replay view),
which builds on this query. Computed when the replay is opened.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `unit_died` | `ext_fs_id`, `unit_died_name`, `cause`, `x`, `y`, `ext_replay_loop` | one per unit died |
| `valued_units` | `variant` | the units and structures that cost resources, with their variants; a frame swarmy builds, `replay_detail::valued_units::valued_units` |

The id of the replay is the parameter, `1` below.

## Output

One row per death that cost resources, by time and position.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `x` | f32 | where it died, in map units | never |
| `y` | f32 | its y | never |
| `game_seconds` | u32 | the game second | never |

## Query

```sql
SELECT CAST(d.x AS REAL) AS x, CAST(d.y AS REAL) AS y,
       CAST(CAST(d.ext_replay_loop AS DOUBLE) / 22.4 AS UINT) AS game_seconds
FROM unit_died d
JOIN valued_units v ON d.unit_died_name = v.variant
WHERE d.ext_fs_id = 1
  AND d.cause IN ('killed', 'own_player')
ORDER BY game_seconds, x, y
```

## Edge cases

- A unit without a cost (a Larva, a Broodling, an Interceptor) is not a death; a death whose
  `cause` is not a loss (killed or own_player) is not one either: a Drone that became a
  building, templars merged into an Archon, a MULE or an AutoTurret that expired, a cancelled
  structure, a hallucination.
- A unit that dies to its own player (`own_player`: a Baneling that exploded, a burned-down
  building, a salvaged Bunker) counts: it died there. A cancelled structure (`cancelled`) does
  not, most of its cost is refunded.
- A replay not in the snapshot has no rows, the map shows no deaths.

## Performance

A filter of the `unit_died` table of one game: milliseconds.

## Example

The game of the test (`tests.rs`, `replay_detail::test_games::fight`): two Marines, an SCV, two
Zerglings and an Overlord die, a Larva dies, a Drone and a Zergling morph and a Supply Depot is
cancelled. Expected: six rows, the Marines
and Zerglings of 300 s to 310 s around (50, 50), the SCV at (30, 30) at 450 s and the Overlord at
(10, 90) at 500 s.
