# Games of a protocol version

## Purpose

The games a replay is compared with for its timings: the games of the snapshot played with the
same protocol version (the same base build of the game). Their ids are the parameter of
[`upgrade_averages`](../upgrade_averages/README.md) and
[`expansion_averages`](../expansion_averages/README.md): as a literal list they are pushed down
to the scans of the large tables.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `ext_protocol_version` | one per player per game |

The protocol version is the parameter, 95248 below.

## Output

One row per game, by id.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the game | never |

## Query

```sql
SELECT DISTINCT ext_fs_id
FROM details
WHERE ext_protocol_version = 95248
ORDER BY ext_fs_id
```

## Edge cases

- A version with no games: no rows, the averages are empty and the replay shows none.
- The protocol version, not the balance patch: two versions of one patch are compared apart.

## Performance

A filter of the hot `details` table: milliseconds.

## Example

The games of the test (`tests.rs`, `replay_detail::test_games::patch_games`): games 1 to 6 are
of version 95248, game 7 of 96000. Expected: 1, 2, 3, 4, 5, 6.
