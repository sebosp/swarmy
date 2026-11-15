# Kills per unit of the players that made a unit

## Purpose

What a unit killed for each one made, per player and game, for the players that made a few of
them (`swarmy_common::EXCEPTIONAL_MIN_BORN`, 3): the distribution the Unit Overview takes its
average and its threshold of an exceptional game from (`threshold`: the mean plus two sample
standard deviations), and the rows [`exceptional`](../exceptional/README.md) picks from.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `game_units` | `ext_fs_id`, `player_name`, `unit`, `born`, `kills`, `value_killed` | one per player, game and unit type |

The unit and the minimum made are the parameters, `'Marine'` and 3 below.

## Output

One row per player and game, by game and player.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the game | never |
| `player_name` | str | the player, with the clan tag as in the replay | never |
| `born` | u32 | the units made | never |
| `kills` | u32 | the enemy units that cost resources they killed | never |
| `value_killed` | u64 | the resources of those | never |
| `kills_per_unit` | f64 | `kills / born` | never |

## Query

```sql
SELECT ext_fs_id, player_name, born, kills, value_killed,
       CAST(kills AS DOUBLE) / CAST(born AS DOUBLE) AS kills_per_unit
FROM game_units
WHERE unit = 'Marine' AND born >= 3
ORDER BY ext_fs_id, player_name
```

## Edge cases

- A player that made two of the unit is not compared: a single lucky kill would be exceptional.
- One player only: the standard deviation is null and taken as 0, the threshold is the mean.
- Nobody made enough: no rows, mean and threshold 0.

## Performance

A filter of the `game_units` table: milliseconds.

## Example

The games of the test (`tests.rs`, `patch_overview::unit_stats::test_games::marine_makers`):
seven players made Marines, six of them at least 3; five killed 0.5 per Marine and Fiona 5.0.
Mean 1.25, standard deviation 1.837, threshold 4.924.
