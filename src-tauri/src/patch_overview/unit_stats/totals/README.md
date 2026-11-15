# Totals of a unit in the snapshot

## Purpose

The six numbers at the top of the "In the snapshot" panel of the Unit Overview: in how many of
the games of the snapshot the unit was made, how many were made, died, how many enemy units they
killed and the resources of those. The kills per unit and the value killed per cost are divided
from them in the view.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id` | one per player per game |
| `game_units` | `ext_fs_id`, `unit`, `born`, `died`, `kills`, `value_killed` | one per player, game and unit type |

The unit is the parameter, by its base name (a sieged tank is a `SiegeTank`), `'Marine'` below.

## Output

One row.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `games` | u32 | the games of the snapshot | never |
| `games_with_unit` | u32 | the games in which a player made the unit | never |
| `born` | u32 | the units made | never |
| `died` | u32 | the units that died | never |
| `kills` | u32 | the enemy units that cost resources they killed | never |
| `value_killed` | u64 | the minerals and vespene of those | never |

## Query

```sql
WITH snapshot AS (
    SELECT COUNT(DISTINCT ext_fs_id) AS games FROM details
),
made AS (
    SELECT COUNT(DISTINCT ext_fs_id) AS games_with_unit, SUM(born) AS born, SUM(kills) AS kills,
           SUM(value_killed) AS value_killed
    FROM game_units
    WHERE unit = 'Marine' AND born > 0
),
lost AS (
    SELECT SUM(died) AS died FROM game_units WHERE unit = 'Marine'
)
SELECT snapshot.games, made.games_with_unit, made.born, lost.died, made.kills, made.value_killed
FROM snapshot CROSS JOIN made CROSS JOIN lost
```

## Edge cases

- A unit the player did not make (mind controlled, given by an ally) counts its death but not
  its kills, and its game is not a game with the unit: the kills are of the units made.
- A unit never made: one row of zeros, with the games of the snapshot.

## Performance

Two aggregations over hot tables: tens of milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `patch_overview::unit_stats::test_games::marine_makers`):
eight games, Marines made in seven of them (60 + 2 = 62 born, 3 died, 25 + 50 + 20 = 95
kills, 4,750 resources).
