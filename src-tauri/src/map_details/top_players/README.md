# Top players of a map

## Purpose

The "Top players" table of Map Details: the ten players with the most games on the map and
their win rate there, so that a player finds who to study on it.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `title`, `player_name`, `player_result`, `player_observe` | one per player per game |

The map title is the parameter of the query, `'Pylon LE'` below.

## Output

At most 10 rows, the most games first, then the most wins, then the name.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `label` | str | the player name as stored in the replay, with its clan tag | never |
| `games` | u32 | the games of the player on the map | never |
| `wins` | i32 | the games won | never |

## Query

```sql
SELECT player_name AS label,
       COUNT(*) AS games,
       SUM(CASE WHEN player_result = 'Win' THEN 1 ELSE 0 END) AS wins
FROM details
WHERE title = 'Pylon LE' AND player_observe = 0
GROUP BY player_name
ORDER BY games DESC, wins DESC, label
LIMIT 10
```

## Edge cases

- Observers (`player_observe = 1`) watched the game, they did not play it: left out.
- The name is the one stored in the replay: a player with two accounts or a changed clan tag is
  two rows (the Player view groups the accounts, `data::details_with_players`; this table does
  not yet).
- Computer players are players here, a game against the A.I. counts.
- Ties and undecided games are games without a win.

## Performance

`details` is a hot table, from memory; the title filter comes before the group by.
Milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`), on Pylon LE unless said otherwise:

| Game | Players |
|---|---|
| 1 | Alice (Terran) beats Bob (Zerg) |
| 2 | Carol (Protoss) beats Alice (Terran) |
| 3 | Bob (Zerg) beats Dave (Zerg) |
| 4 | on Ephemeron LE: Alice beats Carol, left out |
| 5 | Alice (Terran) beats Carol (Protoss), Caster observes |

Expected: Alice 3 games, 2 wins; Bob 2 games, 1 win; Carol 2 games, 1 win (after Bob, by name);
Dave 1 game, 0 wins. Caster is not listed.
