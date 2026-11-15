# The bases of the players of a replay

## Purpose

Where each player of a replay started and expanded: their townhalls with the game second each
finished, drawn on the map of the Replay view in the player's color, the first one as the start
location. Computed when the replay is opened.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `unit_born` | `ext_fs_id`, `player_name`, `unit_type_name`, `unit_tag_index`, `unit_tag_recycle`, `x`, `y`, `ext_replay_loop` | one per unit born |

The id of the replay is the parameter, `1` below.

## Output

One row per townhall, by time, player and position.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `player_name` | str | the player, with the clan tag as in the replay | never |
| `unit_type_name` | str | `Hatchery`, `CommandCenter` or `Nexus` | never |
| `x` | f32 | the x of the townhall, in map units | never |
| `y` | f32 | its y | never |
| `game_seconds` | u32 | when it finished, 0 for the first townhall | never |

## Query

```sql
WITH births AS (
    SELECT player_name, unit_type_name, CAST(x AS REAL) AS x, CAST(y AS REAL) AS y,
           CAST(CAST(ext_replay_loop AS DOUBLE) / 22.4 AS UINT) AS game_seconds,
           ROW_NUMBER() OVER (PARTITION BY unit_tag_index, unit_tag_recycle ORDER BY ext_replay_loop, unit_tag_index) AS birth
    FROM unit_born
    WHERE ext_fs_id = 1
      AND player_name IS NOT NULL
      AND unit_type_name IN ('Hatchery', 'CommandCenter', 'Nexus')
)
SELECT player_name, unit_type_name, x, y, game_seconds
FROM births
WHERE birth = 1
ORDER BY game_seconds, player_name, x, y
```

## Edge cases

- A Command Center that lifts off and lands again is born again with the same tag: it counts
  once, where and when it was first built.
- An Orbital Command, a Planetary Fortress, a Lair or a Hive are the same unit changing type,
  not a new townhall.
- A player without a townhall (an observer, an arcade race) has no bases.
- A townhall of the neutral player (a map with pre-placed structures) has no player name and is
  left out.

## Performance

A filter of the `unit_born` table of one game: milliseconds.

## Example

The game of the test (`tests.rs`, `replay_detail::test_games::fight`): Alice (Terran) starts at
(30, 30) and lands a Command Center at (30, 60) at 120 s, which lifts and lands again at 400 s;
Bob (Zerg) starts at (70, 70) and finishes a Hatchery at (70, 40) at 100 s. Expected, in order:
Alice's Command Center at 0 s, Bob's Hatchery at 0 s, Bob's Hatchery at 100 s, Alice's Command
Center at 120 s.
