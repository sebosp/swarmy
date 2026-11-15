# Opponents of a player

## Purpose

The Opponents table of the Player view: who the player met the most and the record against
them; a click on a record filters the Matches table.

## Inputs

The rows of [`matches`](../matches/README.md): `opponent`, `result`. The player is the parameter
of the query, `'Alice'` below.

## Output

One row per opponent, the most games first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `opponent` | str | the opponent (a player, see `player_rows`) | never |
| `games` | u32 | the games against them, a team game once per enemy | never |
| `wins` | i32 | the games won | never |

## Query

With the `matches` of the Matches query (the CTEs of `player_rows` and `matches` repeated here,
the test runs them):

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
    SELECT ext_fs_id, player_result AS result, player_team_id FROM sides WHERE side = 1
),
others AS (
    SELECT ext_fs_id, player AS opponent, player_team_id AS opponent_team_id
    FROM players
    WHERE player <> 'Alice'
),
matches AS (
    SELECT m.result, o.opponent
    FROM mine m
    JOIN others o ON m.ext_fs_id = o.ext_fs_id
    WHERE o.opponent_team_id <> m.player_team_id
)
SELECT opponent,
       COUNT(*) AS games,
       SUM(CASE WHEN result = 'Win' THEN 1 ELSE 0 END) AS wins
FROM matches
GROUP BY opponent
ORDER BY games DESC, opponent
```

## Edge cases

- A team game counts once for each enemy, so the games of the opponents add up to more than
  the games of the player.
- A game without another player has no opponent and is not here.

## Performance

The Matches query then a group by over the player's games: under 100 ms on the 36,000-game
snapshot for a player with 6,000 games.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`): Alice met Bob in game 1
(won), Carol in games 2 (lost) and 5 (won), Dave in games 3 (tie) and 5 (won). Expected: Carol 2
games 1 win, Dave 2 games 1 win, Bob 1 game 1 win.
