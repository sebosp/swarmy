# Metrics of the recent games of a player

## Purpose

The numbers the Progress view follows over the last games of the player, with a goal for each:
the workers at 06:00, the income at 08:00, the army at 10:00 and the seconds supply blocked. They
come from the stats the game writes every 10 seconds (160 game loops) for each player.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `stats` | `ext_fs_id`, `player_id`, `ext_replay_loop`, `workers_active_count`, `minerals_collection_rate`, `vespene_collection_rate`, `minerals_used_current_army`, `vespene_used_current_army`, `food_made`, `food_used` | one per player every 160 loops |

and the games of [`games`](../games/README.md) (`ext_fs_id`, `player_id`). The player is the
parameter of the query, `'Alice'` below.

## Output

One row per game of the player, by replay id.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the replay | never |
| `player_id` | u8 | the tracker id of the player | never |
| `workers_6` | f64 | the workers at 06:00: the last stats in the 15 seconds before it | the game is shorter, or has no stats |
| `income_8` | f64 | minerals plus vespene collected per minute at 08:00 | the same |
| `army_10` | f64 | minerals plus vespene of the army alive at 10:00 | the same |
| `supply_blocked` | f64 | the seconds supply blocked: the stats with the supply used at the supply made, below 200, times 160 / 22.4 | never, 0 without a block |

## Query

At Faster speed a minute is 1,344 game loops and 15 seconds 336: 06:00 is loop 8064 (window 7728
to 8064), 08:00 loop 10752 (10416 to 10752), 10:00 loop 13440 (13104 to 13440); the stats come
every 160 loops, 7.142857 seconds. The `own` games are the ones of the Recent games query, its
CTEs repeated here.

```sql
WITH named AS (
    SELECT *,
           CASE WHEN STRPOS(player_name, '<sp/>') > 0 THEN SPLIT_PART(player_name, '<sp/>', 2)
                ELSE player_name END AS name
    FROM details
    WHERE player_observe = 0
),
ids AS (
    SELECT DISTINCT ext_fs_id, player_name, player_id FROM game_players
),
sides AS (
    SELECT *, ROW_NUMBER() OVER (PARTITION BY ext_fs_id ORDER BY player_name) AS side
    FROM named
    WHERE name = 'Alice'
),
own AS (
    SELECT s.ext_fs_id, i.player_id
    FROM sides s
    JOIN ids i ON s.ext_fs_id = i.ext_fs_id AND s.player_name = i.player_name
    WHERE s.side = 1
    ORDER BY s.ext_datetime DESC, s.ext_fs_id DESC
    LIMIT 200
),
mine AS (
    SELECT st.*
    FROM stats st
    JOIN own o ON st.ext_fs_id = o.ext_fs_id AND st.player_id = o.player_id
),
window_6 AS (
    SELECT ext_fs_id, player_id, CAST(workers_active_count AS DOUBLE) AS workers_6,
           ROW_NUMBER() OVER (PARTITION BY ext_fs_id, player_id ORDER BY ext_replay_loop DESC) AS newest
    FROM mine WHERE ext_replay_loop > 7728 AND ext_replay_loop <= 8064
),
at_6 AS (SELECT ext_fs_id, player_id, workers_6 FROM window_6 WHERE newest = 1),
window_8 AS (
    SELECT ext_fs_id, player_id,
           CAST(minerals_collection_rate + vespene_collection_rate AS DOUBLE) AS income_8,
           ROW_NUMBER() OVER (PARTITION BY ext_fs_id, player_id ORDER BY ext_replay_loop DESC) AS newest
    FROM mine WHERE ext_replay_loop > 10416 AND ext_replay_loop <= 10752
),
at_8 AS (SELECT ext_fs_id, player_id, income_8 FROM window_8 WHERE newest = 1),
window_10 AS (
    SELECT ext_fs_id, player_id,
           CAST(minerals_used_current_army + vespene_used_current_army AS DOUBLE) AS army_10,
           ROW_NUMBER() OVER (PARTITION BY ext_fs_id, player_id ORDER BY ext_replay_loop DESC) AS newest
    FROM mine WHERE ext_replay_loop > 13104 AND ext_replay_loop <= 13440
),
at_10 AS (SELECT ext_fs_id, player_id, army_10 FROM window_10 WHERE newest = 1),
blocked AS (
    SELECT ext_fs_id, player_id, COUNT(*) * 7.142857142857143 AS supply_blocked
    FROM mine
    WHERE food_made > 0 AND food_used >= food_made AND food_made < 200
    GROUP BY ext_fs_id, player_id
)
SELECT o.ext_fs_id, o.player_id, w.workers_6, i.income_8, a.army_10,
       COALESCE(b.supply_blocked, 0.0) AS supply_blocked
FROM own o
LEFT JOIN at_6 w ON o.ext_fs_id = w.ext_fs_id AND o.player_id = w.player_id
LEFT JOIN at_8 i ON o.ext_fs_id = i.ext_fs_id AND o.player_id = i.player_id
LEFT JOIN at_10 a ON o.ext_fs_id = a.ext_fs_id AND o.player_id = a.player_id
LEFT JOIN blocked b ON o.ext_fs_id = b.ext_fs_id AND o.player_id = b.player_id
ORDER BY o.ext_fs_id
```

## Edge cases

- A game shorter than the minute has no value for it: the chart leaves the point out, the
  average too.
- Supply blocked counts the stats, 7.14 seconds each: a block of 3 seconds counts 0 or 7.
- A supply made of 200 is the cap, not a block.
- A game missing from `game_players` has no `player_id`: no metrics for it.

## Performance

`stats.arrow` is an event file, read from its day partitions for the 200 games (the one query of
the views that reads events; `game_minutes.arrow` has the workers, income and army per minute
but not the supply blocked, the day it has it this query moves there). 200 games: a second on
the 36,000-game snapshot the first time, then the files are in the page cache.

## Example

The games of the test (`tests.rs`): Alice's game 1 has stats at loops 5000 (30 supply made and
used: blocked), 7900 (40 workers; 46 used of 44 made: blocked), 8000 (42 workers), 10500
(1,200 minerals and 300 vespene per minute), 13200 (1,500 and 400 in the army). Expected:
workers_6 = 42, income_8 = 1,500, army_10 = 1,900, supply_blocked = 2 × 7.142857 = 14.29. Game 2
has no stats: no values and 0 seconds blocked; game 3 the same.
