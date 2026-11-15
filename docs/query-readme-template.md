# <Query name, as the panel calls it>

<!-- Copy to src-tauri/src/<view>/<query>/README.md. It is the rustdoc of the query file
     (`#![doc = include_str!("<query>/README.md")]`) and GitHub shows it in the directory. Written
     for a person who knows SQL and StarCraft II, not the code. The contract test
     (data/contract.rs) needs the headings Purpose, Inputs, Output, Query and Example, and either
     a ```sql fence or the sentence "SQL: not expressible" under Query. -->

## Purpose

One or two sentences: the question the panel answers and for whom. "The last games played on
the map, newest first, so that a player sees who plays it and how long the games last."

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details.arrow` | `ext_fs_id`, `title`, `player_name`, `player_result`, `player_observe`, `ext_datetime` | one per player per game |
| `game_players.arrow` | `ext_fs_id`, `game_seconds` | one per player per game |

## Output

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the replay | never |
| `date` | str `YYYY-MM-DD` | the day of the game | never |
| ... | | | |

Sorted by ..., at most N rows.

## Query

The SQL below is run by the tests on the same games as the Polars plan, the results must be
equal. (Or: `SQL: not expressible`, because ..., and the test asserts the expected frame.)

```sql
SELECT ...
FROM details
WHERE player_observe = 0
GROUP BY ...
ORDER BY ...
```

## Edge cases

- Observers (`player_observe = 1`) are not players: left out.
- Computer players (`player_control = 3`): ...
- Ties, games without a result, arcade races, duplicate replays: ...

## Performance

Which files are scanned (the hot `details` from memory, the day partitions of `stats.arrow`...),
what the filter pushes down to the scan, the cost on the 36,000-game snapshot (from the bench).

## Example

The games of the test (`data::fixtures`), and the answer a reader can recompute:

| Game | Map | Players | Result |
|---|---|---|---|
| 1 | Pylon LE | Alice (Terran) vs Bob (Zerg) | Alice wins |
| 2 | Pylon LE | Alice (Terran) vs Carol (Protoss) | Carol wins |

Expected: Alice 2 games, 1 win → 50 %; Bob 1 game, 0 wins; Carol 1 game, 1 win.
