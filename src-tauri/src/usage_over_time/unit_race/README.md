# Race of a unit

## Purpose

The race whose games the use of a unit is divided by in the Unit Overview chart: the race that
made the most of it. The balance data knows the race of a unit too, but a mind-controlled or
mirrored unit has rows under another race; the one with the most made is the maker.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `unit_usage` by year | `unit`, `player_race`, `born` | one per year, protocol version, race, account, player and unit |

The unit is the parameter, `'Zergling'` below.

## Output

One row, or none when the unit was never made.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `race` | str | `Terran`, `Zerg`, `Protoss` or `Other` | never |
| `born` | u64 | the units made by that race | never |

## Query

```sql
SELECT CASE WHEN player_race = 'Terran' THEN 'Terran'
            WHEN player_race = 'Zerg' THEN 'Zerg'
            WHEN player_race = 'Protoss' THEN 'Protoss'
            ELSE 'Other' END AS race,
       SUM(born) AS born
FROM year_unit_usage
WHERE unit = 'Zergling'
GROUP BY race
ORDER BY born DESC, race
LIMIT 1
```

## Edge cases

- Two races with the same count: the first by name.
- A unit never made gives no row and an empty race: the chart then has no games to divide by.

## Performance

The yearly usage table, a few rows per unit: a millisecond.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): Alice made
30 Zerglings as Zerg. Expected: `Zerg`, 30.
