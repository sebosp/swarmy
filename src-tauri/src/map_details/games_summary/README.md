# Games summary of a map

## Purpose

The first line of Map Details: how many games were played on the map, between which dates, and
how long they last on average, so that a player knows how much the rest of the view rests on.
It also finds the map cache to draw (the cache handles of the newest game).

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `title`, `ext_datetime`, `cache_handles` | one per player per game |
| `game_players` | `ext_fs_id`, `game_seconds` | one per player per game |

The map title is the parameter of the query, `'Pylon LE'` below.

## Output

One row.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `games` | u32 | the games played on the map | never, 0 without games |
| `min_date` | str `YYYY-MM-DD` | the day of the oldest game | no games |
| `max_date` | str `YYYY-MM-DD` | the day of the newest game | no games |
| `avg_duration_seconds` | f64 | the mean length of the games, in seconds at Faster speed | no games, or none of them in `game_players` |
| `cache_handles` | str | the comma separated cache handles of the newest game | no games |

## Query

```sql
WITH games AS (
    SELECT ext_fs_id, MIN(ext_datetime) AS played_at, MIN(cache_handles) AS cache_handles
    FROM details
    WHERE title = 'Pylon LE'
    GROUP BY ext_fs_id
),
ranked AS (
    SELECT *, ROW_NUMBER() OVER (ORDER BY played_at DESC, ext_fs_id DESC) AS newest
    FROM games
),
durations AS (
    SELECT ext_fs_id, MIN(game_seconds) AS game_seconds
    FROM game_players
    GROUP BY ext_fs_id
)
SELECT COUNT(*) AS games,
       STRFTIME(MIN(played_at), '%Y-%m-%d') AS min_date,
       STRFTIME(MAX(played_at), '%Y-%m-%d') AS max_date,
       AVG(game_seconds) AS avg_duration_seconds,
       MAX(CASE WHEN newest = 1 THEN cache_handles END) AS cache_handles
FROM ranked
LEFT JOIN durations ON ranked.ext_fs_id = durations.ext_fs_id
```

## Edge cases

- Observers count as rows of `details` but a game is counted once (`GROUP BY ext_fs_id`).
- A game missing from `game_players` (a snapshot written before the aggregations) has no length
  and is left out of the average, not of the count.
- Two versions of a map share a title and may have different cache handles: the ones of the
  newest game are used, so that the newest version of the map is drawn.

## Performance

`details` and `game_players` are hot tables, read from memory; the title filter is applied before
the group by. Milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`):

| Game | Day | Map | Players | Length |
|---|---|---|---|---|
| 1 | 2026-01-15 | Pylon LE | Alice (Terran) beats Bob (Zerg) | 600 s |
| 2 | 2026-01-16 | Pylon LE | Carol (Protoss) beats Alice (Terran) | 900 s |
| 3 | 2026-01-17 | Pylon LE | Bob (Zerg) beats Dave (Zerg) | 300 s |
| 4 | 2026-01-18 | Ephemeron LE | Alice (Terran) beats Carol (Protoss) | 600 s |
| 5 | 2026-01-19 | Pylon LE, cache `pylon-v2` | Alice (Terran) beats Carol (Protoss), Caster observes | 600 s |

Expected for Pylon LE: 4 games, from 2026-01-15 to 2026-01-19, (600 + 900 + 300 + 600) / 4 =
600 s on average, cache handles `pylon-v2` (game 5 is the newest).
