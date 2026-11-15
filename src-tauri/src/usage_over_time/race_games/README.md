# Games of a race per period

## Purpose

The base of every chart over time (Race Detail, Ability, Unit Overview): how many games the
players of a race played in each day, month or year, and their 1v1 games against the other races
with how many they won. The uses of an ability or a unit are divided by these games.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `player_usage` of the grain (`year=*/month=*/player_usage.arrow` for months) | `period`, `player_race`, `games`, `versus_games`, `versus_wins` | one per period, protocol version, race, account and player |

The grain (day, month, year) and the race are the parameters; below the months and `'Zerg'`,
the table `month_player_usage`.

## Output

One row per period, the oldest first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `period` | str | `2026-01-15`, `2026-01` or `2026` | never |
| `games` | u32 | the games of the players of the race in the period | never |
| `versus_games` | u32 | the 1v1 games of the race against another race | never |
| `versus_wins` | u32 | the ones the race won | never |

## Query

```sql
SELECT period, SUM(games) AS games, SUM(versus_games) AS versus_games, SUM(versus_wins) AS versus_wins
FROM month_player_usage
WHERE CASE WHEN player_race = 'Terran' THEN 'Terran'
           WHEN player_race = 'Zerg' THEN 'Zerg'
           WHEN player_race = 'Protoss' THEN 'Protoss'
           ELSE 'Other' END = 'Zerg'
GROUP BY period
ORDER BY period
```

The Polars plan maps the localized race names too (`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`).

## Edge cases

- A mirror is not a `versus` game, the race always wins it.
- Team games count as games of the race, not as `versus` games.
- A period without games of the race has no row: the view adds the empty periods between the
  first and the last.

## Performance

The usage tables are small (a row per player, race and period) and read from the day, month or
year partitions: milliseconds, tens of them for the days of the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`), the Zerg
players by month: January, Bob lost to a Terran (1 game, 1 versus, 0 won); February, Dave tied
a Terran, Bob beat a Protoss computer, and Alice, Bob and Dave played a 2v2 (5 games, 2 versus,
1 won); March, Bob beat a Terran (1 game, 1 versus, 1 won).
