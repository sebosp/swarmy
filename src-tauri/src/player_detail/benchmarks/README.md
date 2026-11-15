# Economy benchmarks of the snapshot

## Purpose

What a player's economy is compared to in the Player view ("Benchmarks" of the insights): the
average workers, supply, income and army value at minutes 4, 6, 8 and 10
(`replay_detail::data::BENCHMARK_MINUTES`) of every player of a 1v1 game of the snapshot, per
matchup from the player's side (`ZvT`: the Zerg players against Terran). Computed once per
snapshot and kept in memory (`insights::snapshot_benchmarks`).

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `players` (`data::snapshot_players`: `details`, `game_players`, `patch_versions`) | `ext_fs_id`, `player_id`, `race` | one per player per game, no observers, StarCraft II races only |
| `game_minutes` | `ext_fs_id`, `player_id`, `minute`, `workers`, `supply`, `minerals_rate`, `vespene_rate`, `army_value` | one per player, game and minute |

## Output

One row per matchup and benchmark minute, by matchup and minute.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `matchup` | str | the player's race letter, `v`, the opponent's | never |
| `minute` | u32 | 4, 6, 8 or 10 | never |
| `games` | u32 | the player games with a sample at the minute | never |
| `workers` | f64 | the average workers | never |
| `supply` | f64 | the average supply used | never |
| `income` | f64 | the average minerals plus vespene collected per minute | never |
| `army_value` | f64 | the average resources of the army | never |

## Query

The `players` CTE is `data::snapshot_players`; `pairs` numbers the two rows of a 1v1, so that
the other row is found even when two players have the same name.

```sql
WITH ids AS (
    SELECT DISTINCT ext_fs_id, player_name, player_id FROM game_players
),
named AS (
    SELECT d.ext_fs_id, d.player_name, i.player_id,
           CASE WHEN d.player_race = 'Terran' THEN 'Terran'
                WHEN d.player_race = 'Zerg' THEN 'Zerg'
                WHEN d.player_race = 'Protoss' THEN 'Protoss'
                ELSE 'Other' END AS race,
           d.player_result, d.title, v.patch
    FROM details d
    JOIN patch_versions v ON d.ext_protocol_version = v.ext_protocol_version
    LEFT JOIN ids i ON d.ext_fs_id = i.ext_fs_id AND d.player_name = i.player_name
    WHERE d.player_observe = 0
),
players AS (
    SELECT * FROM named WHERE race <> 'Other'
),
counts AS (
    SELECT ext_fs_id, COUNT(*) AS n FROM players GROUP BY ext_fs_id
),
pairs AS (
    SELECT p.*, ROW_NUMBER() OVER (PARTITION BY p.ext_fs_id ORDER BY p.player_name) AS side
    FROM players p JOIN counts c ON p.ext_fs_id = c.ext_fs_id WHERE c.n = 2
),
matchups AS (
    SELECT a.ext_fs_id, a.player_id, CONCAT(LEFT(a.race, 1), 'v', LEFT(b.race, 1)) AS matchup
    FROM pairs a
    JOIN pairs b ON a.ext_fs_id = b.ext_fs_id
    WHERE a.side <> b.side
)
SELECT m.matchup, g.minute, COUNT(*) AS games,
       AVG(g.workers) AS workers,
       AVG(g.supply) AS supply,
       AVG(g.minerals_rate + g.vespene_rate) AS income,
       AVG(g.army_value) AS army_value
FROM game_minutes g
JOIN matchups m ON g.ext_fs_id = m.ext_fs_id AND g.player_id = m.player_id
WHERE g.minute IN (4, 6, 8, 10)
GROUP BY m.matchup, g.minute
ORDER BY m.matchup, g.minute
```

The Polars plan maps the localized race names too (`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`).

## Edge cases

- A mirror (`ZvZ`) counts both players, each from their side.
- A game shorter than a benchmark minute has no sample for it: not in `games` of that minute.
- A 1v1 watched by observers is a 1v1: the observers are not players. A game against an arcade
  race (Primal Zerg) is not: one player only.

## Performance

The pairing of the players of every game and a join with the minutes: a hundred milliseconds
on the 36,000-game snapshot, once per snapshot.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::with_minutes`): in game 1 Alice
(Terran) has 20 workers at minute 4 and Bob (Zerg) 22; in game 6 Alice2 (Terran) has 16 and Bob
20. Expected: `TvZ` minute 4, 2 games, 18 workers; `ZvT` minute 4, 2 games, 21 workers.
