# Use of a unit per period

## Purpose

The chart over time of the Unit Overview: how many of a unit were made in each day, month or
year, in how many player games, how many of those were won, and what the units did (died,
kills, resources killed), against the games of the race that makes it
([`race_games`](../race_games/README.md), the race from [`unit_race`](../unit_race/README.md)).

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `unit_usage` of the grain | `period`, `unit`, `games`, `wins`, `born`, `died`, `kills`, `value_killed` | one per period, protocol version, race, account, player and unit |

The grain and the unit (its base name, a sieged tank is a `SiegeTank`) are the parameters;
below the months and `'Zergling'`, the table `month_unit_usage`.

## Output

One row per period the unit was made in, the oldest first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `period` | str | `2026-01-15`, `2026-01` or `2026` | never |
| `games` | u32 | the player games the unit was made in | never |
| `wins` | u32 | the ones won | never |
| `born` | u64 | the units made | never |
| `died` | u64 | the units of the type that died | never |
| `kills` | u64 | the enemy units that cost resources they killed | never |
| `value_killed` | u64 | the minerals and vespene of those | never |

## Query

```sql
SELECT period, SUM(games) AS games, SUM(wins) AS wins, SUM(born) AS born, SUM(died) AS died,
       SUM(kills) AS kills, SUM(value_killed) AS value_killed
FROM month_unit_usage
WHERE unit = 'Zergling'
GROUP BY period
ORDER BY period
```

## Edge cases

- The unit is matched by its name in the aggregations, the base name of its variants.
- A period where nobody made the unit has no row: the view shows 0 for it.

## Performance

The usage table of the grain, a filter and a group by: milliseconds.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): Alice made
30 Zerglings with 10 kills in game 5 (February, won). Expected: one row, `2026-02`, 1 game, 1
win, 30 born, 0 died, 10 kills, 500 resources killed.
