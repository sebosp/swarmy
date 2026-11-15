# Maps of the snapshot with their games

## Purpose

The Map List view: every map (each version of it, by the sha256 of its map info) with how many
games were played on it and between which dates, the most played first, filtered by a part of
the map title or of a player name.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `title`, `map_info_sha256`, `ext_datetime`, `cache_handles`, `player_name`, `player_observe` | one per player per game |

The filters are the parameters of the query; below: a player whose name contains `alice`, any
map.

## Output

One row per map version, the most played first, at most 1,000.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `title` | str | the map title as stored in the replays | never |
| `map_info_sha256` | str | the sha256 of the map info: a version of the map | never |
| `min_date` | str `YYYY-MM-DD` | the day of the oldest game | never |
| `max_date` | str `YYYY-MM-DD` | the day of the newest game | never |
| `num_games` | u32 | the games on this version of the map | never |
| `cache_handles` | str | the comma separated cache handles of the newest game, to draw the map | never |

## Query

```sql
WITH games AS (
    SELECT ext_fs_id,
           FIRST(title) AS title,
           FIRST(map_info_sha256) AS map_info_sha256,
           MIN(ext_datetime) AS played_at,
           FIRST(cache_handles) AS cache_handles
    FROM details
    WHERE player_observe = 0
      AND STRPOS(LOWER(player_name), 'alice') > 0
    GROUP BY ext_fs_id
    ORDER BY played_at DESC, ext_fs_id DESC
)
SELECT title, map_info_sha256,
       STRFTIME(MIN(played_at), '%Y-%m-%d') AS min_date,
       STRFTIME(MAX(played_at), '%Y-%m-%d') AS max_date,
       COUNT(*) AS num_games,
       FIRST(cache_handles) AS cache_handles
FROM games
GROUP BY title, map_info_sha256
ORDER BY num_games DESC, title, map_info_sha256
LIMIT 1000
```

The map filter is one more line of the `WHERE`: `AND STRPOS(LOWER(title), 'pylon') > 0`.

## Edge cases

- A map played in two versions (a new season's edit) is two rows with the same title.
- The player filter keeps the games where any player's name contains the text; observers are
  not players.
- The map, its sha256 and its cache handles are the same for every player of a game: the first
  row's are taken.
- The cache handles differ between the games of a map (the mods and skins loaded): the newest
  game's are kept, they draw the map as it is now (the `games` are ordered newest first and
  `FIRST` takes the first of the group).

## Performance

`details` is a hot table, from memory; two group bys over its rows. Tens of milliseconds on the
36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`), the players
whose name contains `alice`: Alice (games 1, 2 and 5), AliceEU (3) and Alice2 (6). Expected:
Pylon LE with 4 games (1, 3, 5, 6) from 2026-01-15 to 2026-03-01, then Ephemeron LE with 1 game
(2) on 2026-02-03.
