# Recent games of the player of Progress

## Purpose

The games the Progress view follows: the last 200 games of the player (the Main Player Name of
Config, or the player chosen above the charts), the oldest first, with their opponents and
whether they were computers, so that the view can keep only the games against humans or
computers. The metrics of each game come from the [`metrics`](../metrics/README.md) query.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `player_name`, `player_observe`, `player_team_id`, `player_race`, `player_result`, `player_control`, `title`, `ext_datetime` | one per player per game |
| `game_players` | `ext_fs_id`, `player_name`, `player_id` | one per player per game |

The player is the parameter of the query, by the name without the clan tag: `'Alice'` below.

## Output

One row per game, the oldest first, at most 200.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the replay | never |
| `player_id` | u8 | the tracker id of the player in the game, the one of the stats | the game is not in `game_players` |
| `team_id` | u8 | the team of the player | never |
| `race` | str | `Terran`, `Zerg`, `Protoss` or `Other` | never |
| `result` | str | `Win`, `Loss`, `Tie` or `Undecided` | never |
| `title` | str | the map | never |
| `date` | str `YYYY-MM-DD HH:MM` | when the game was played | never |
| `opponents` | list of str | the players of the other teams, as stored | no other team |
| `computers` | i32 | how many of the opponents are the game's A.I. | no other team |

## Query

```sql
WITH named AS (
    SELECT *,
           CASE WHEN STRPOS(player_name, '<sp/>') > 0 THEN SPLIT_PART(player_name, '<sp/>', 2)
                ELSE player_name END AS name
    FROM details
    WHERE player_observe = 0
),
ids AS (
    SELECT DISTINCT ext_fs_id, player_name, player_id FROM game_players
),
sides AS (
    SELECT *, ROW_NUMBER() OVER (PARTITION BY ext_fs_id ORDER BY player_name) AS side
    FROM named
    WHERE name = 'Alice'
),
own AS (
    SELECT s.ext_fs_id, i.player_id, s.player_team_id AS team_id,
           CASE WHEN s.player_race = 'Terran' THEN 'Terran'
                WHEN s.player_race = 'Zerg' THEN 'Zerg'
                WHEN s.player_race = 'Protoss' THEN 'Protoss'
                ELSE 'Other' END AS race,
           s.player_result AS result,
           s.title,
           STRFTIME(s.ext_datetime, '%Y-%m-%d %H:%M') AS date
    FROM sides s
    LEFT JOIN ids i ON s.ext_fs_id = i.ext_fs_id AND s.player_name = i.player_name
    WHERE s.side = 1
    ORDER BY s.ext_datetime DESC, s.ext_fs_id DESC
    LIMIT 200
),
opponents AS (
    SELECT d.ext_fs_id,
           ARRAY_AGG(d.player_name) AS opponents,
           SUM(CASE WHEN d.player_control = 3 THEN 1 ELSE 0 END) AS computers
    FROM named d
    JOIN own o ON d.ext_fs_id = o.ext_fs_id
    WHERE d.player_team_id <> o.team_id
    GROUP BY d.ext_fs_id
)
SELECT o.*, p.opponents, p.computers
FROM own o
LEFT JOIN opponents p ON o.ext_fs_id = p.ext_fs_id
ORDER BY o.date, o.ext_fs_id
```

## Edge cases

- The player is a name, not the accounts of the Player view: the Main Player Name of Config is
  typed by the user, and their clan tag changes.
- Observers are neither the player nor opponents.
- Teammates are not opponents (a 2v2 lists the two enemies).
- A computer opponent has `player_control` 3; the view filters humans and computers by it.
- A game missing from `game_players` has no `player_id` and no metrics.

## Performance

`details` and `game_players` are hot tables, from memory: a filter on one name, a sort of their
games and a group by over the opponents of 200 games. A few milliseconds.

## Example

The games of the test (`tests.rs`): Alice played three games on Pylon LE, in this order: against
Bob (won, 2026-01-15), against the computer A.I. 1 (Very Easy) (won, 2026-01-20) and with Bob
against Carol and Dave (lost, 2026-02-01). Expected: three rows in that order; the opponents
`[Bob]` with 0 computers, `[A.I. 1 (Very Easy)]` with 1, `[Carol, Dave]` with 0.
