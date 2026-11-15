# Units of a race

## Purpose

The units table of Race Detail: which army units the players of the race make, in what share of
their games, how many per game, their kills and the win rate of the games they appear in, from
the per-game aggregations of s2protocol-rs.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `game_units` | `ext_fs_id`, `player_name`, `unit`, `born`, `kills` | one per unit type per player per game |
| `race_units` | `unit`, `base` | the army units of the race in the balance data (cost and army supply), with their variant names; a frame swarmy builds, `race_detail::units::race_units` |

and the `players` of the snapshot (the tables of [`player_games`](../player_games/README.md)).
The race is the parameter, `'Zerg'` below.

## Output

One row per unit type (as `game_units` names it), the most games first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `unit` | str | the unit type; `shape` shows its base name | never |
| `games` | i32 | the player-games the unit was made in | never |
| `wins` | i32 | the ones the player won | never |
| `born` | u32 | the units made in all the games | never |
| `kills` | u32 | the enemy units that cost resources they killed | never |

## Query

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
mine AS (
    SELECT ext_fs_id, player_name, player_result FROM players WHERE race = 'Zerg'
),
made AS (
    SELECT u.unit, u.born, u.kills, m.player_result
    FROM game_units u
    JOIN race_units r ON u.unit = r.unit
    JOIN mine m ON u.ext_fs_id = m.ext_fs_id AND u.player_name = m.player_name
)
SELECT unit,
       SUM(CASE WHEN born > 0 THEN 1 ELSE 0 END) AS games,
       SUM(CASE WHEN born > 0 AND player_result = 'Win' THEN 1 ELSE 0 END) AS wins,
       SUM(born) AS born,
       SUM(kills) AS kills
FROM made
GROUP BY unit
HAVING SUM(CASE WHEN born > 0 THEN 1 ELSE 0 END) > 0
ORDER BY games DESC, unit
```

## Edge cases

- Only the army units of the race that cost resources (no workers, no buildings, no larvae); a
  variant (burrowed, sieged) is a row of its own in `game_units`, `shape` shows the base name.
- A unit type with kills but never made (a mind-controlled one) is left out.
- Team games count as games of the race.

## Performance

`game_units` and `details` from memory, a semi join on the unit names and a join on the players
of the race: tens of milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): the only
Zerg with units is Alice in game 5 (won), 30 Zerglings with 10 kills. Expected: one row,
`Zergling`, 1 game, 1 win, 30 born, 10 kills.
