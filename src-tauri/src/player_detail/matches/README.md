# Matches of a player

## Purpose

The Matches table of the Player view: every game of the player with the opponent, the map, the
races and the result, the newest first; a team game is one row per opponent. The Opponents table
and the insights of the view are computed from these rows.

## Inputs

The rows of [`player_rows`](../player_rows/README.md): `player`, `ext_fs_id`, `player_name`,
`ext_datetime`, `title`, `player_race`, `player_result`, `ext_protocol_version`, `name`,
`player_team_id`. The player is the parameter of the query, `'Alice'` below.

## Output

One row per game and opponent, the newest game first, the opponents by name within a game.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the replay | never |
| `date` | str `YYYY-MM-DD` | the day of the game | never |
| `map_title` | str | the map | never |
| `race` | str | the race of the player: `Terran`, `Zerg`, `Protoss` or `Other` | never |
| `result` | str | `Win`, `Loss`, `Tie` or `Undecided` | never |
| `protocol_version` | u32 | the base build of the replay | never |
| `played_as` | str | the name of the player's account in that game | never |
| `opponent` | str | the opponent (a player, see `player_rows`) | a game without another player |
| `opponent_race` | str | the race of the opponent | never (`Other` without opponent) |

## Query

With the `players` of `player_rows` (its CTEs repeated here, the test runs them):

```sql
WITH rows AS (
    SELECT *,
           CASE WHEN player_toon_id = 0 THEN NULL
                ELSE CONCAT(CAST(player_toon_region AS VARCHAR), '-',
                            CAST(player_toon_realm AS VARCHAR), '-',
                            CAST(player_toon_id AS VARCHAR)) END AS account,
           CASE WHEN STRPOS(player_name, '<sp/>') > 0 THEN SPLIT_PART(player_name, '<sp/>', 2)
                ELSE player_name END AS name
    FROM details
    WHERE player_observe = 0
),
name_games AS (
    SELECT account, name, COUNT(DISTINCT ext_fs_id) AS games
    FROM rows WHERE account IS NOT NULL GROUP BY account, name
),
ranked AS (
    SELECT *, ROW_NUMBER() OVER (PARTITION BY account ORDER BY 0 - CAST(games AS BIGINT), name) AS rank
    FROM name_games
),
account_names AS (SELECT account, name AS account_name FROM ranked WHERE rank = 1),
players AS (
    SELECT r.*, COALESCE(g.grouped_as, a.account_name, r.name) AS player
    FROM rows r
    LEFT JOIN account_names a ON r.account = a.account
    LEFT JOIN player_groups g ON r.account = g.account
),
sides AS (
    SELECT *, ROW_NUMBER() OVER (PARTITION BY ext_fs_id ORDER BY player_name) AS side
    FROM players
    WHERE player = 'Alice'
),
mine AS (
    SELECT ext_fs_id,
           STRFTIME(ext_datetime, '%Y-%m-%d') AS date,
           title AS map_title,
           CASE WHEN player_race = 'Terran' THEN 'Terran'
                WHEN player_race = 'Zerg' THEN 'Zerg'
                WHEN player_race = 'Protoss' THEN 'Protoss'
                ELSE 'Other' END AS race,
           player_result AS result,
           ext_protocol_version AS protocol_version,
           name AS played_as,
           player_team_id
    FROM sides
    WHERE side = 1
),
others AS (
    SELECT ext_fs_id,
           player AS opponent,
           CASE WHEN player_race = 'Terran' THEN 'Terran'
                WHEN player_race = 'Zerg' THEN 'Zerg'
                WHEN player_race = 'Protoss' THEN 'Protoss'
                ELSE 'Other' END AS opponent_race,
           player_team_id AS opponent_team_id
    FROM players
    WHERE player <> 'Alice'
)
SELECT m.ext_fs_id, m.date, m.map_title, m.race, m.result, m.protocol_version, m.played_as,
       o.opponent, o.opponent_race
FROM mine m
LEFT JOIN others o ON m.ext_fs_id = o.ext_fs_id
WHERE o.opponent_team_id IS NULL OR o.opponent_team_id <> m.player_team_id
ORDER BY m.date DESC, m.ext_fs_id DESC, o.opponent
```

## Edge cases

- Teammates are not opponents: a 2v2 gives two rows, one per enemy.
- A game with no other player (a replay with the opponent missing) keeps one row with no
  opponent, so that the game is still counted.
- The opponent is a player (their accounts), the name shown is the one of their most played
  account.

## Performance

`player_rows` twice (Polars shares it), a filter on one player, a join on the game ids of their
games: under 100 ms on the 36,000-game snapshot for a player with 6,000 games.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`), Alice's:

| Game | Day | Opponents |
|---|---|---|
| 1 | 2026-01-15 | Bob |
| 2 | 2026-02-03 | Carol |
| 3 | 2026-02-10 | Dave (as AliceEU) |
| 5 | 2026-02-20 | Carol and Dave (2v2 with Bob) |

Expected: five rows, newest first: game 5 Carol, game 5 Dave, game 3 Dave, game 2 Carol, game 1
Bob; Bob is not an opponent of game 5, he was Alice's teammate.
