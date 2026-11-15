# Matchups of a map

## Purpose

The matchups table of Map Details: how many 1v1 games of each matchup were played on the map and
how often the first race won, so that a player sees whether the map favors a race. The view
leaves the mirror matchups out (a mirror always has a winner) and sorts by games.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `title`, `player_name`, `player_race`, `player_result`, `player_observe` | one per player per game |

The map title is the parameter of the query, `'Pylon LE'` below.

## Output

One row per matchup, sorted by label.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `label` | str | the matchup, the race letters sorted: `PvT`, `PvZ`, `TvZ`, and the mirrors `PvP`, `TvT`, `ZvZ` | never |
| `games` | u32 | the 1v1 games of the matchup | never |
| `wins` | i32 | the games the first race won; 0 in a mirror | never |

## Query

The race letter: the Polars plan maps the localized race names of the replays too
(`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`: 테란, Terraner, Терраны...), the SQL shows the English
ones. The two sides of a game are ordered by race letter then name, so that the first side is
the first race of the label and a mirror still has a first side.

```sql
WITH players AS (
    SELECT ext_fs_id, player_name,
           CASE WHEN player_race = 'Terran' THEN 'T'
                WHEN player_race = 'Zerg' THEN 'Z'
                WHEN player_race = 'Protoss' THEN 'P'
                ELSE '?' END AS race,
           player_result = 'Win' AS won
    FROM details
    WHERE title = 'Pylon LE' AND player_observe = 0
),
ordered AS (
    SELECT *,
           ROW_NUMBER() OVER (PARTITION BY ext_fs_id ORDER BY race, player_name) AS side
    FROM players
    WHERE race <> '?'
),
games AS (
    SELECT ext_fs_id,
           MIN(race) AS race,
           MAX(race) AS other_race,
           COUNT(*) AS players,
           SUM(CASE WHEN side = 1 AND won THEN 1 ELSE 0 END) AS first_wins
    FROM ordered
    GROUP BY ext_fs_id
)
SELECT CONCAT(race, 'v', other_race) AS label,
       COUNT(*) AS games,
       SUM(CASE WHEN race <> other_race THEN first_wins ELSE 0 END) AS wins
FROM games
WHERE players = 2
GROUP BY label
ORDER BY label
```

## Edge cases

- Observers are not players (`player_observe = 0`), a 1v1 with a caster is still a 1v1.
- Games with one, three or more players (team games, a replay with a player missing) have no
  matchup and are left out, after the observers and the arcade races are removed.
- The arcade and co-op races (`?`) are not StarCraft II races: their players are left out, so a
  game of a Terran against a commander is not a 1v1.
- A mirror (`ZvZ`) counts its games and 0 wins: the view hides it, the Replay List link still
  opens its games.
- A tie or an undecided game counts as a game without a win for the first race.

## Performance

`details` is a hot table, from memory. The title filter comes first, then a sort and a group by
over the games of one map: milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`), on Pylon LE unless said otherwise:

| Game | Players | Matchup | First race won |
|---|---|---|---|
| 1 | Alice (Terran) beats Bob (Zerg) | TvZ | yes |
| 2 | Carol (Protoss) beats Alice (Terran) | PvT | yes |
| 3 | Bob (Zerg) beats Dave (Zerg) | ZvZ | mirror, 0 |
| 4 | on Ephemeron LE: Alice (Terran) beats Carol (Protoss) | another map | left out |
| 5 | Alice (Terran) beats Carol (Protoss), Caster observes | PvT | no |

Expected: `PvT` 2 games, 1 win; `TvZ` 1 game, 1 win; `ZvZ` 1 game, 0 wins.
