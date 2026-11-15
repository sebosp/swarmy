# Maps of a race

## Purpose

The maps table of Race Detail: the maps that favour the race, from its 1v1 games against the
other races on each map and the matchups behind the win rate.

## Inputs

The `players` of the snapshot and their 1v1 games (`mine`, see the Query); the tables are the
ones of [`player_games`](../player_games/README.md). The race is the parameter, `'Zerg'` below.

## Output

One row per map and matchup, by map and matchup; `shape` groups the matchups under their map,
keeps the maps with at least 5 games (`RACE_MAP_MIN_GAMES`) and sorts them by win rate.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `title` | str | the map | never |
| `matchup` | str | `ZvP` or `ZvT`, the race first | never |
| `games` | u32 | the 1v1 games of the matchup on the map | never |
| `wins` | i32 | the games the race won | never |

## Query

As the [matchups](../matchups/README.md), by map and without the mirrors:

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
SELECT title,
       CONCAT('Z', 'v', opponent) AS matchup,
       COUNT(*) AS games,
       SUM(CASE WHEN player_result = 'Win' THEN 1 ELSE 0 END) AS wins
FROM mine
WHERE opponent <> 'Z'
GROUP BY title, matchup
ORDER BY title, matchup
```

## Edge cases

- Mirrors are left out of the win rates.
- A map with fewer than 5 games of the race is not shown: too few to say anything.
- A map played in two languages is two titles.

## Performance

As the matchups: tens of milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): the four
non-mirror 1v1 games with a Zerg are on Pylon LE: `ZvT` in games 1, 3 and 6 (one win) and `ZvP`
in game 4 (won). Expected: two rows, `Pylon LE` / `ZvP` 1 game 1 win and `Pylon LE` / `ZvT` 3
games 1 win; `shape` shows no map, 4 games are fewer than 5.
