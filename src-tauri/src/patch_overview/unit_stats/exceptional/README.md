# Exceptional games of a unit

## Purpose

The "Exceptional games" table of the Unit Overview: the players whose units killed far more
than usual, two standard deviations above the average kills per unit
([`kills_per_unit`](../kills_per_unit/README.md)), with the game to open: a replay worth
watching to see the unit used well.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| [`kills_per_unit`](../kills_per_unit/README.md) | `ext_fs_id`, `player_name`, `born`, `kills`, `value_killed`, `kills_per_unit` | one per player and game with enough of the unit |
| [`games`](../games/README.md) | `ext_fs_id`, `date`, `title`, `ext_fs_file_name`, `ext_protocol_version` | one per game |

The unit, the minimum made and the threshold are the parameters, `'Marine'`, 3 and 4.924 below
(the threshold of the example of `kills_per_unit`).

## Output

The best first, at most 20 (`swarmy_common::EXCEPTIONAL_GAMES_LIMIT`).

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the game | never |
| `player_name` | str | the player | never |
| `born` | u32 | the units made | never |
| `kills` | u32 | the enemy units they killed | never |
| `value_killed` | u64 | the resources of those | never |
| `kills_per_unit` | f64 | `kills / born` | never |
| `date` | str | `YYYY-MM-DD` of the game | never |
| `title` | str | the map | never |
| `ext_fs_file_name` | str | the replay file | never |
| `ext_protocol_version` | u32 | the base build of the replay | never |

## Query

```sql
WITH made AS (
    SELECT ext_fs_id, player_name, born, kills, value_killed,
           CAST(kills AS DOUBLE) / CAST(born AS DOUBLE) AS kills_per_unit
    FROM game_units
    WHERE unit = 'Marine' AND born >= 3
),
games AS (
    SELECT ext_fs_id,
           STRFTIME(MIN(ext_datetime), '%Y-%m-%d') AS date,
           FIRST(title) AS title,
           FIRST(ext_fs_file_name) AS ext_fs_file_name,
           FIRST(ext_protocol_version) AS ext_protocol_version
    FROM details
    GROUP BY ext_fs_id
)
SELECT made.ext_fs_id, made.player_name, made.born, made.kills, made.value_killed, made.kills_per_unit,
       games.date, games.title, games.ext_fs_file_name, games.ext_protocol_version
FROM made JOIN games ON made.ext_fs_id = games.ext_fs_id
WHERE made.kills_per_unit >= 4.924 AND made.kills > 0
ORDER BY made.kills_per_unit DESC, made.kills DESC, made.ext_fs_id, made.player_name
LIMIT 20
```

## Edge cases

- One player only: the threshold is their own mean, their game is exceptional if they killed
  anything; the view says what the threshold is.
- A game with no kills is never exceptional, even at a threshold of 0.
- Ties by kills per unit: the one with more kills first, then by game and player.

## Performance

A filter of `game_units` joined with the games: tens of milliseconds.

## Example

The games of the test (`tests.rs`, `patch_overview::unit_stats::test_games::marine_makers`):
Fiona in game 6 (10 Marines, 50 kills, 5.0 per Marine) is the only exceptional game; the others
killed 0.5 per Marine and Gina's 2 Marines are not compared.
