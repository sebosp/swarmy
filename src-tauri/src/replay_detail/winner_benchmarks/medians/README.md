# Medians of the winners of a matchup

## Purpose

What the winners of the same matchup had at 04:00, 06:00, 08:00 and 10:00 in the games of the
patch: the median workers, supply, income and army value of the human players who won a 1v1 with
the same race against the same race, so that a replay shows where its players were behind the
games that were won ("/ the median of the 312 winners of the matchup in the patch").

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `player_name`, `player_race`, `player_result`, `player_control`, `player_observe`, `ext_protocol_version` | one per player per game |
| `game_players` | `ext_fs_id`, `player_name`, `player_id` | one per player per game, the tracker ids |
| `game_minutes` | `ext_fs_id`, `player_id`, `minute`, `workers`, `supply`, `minerals_rate`, `vespene_rate`, `army_value` | one per player, game and minute |

The protocol version is the parameter, 95248 below.

## Output

One row per race, enemy race and benchmark minute with at least 5 winners
(`MIN_WINNER_SAMPLES`), by race, enemy race and minute.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `race` | str | the race of the winners | never |
| `enemy_race` | str | the race they beat | never |
| `minute` | u32 | 4, 6, 8 or 10 | never |
| `workers` | i32 | the median workers of the winners at the minute | never |
| `supply` | i32 | the median supply used | never |
| `minerals_rate` | i32 | the median minerals collected per minute | never |
| `vespene_rate` | i32 | the median vespene per minute | never |
| `army_value` | i32 | the median resources of the army | never |
| `samples` | u32 | the winners with a sample at the minute | never |

## Query

```sql
WITH ids AS (
    SELECT DISTINCT ext_fs_id, player_name, player_id FROM game_players
),
players AS (
    SELECT d.ext_fs_id, i.player_id,
           CASE WHEN d.player_race = 'Terran' THEN 'Terran'
                WHEN d.player_race = 'Zerg' THEN 'Zerg'
                WHEN d.player_race = 'Protoss' THEN 'Protoss'
                ELSE 'Other' END AS race,
           d.player_result, d.player_control
    FROM details d
    JOIN ids i ON d.ext_fs_id = i.ext_fs_id AND d.player_name = i.player_name
    WHERE d.player_observe = 0 AND d.ext_protocol_version = 95248
),
games AS (
    SELECT ext_fs_id FROM (
        SELECT ext_fs_id, COUNT(*) AS players, MIN(player_control) AS min_control, MAX(player_control) AS max_control
        FROM players GROUP BY ext_fs_id
    )
    WHERE players = 2 AND min_control = 2 AND max_control = 2
),
winners AS (
    SELECT p.ext_fs_id, p.player_id, p.race, e.race AS enemy_race
    FROM players p
    JOIN games g ON p.ext_fs_id = g.ext_fs_id
    JOIN players e ON p.ext_fs_id = e.ext_fs_id
    WHERE p.player_result = 'Win' AND p.player_id <> e.player_id
)
SELECT * FROM (
    SELECT w.race, w.enemy_race, m.minute,
           CAST(MEDIAN(m.workers) AS INT) AS workers,
           CAST(MEDIAN(m.supply) AS INT) AS supply,
           CAST(MEDIAN(m.minerals_rate) AS INT) AS minerals_rate,
           CAST(MEDIAN(m.vespene_rate) AS INT) AS vespene_rate,
           CAST(MEDIAN(m.army_value) AS INT) AS army_value,
           COUNT(*) AS samples
    FROM game_minutes m
    JOIN winners w ON m.ext_fs_id = w.ext_fs_id AND m.player_id = w.player_id
    WHERE m.minute IN (4, 6, 8, 10)
    GROUP BY w.race, w.enemy_race, m.minute
)
WHERE samples >= 5
ORDER BY race, enemy_race, minute
```

The Polars plan maps the localized race names too (`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`).

## Edge cases

- A game against a computer (`player_control` 3) is not a game of two humans.
- A mirror is a matchup too, `ZvZ`: the winners of either side.
- A game that ended before the minute has no sample for it: the medians of the later minutes
  are of the longer games.
- A tie or a game without a result has no winner.
- The medians are truncated to whole numbers.

## Performance

The players of the version and their minutes: a hundred milliseconds, once per version.

## Example

The games of the test (`tests.rs`, `replay_detail::test_games::patch_games`): the Terran players
won the five human TvZ games 1 to 5 with 20, 21, 22, 23 and 30 workers at minute 4 (median 22,
supply 30, 500 + 200 income, army 400) and 36, 37, 38, 39 and 50 at minute 6 (median 38); game 6
is against a computer. The Zerg won none: no `ZvT` rows.
