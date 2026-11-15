# Unit effectiveness of a player

## Purpose

The Units table of the Player view: for each unit type the player built, how many they made and
how many enemy units (that cost resources) those killed, with the most kills per unit and the
best trade. The counts come from the per-game aggregations of s2protocol-rs.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `game_units` | `ext_fs_id`, `player_name`, `unit`, `born`, `kills`, `value_killed` | one per unit type per player per game |

and the rows of [`player_rows`](../player_rows/README.md): `player`, `ext_fs_id`, `player_name`.
The player is the parameter of the query, `'Alice'` below.

## Output

One row per unit type, by name.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `unit` | str | the unit type, its base name (a sieged tank is a Siege Tank) | never |
| `born` | u32 | the units of the type the player made | never |
| `kills` | u32 | the enemy units that cost resources they killed | never |
| `value_killed` | u64 | the minerals and vespene of those | never |

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
    SELECT ext_fs_id, player_name FROM players WHERE player = 'Alice'
)
SELECT u.unit, SUM(u.born) AS born, SUM(u.kills) AS kills, SUM(u.value_killed) AS value_killed
FROM game_units u
JOIN mine m ON u.ext_fs_id = m.ext_fs_id AND u.player_name = m.player_name
GROUP BY u.unit
ORDER BY u.unit
```

## Edge cases

- The join is on the game and the name as stored in the replay (with the clan tag), the way
  `game_units` names the players: every account of the player, every name it played as.
- Units of another race the player played a few times are here too; the view picks the most
  effective one among the units the player usually builds.
- A snapshot of an older version has no `game_units`: an error asks to Optimize again.

## Performance

`game_units` is a hot table, from memory, joined to the games of the player: under 100 ms on
the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`), Alice's units:

| Game | Units |
|---|---|
| 1 | 20 Marines, 15 kills, 750 resources |
| 2 | 10 Marines, 5 kills, 250; 4 Marauders, 6 kills, 600 |
| 5 | 30 Zerglings, 10 kills, 500 |

Expected: Marauder 4 born, 6 kills, 600; Marine 30 born, 20 kills, 1,000; Zergling 30 born,
10 kills, 500.
