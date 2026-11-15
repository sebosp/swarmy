# Use of a unit by month

## Purpose

How the use of a unit changed with time, as the Unit Overview sends it: for each month of the
snapshot, the games played, the games in which a player made the unit and how many were made.
Every month with a game is a row, so that a month where nobody made the unit shows as 0.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| [`games`](../games/README.md) | `ext_fs_id`, `month` | one per game |
| `game_units` | `ext_fs_id`, `unit`, `born` | one per player, game and unit type |

The unit is the parameter, `'Marine'` below.

## Output

One row per month with games, the oldest first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `month` | str | `YYYY-MM` | never |
| `games` | u32 | the games of the month | never |
| `games_with_unit` | u32 | the ones in which the unit was made | never |
| `born` | u32 | the units made in the month | never |

## Query

```sql
WITH games AS (
    SELECT ext_fs_id, STRFTIME(MIN(ext_datetime), '%Y-%m') AS month
    FROM details
    GROUP BY ext_fs_id
),
made AS (
    SELECT ext_fs_id, SUM(born) AS born
    FROM game_units
    WHERE unit = 'Marine' AND born > 0
    GROUP BY ext_fs_id
)
SELECT games.month, COUNT(*) AS games, COUNT(made.born) AS games_with_unit, SUM(made.born) AS born
FROM games LEFT JOIN made ON games.ext_fs_id = made.ext_fs_id
GROUP BY games.month
ORDER BY games.month
```

## Edge cases

- A month without the unit: `games_with_unit` and `born` are 0 (the count of nulls, the sum of
  nothing).
- Two players making the unit in a game: one game with the unit, the units of both.

## Performance

A group by of the games, a left join and a group by: tens of milliseconds.

## Example

The games of the test (`tests.rs`, `patch_overview::unit_stats::test_games::marine_makers`):
`2026-01`, 6 games, 6 with Marines, 60 born; `2026-02`, 2 games, 1 with Marines (the other is
Zerg against Protoss), 2 born.
