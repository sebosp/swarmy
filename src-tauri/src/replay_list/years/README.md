# Years of the snapshot

## Purpose

The time frame dropdown of the Replay List (part of the `list_replay_filters` command): the
years with games, the newest first, so that the list is filtered to one year in a click.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_datetime` | one per player per game |

## Output

One row per year.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `year` | i32 | a year with at least one game | never |

## Query

```sql
SELECT DISTINCT DATE_PART('year', ext_datetime) AS year
FROM details
ORDER BY year DESC
```

## Edge cases

- The year is the one of the local time written in the replay, as the rest of swarmy.

## Performance

`details` is a hot table, from memory; a unique over one column: a millisecond.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`) were all played in 2026.
Expected: one row, 2026.
