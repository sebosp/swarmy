# Maps of the snapshot

## Purpose

The map field of Map Details (the `list_maps` command): every map of the snapshot
with its number of games, the most played first, so that typing a few letters finds the map.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `title` | one per player per game |

## Output

One row per map title.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `title` | str | the map title as stored in the replays, localized | never |
| `games` | u32 | the games played on the map | never |

## Query

```sql
SELECT title, COUNT(DISTINCT ext_fs_id) AS games
FROM details
GROUP BY title
ORDER BY games DESC, title
```

## Edge cases

- A map played in two languages is two titles (`Pylon LE` and `파일런 - 래더`): the snapshot
  stores the title of the replay.
- Observers are rows of `details` too, a game is counted once (`DISTINCT ext_fs_id`).

## Performance

`details` is a hot table, from memory; a group by over every row of the snapshot, some 80,000
rows for 36,000 games: tens of milliseconds.

## Example

The games of the test (`tests.rs`): three games on Pylon LE (one with an observer), one on
Ephemeron LE. Expected: Pylon LE 3, Ephemeron LE 1.
