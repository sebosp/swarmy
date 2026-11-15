# Average time of the upgrades in a patch

## Purpose

When each upgrade usually finishes in the games of the patch, so that the build order of a
replay says how much faster or slower each of its upgrades was ("12s slower than average").
Computed once per protocol version and kept in memory (`patch_timings_cached`).

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `upgrades` | `ext_fs_id`, `name`, `ext_replay_loop` | one per upgrade finished by a player |

The ids of the games of the version ([`games`](../games/README.md)) are the parameter,
`(1, 2, 3, 4, 5, 6)` below.

## Output

One row per upgrade finished at least 3 times (`MIN_SAMPLES`), by name.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `name` | str | the upgrade as the tracker names it, `TerranInfantryWeaponsLevel1` | never |
| `game_seconds` | u32 | the average game second it finished | never |
| `samples` | u32 | the times it finished in the games | never |

## Query

```sql
SELECT * FROM (
    SELECT name,
           CAST(AVG(CAST(CAST(ext_replay_loop AS DOUBLE) / 22.4 AS UINT)) AS UINT) AS game_seconds,
           COUNT(*) AS samples
    FROM upgrades
    WHERE ext_fs_id IN (1, 2, 3, 4, 5, 6)
      AND ext_replay_loop > 0
      AND NOT (STARTS_WITH(name, 'Spray') OR STARTS_WITH(name, 'Reward') OR STARTS_WITH(name, 'GameHeart'))
    GROUP BY name
)
WHERE samples >= 3
ORDER BY name
```

## Edge cases

- The sprays, rewards and GameHeart markers are registered as upgrades at loop 0 or when used:
  not part of a build order, left out.
- An upgrade finished twice in a game (two players) is two samples.
- Fewer than three samples: no row, the replay shows no average for it.

## Performance

A filter of the `upgrades` table of the games and a group by: tens of milliseconds, once per
version.

## Example

The games of the test (`tests.rs`, `replay_detail::test_games::patch_games`): Terran Infantry
Weapons Level 1 finished at 300, 310 and 320 s in games 1 to 3 (average 310, 3 samples);
Stimpack twice only and a Terran spray are left out.
