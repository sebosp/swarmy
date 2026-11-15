# Average time of the expansions in a patch

## Purpose

When the players of the patch usually take their first, second, third expansion of each kind, so
that a replay says how much earlier or later each expansion of the build order was. The snapshot
has the townhalls once they finish: the averages are of the finish, `patch_timings` subtracts the
build time of the patch to compare when they started.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `unit_born` | `ext_fs_id`, `control_player_id`, `unit_tag_index`, `unit_tag_recycle`, `unit_type_name`, `ext_replay_loop` | one per unit born |

The ids of the games of the version ([`games`](../games/README.md)) are the parameter,
`(1, 2, 3, 4, 5, 6)` below.

## Output

One row per townhall type and number with at least 3 samples (`MIN_SAMPLES`), by type and
number.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `unit_type_name` | str | `Hatchery`, `CommandCenter` or `Nexus` | never |
| `number` | u32 | the nth townhall of the player after the initial one | never |
| `game_seconds` | u32 | the average game second it finished | never |
| `samples` | u32 | the townhalls averaged | never |

## Query

```sql
WITH townhalls AS (
    SELECT ext_fs_id, control_player_id, unit_tag_index, unit_tag_recycle, unit_type_name,
           CAST(CAST(ext_replay_loop AS DOUBLE) / 22.4 AS UINT) AS game_seconds
    FROM unit_born
    WHERE ext_fs_id IN (1, 2, 3, 4, 5, 6)
      AND ext_replay_loop > 0
      AND unit_type_name IN ('Hatchery', 'CommandCenter', 'Nexus')
),
finished AS (
    SELECT ext_fs_id, control_player_id, unit_tag_index,
           FIRST(unit_type_name) AS unit_type_name, MIN(game_seconds) AS game_seconds
    FROM townhalls
    GROUP BY ext_fs_id, control_player_id, unit_tag_index, unit_tag_recycle
),
numbered AS (
    SELECT unit_type_name, game_seconds,
           CAST(ROW_NUMBER() OVER (PARTITION BY ext_fs_id, control_player_id ORDER BY game_seconds, unit_tag_index) AS UINT) AS number
    FROM finished
)
SELECT * FROM (
    SELECT unit_type_name, number, CAST(AVG(game_seconds) AS UINT) AS game_seconds, COUNT(*) AS samples
    FROM numbered
    GROUP BY unit_type_name, number
)
WHERE samples >= 3
ORDER BY unit_type_name, number
```

## Edge cases

- A Command Center that lifts off and lands is born again with the same tag: counted once, at
  its first finish.
- The initial townhall is born at loop 0: not an expansion.
- A townhall of another type (a Terran making a Hatchery with a mind controlled Drone) is
  numbered among that type's.
- Fewer than three samples: no row.

## Performance

A filter of the `unit_born` table of the games (the townhalls are few), a group by and a window:
tens of milliseconds, once per version.

## Example

The games of the test (`tests.rs`, `replay_detail::test_games::patch_games`): the Terran players
of games 1 to 4 finished their second Command Center at 120, 130, 140 and 150 s (the one of game
1 landed again at 400 s), the Zerg players of games 1 to 5 their second Hatchery at 100, 110,
120, 130 and 140 s, Bob a third one at 300 s in game 5 only. Expected: CommandCenter 1 at 135 s
(4), Hatchery 1 at 120 s (5).
