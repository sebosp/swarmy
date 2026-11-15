# Summary of the snapshot

## Purpose

The line of the Home view (and of the Help panel) that says what the snapshot holds: how many
games on how many maps, from which day to which. The sizes, partitions and state of the snapshot
come from its metadata files, not from a query.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `ext_datetime`, `title` | one per player per game |

## Output

One row.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `min_date` | str | `YYYY-MM-DD` of the oldest game | the snapshot is empty |
| `max_date` | str | `YYYY-MM-DD` of the newest game | the snapshot is empty |
| `num_games` | u32 | the games (distinct replay ids) | never |
| `num_maps` | u32 | the distinct map titles | never |

## Query

```sql
SELECT STRFTIME(MIN(ext_datetime), '%Y-%m-%d') AS min_date,
       STRFTIME(MAX(ext_datetime), '%Y-%m-%d') AS max_date,
       COUNT(DISTINCT ext_fs_id) AS num_games,
       COUNT(DISTINCT title) AS num_maps
FROM details
```

## Edge cases

- The replay ids have gaps (replays added later, duplicates moved away): the games are the
  distinct ids, not the largest one.
- Two versions of a map with the same title are one map here, the Map List tells them apart.

## Performance

Four aggregations over the hot `details` table: milliseconds.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): 6 games on
2 maps (Pylon LE, Ephemeron LE), from 2026-01-15 to 2026-03-01.
