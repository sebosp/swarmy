# Matchups of a race

## Purpose

The matchups table of Race Detail: the 1v1 games of the race against each race and how many it
won, so that a player sees which matchup the race struggles in.

## Inputs

The `players` of the snapshot and their 1v1 games (`mine`, see the Query); the tables are the
ones of [`player_games`](../player_games/README.md). The race is the parameter, `'Zerg'` below.

## Output

One row per matchup.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `matchup` | str | `ZvP`, `ZvT`, `ZvZ`: the race first, the opponent's letter second | never |
| `games` | u32 | the 1v1 games; a ZvZ counts twice, once per side | never |
| `wins` | i32 | the games the race won; half of a mirror's | never |

## Query

The `players` CTE is `data::snapshot_players`; `mine` are the rows of the race in the games of
two players, with the letter of the other player's race.

```sql
WITH ids AS (
    SELECT DISTINCT ext_fs_id, player_name, player_id FROM game_players
),
named AS (
    SELECT d.ext_fs_id, d.player_name, i.player_id,
           CASE WHEN d.player_race = 'Terran' THEN 'Terran'
                WHEN d.player_race = 'Zerg' THEN 'Zerg'
                WHEN d.player_race = 'Protoss' THEN 'Protoss'
                ELSE 'Other' END AS race,
           d.player_result, d.title, v.patch
    FROM details d
    JOIN patch_versions v ON d.ext_protocol_version = v.ext_protocol_version
    LEFT JOIN ids i ON d.ext_fs_id = i.ext_fs_id AND d.player_name = i.player_name
    WHERE d.player_observe = 0
),
players AS (
    SELECT * FROM named WHERE race <> 'Other'
),
counts AS (
    SELECT ext_fs_id, COUNT(*) AS n FROM players GROUP BY ext_fs_id
),
pairs AS (
    SELECT p.*, ROW_NUMBER() OVER (PARTITION BY p.ext_fs_id ORDER BY p.player_name) AS side
    FROM players p JOIN counts c ON p.ext_fs_id = c.ext_fs_id WHERE c.n = 2
),
mine AS (
    SELECT a.ext_fs_id, a.player_name, a.player_result, a.title, a.patch, LEFT(b.race, 1) AS opponent
    FROM pairs a
    JOIN pairs b ON a.ext_fs_id = b.ext_fs_id
    WHERE a.race = 'Zerg' AND a.side <> b.side
)
SELECT CONCAT('Z', 'v', opponent) AS matchup,
       COUNT(*) AS games,
       SUM(CASE WHEN player_result = 'Win' THEN 1 ELSE 0 END) AS wins
FROM mine
GROUP BY matchup
ORDER BY matchup
```

## Edge cases

- Only the games of exactly two players of a StarCraft II race: no team games, no games against
  an arcade race.
- A mirror is counted from both sides: two games, one win.
- Computer players are players: a game against the A.I. is a 1v1. Two players with the same name
  (two A.I. of the same level) are told apart by their side, their row in the game.

## Performance

`details` and `game_players` from memory, a self join of the 1v1 rows on the game: tens of
milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`), the 1v1
games with a Zerg: 1 (Bob lost to a Terran), 3 (Dave tied a Terran), 4 (Bob beat a Protoss
computer), 6 (Bob beat a Terran); game 5 is a 2v2. Expected: `ZvP` 1 game 1 win, `ZvT` 3 games
1 win.
