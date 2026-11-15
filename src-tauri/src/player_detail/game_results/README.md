# Game results of a player

## Purpose

The header of the Player view (games, wins, ties, the races played) and its two charts, the games
and wins per month and per protocol version: one row per game of the player, counted by `shape`.

## Inputs

The rows of [`player_rows`](../player_rows/README.md): `player`, `ext_fs_id`, `player_name`,
`ext_datetime`, `player_race`, `player_result`, `ext_protocol_version`, `name`. The player is the
parameter of the query, `'Alice'` below.

## Output

One row per game of the player, the oldest first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the replay | never |
| `date` | str `YYYY-MM-DD` | the day of the game | never |
| `month` | str `YYYY-MM` | the month of the game, for the chart | never |
| `race` | str | `Terran`, `Zerg`, `Protoss` or `Other` (an arcade race) | never |
| `result` | str | `Win`, `Loss`, `Tie` or `Undecided` | never |
| `protocol_version` | u32 | the base build of the replay, for the chart | never |
| `played_as` | str | the name of the account in that game, without the clan tag | never |

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
mine AS (
    SELECT *, ROW_NUMBER() OVER (PARTITION BY ext_fs_id ORDER BY player_name) AS side
    FROM players
    WHERE player = 'Alice'
)
SELECT ext_fs_id,
       STRFTIME(ext_datetime, '%Y-%m-%d') AS date,
       STRFTIME(ext_datetime, '%Y-%m') AS month,
       CASE WHEN player_race = 'Terran' THEN 'Terran'
            WHEN player_race = 'Zerg' THEN 'Zerg'
            WHEN player_race = 'Protoss' THEN 'Protoss'
            ELSE 'Other' END AS race,
       player_result AS result,
       ext_protocol_version AS protocol_version,
       name AS played_as
FROM mine
WHERE side = 1
ORDER BY ext_fs_id
```

The Polars plan maps the localized race names too (`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`).

## Edge cases

- A player with two of their accounts in one game (a 2v2 with oneself) is one game, the side of
  the first account by name.
- Ties and undecided games are games, not wins; `shape` counts the ties for the header.
- A snapshot written before the protocol version was stored has `protocol_version` 0: one bar
  "0" in the chart, Optimize again to fix it. The replays are never read for it.

## Performance

`player_rows` then a filter on one player, a sort and a unique over their games: tens of
milliseconds on the 36,000-game snapshot, also for a player with 6,000 games.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`), Alice's are:

| Game | Day | Alice | Result | Version |
|---|---|---|---|---|
| 1 | 2026-01-15 | `&lt;TL&gt;<sp/>Alice`, Terran | Win | 95248 |
| 2 | 2026-02-03 | Alice, Terran | Loss | 95248 |
| 3 | 2026-02-10 | AliceEU (grouped under Alice), Terran | Tie | 96000 |
| 5 | 2026-02-20 | Alice, Zerg, 2v2 with Bob | Win | 95248 |

Expected: 4 games, 2 wins, 1 tie; Terran 3 games 1 win, Zerg 1 game 1 win; 2026-01 1 game 1 win,
2026-02 3 games 1 win; version 95248 3 games 2 wins, 96000 1 game 0 wins.
