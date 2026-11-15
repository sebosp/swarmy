# Games of the snapshot

## Purpose

One row per game with what the Unit Overview says about a game: its month, its day, the map,
the replay file and the protocol version. [`by_month`](../by_month/README.md) counts them and
[`exceptional`](../exceptional/README.md) joins them to the players that made the unit; the
`details` table has a row per player, this query folds them.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `ext_datetime`, `title`, `ext_fs_file_name`, `ext_protocol_version` | one per player per game |

## Output

One row per game, by id.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the id of the replay | never |
| `month` | str | `YYYY-MM` of the game | never |
| `date` | str | `YYYY-MM-DD` of the game | never |
| `title` | str | the map | never |
| `ext_fs_file_name` | str | the replay file, to open it | never |
| `ext_protocol_version` | u32 | the base build of the replay | never |

## Query

```sql
SELECT ext_fs_id,
       STRFTIME(MIN(ext_datetime), '%Y-%m') AS month,
       STRFTIME(MIN(ext_datetime), '%Y-%m-%d') AS date,
       FIRST(title) AS title,
       FIRST(ext_fs_file_name) AS ext_fs_file_name,
       FIRST(ext_protocol_version) AS ext_protocol_version
FROM details
GROUP BY ext_fs_id
ORDER BY ext_fs_id
```

## Edge cases

- The rows of a game have the same date, map and file: the first is taken.
- The games of observers count as games too: a game is a game.

## Performance

A group by over the hot `details` table: milliseconds.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): six rows,
game 1 on `2026-01` / `2026-01-15`, Pylon LE, `1.SC2Replay`, version 95248; game 2 on Ephemeron
LE.
