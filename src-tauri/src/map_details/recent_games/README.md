# Recent games of a map

## Purpose

The "Recent games" table of Map Details: the 20 newest games on the map with their players and
who won, so that a player opens a fresh game on the map in one click.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `title`, `ext_datetime`, `player_team_id`, `player_name`, `player_result`, `player_race`, `player_observe`, `ext_fs_file_name`, `ext_fs_sha256` | one per player per game |
| `game_players` | `ext_fs_id`, `game_seconds` | one per player per game |

The map title is the parameter of the query, `'Pylon LE'` below.

## Output

One row per player of the 20 newest games: the newest game first, within a game the teams in
order then the names. `shape` groups the rows into one `ReplayListEntry` per game.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the replay | never |
| `date` | str `YYYY-MM-DD` | the day of the game | never |
| `player_team_id` | u8 | the team of the player | never |
| `player_name` | str | the name as stored in the replay | never |
| `player_result` | str | `Win`, `Loss`, `Tie` or `Undecided` | never |
| `player_race` | str | the race as stored in the replay, localized | never |
| `file_name` | str | the path of the replay file | never |
| `sha256` | str | the sha256 of the replay file, the notes are kept by it | never |
| `game_seconds` | u32 | the length of the game | the game is not in `game_players` |

## Query

```sql
WITH newest AS (
    SELECT ext_fs_id, MIN(ext_datetime) AS played_at
    FROM details
    WHERE title = 'Pylon LE'
    GROUP BY ext_fs_id
    ORDER BY played_at DESC, ext_fs_id DESC
    LIMIT 20
),
durations AS (
    SELECT ext_fs_id, MIN(game_seconds) AS game_seconds
    FROM game_players
    GROUP BY ext_fs_id
)
SELECT d.ext_fs_id,
       STRFTIME(n.played_at, '%Y-%m-%d') AS date,
       d.player_team_id,
       d.player_name,
       d.player_result,
       d.player_race,
       d.ext_fs_file_name AS file_name,
       d.ext_fs_sha256 AS sha256,
       g.game_seconds
FROM details d
JOIN newest n ON d.ext_fs_id = n.ext_fs_id
LEFT JOIN durations g ON d.ext_fs_id = g.ext_fs_id
WHERE d.player_observe = 0
ORDER BY date DESC, d.ext_fs_id DESC, d.player_team_id, d.player_name
```

## Edge cases

- Observers are not listed with the players.
- The 20 games are chosen before the observers are removed, so a game of observers only (none
  exist in practice) would be a game without players.
- Two games of the same day are ordered by replay id, the ids follow the time of the games.
- A game missing from `game_players` has a length of 0 in the entry.
- The entries carry no rating, tags, notes or best ability rank (the Replay List computes
  those from the personal database and the ability ranks): Map Details does not show them.

## Performance

`details` and `game_players` are hot tables, from memory. The 20 games are chosen first, the
players are joined to them: the work after the title filter is bounded by 20 games.
Milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`), on Pylon LE unless said otherwise:

| Game | Day | Players |
|---|---|---|
| 1 | 2026-01-15 | Alice (Terran) beats Bob (Zerg) |
| 2 | 2026-01-16 | Carol (Protoss) beats Alice (Terran) |
| 3 | 2026-01-17 | Bob (Zerg) beats Dave (Zerg) |
| 4 | 2026-01-18 | on Ephemeron LE: Alice beats Carol, left out |
| 5 | 2026-01-19 | Alice (Terran) beats Carol (Protoss), Caster observes |

Expected: games 5, 3, 2, 1 in this order; game 5 lists Alice and Carol (not Caster), with Alice
`Win` and Carol `Loss`; game 2 lists Carol first (team 1) then Alice.
