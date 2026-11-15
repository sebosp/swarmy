# Use of an ability per period

## Purpose

The chart over time of the Ability view (and of Race Detail's abilities): in how many games the
players of the race used the ability each day, month or year, how many of those they won and
how many times they used it, against the games of the race of the same period
([`race_games`](../race_games/README.md)).

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `ability_usage` of the grain | `period`, `player_race`, `ability`, `games`, `wins`, `uses` | one per period, protocol version, race, account, player and ability |

The grain, the ability and its race are the parameters; below the months, `'SpawnLarva.Execute'`
and `'Zerg'`, the table `month_ability_usage`.

## Output

One row per period with a use, the oldest first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `period` | str | `2026-01-15`, `2026-01` or `2026` | never |
| `games` | u32 | the player games the ability was used in | never |
| `wins` | u32 | the ones won | never |
| `uses` | u64 | the uses | never |

## Query

```sql
SELECT period, SUM(games) AS games, SUM(wins) AS wins, SUM(uses) AS uses
FROM month_ability_usage
WHERE ability = 'SpawnLarva.Execute'
  AND CASE WHEN player_race = 'Terran' THEN 'Terran'
           WHEN player_race = 'Zerg' THEN 'Zerg'
           WHEN player_race = 'Protoss' THEN 'Protoss'
           ELSE 'Other' END = 'Zerg'
GROUP BY period
ORDER BY period
```

## Edge cases

- The race filter leaves out a mind-controlled or mirrored use by another race.
- A period where nobody used the ability has no row: the view shows 0 for it, with the games of
  the race.

## Performance

The usage table of the grain, a filter and a group by: milliseconds.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends` with
abilities): Bob used Spawn Larva 20 times in game 1 (January, lost), Alice 15 times in game 5
(February, won). Expected: `2026-01` 1 game 0 wins 20 uses, `2026-02` 1 game 1 win 15 uses.
