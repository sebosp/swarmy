# Players of the snapshot

## Purpose

The player fields of the Player, Progress and Replay List views (the `list_players` command):
every player of the snapshot with their games and wins, the most games first, so that typing a
few letters finds the player.

## Inputs

The rows of [`player_rows`](../player_rows/README.md): `player`, `ext_fs_id`, `player_result`.

## Output

One row per player.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `player` | str | the player (see `player_rows`) | never |
| `games` | u32 | the games played | never |
| `wins` | i32 | the games won | never |

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
)
SELECT player,
       COUNT(DISTINCT ext_fs_id) AS games,
       SUM(CASE WHEN player_result = 'Win' THEN 1 ELSE 0 END) AS wins
FROM players
WHERE player <> ''
GROUP BY player
ORDER BY games DESC, player
```

## Edge cases

- Computer players are players (`A.I. 1 (Very Easy)`), with their games.
- A player with two accounts in one game (a 2v2 with oneself) counts the game once.

## Performance

`player_rows` then one group by: tens of milliseconds on the 36,000-game snapshot, 16,000
players.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`). Expected: Alice 4 games
(1, 2, 3 as AliceEU, 5), 2 wins; Bob 4 games (1, 4, 5, 6), 3 wins; Carol 2 games 1 win; Dave 2
games 0 wins; A.I. 1 (Very Easy) 1 game 0 wins; Alice2 1 game 0 wins.
