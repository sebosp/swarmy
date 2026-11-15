# Protocol versions of the snapshot

## Purpose

The Patch dropdown of the Replay List (part of the `list_replay_filters` command): the protocol
versions (base builds) of the games with how many games each has, the newest first.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `ext_protocol_version` | one per player per game |

## Output

One row per protocol version.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_protocol_version` | u32 | the base build of the replays | never |
| `games` | u32 | the games of that version | never |

## Query

```sql
SELECT ext_protocol_version, COUNT(DISTINCT ext_fs_id) AS games
FROM details
GROUP BY ext_protocol_version
ORDER BY ext_protocol_version DESC
```

## Edge cases

- A snapshot written before the version was stored has one version, 0.
- Observers are rows of `details` too, a game is counted once.

## Performance

`details` is a hot table, from memory; one group by over its rows: a few milliseconds on the
36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`): games 1, 2 and 5 are of
version 95248, games 3, 4 and 6 of 96000. Expected: 96000 with 3 games, then 95248 with 3.
