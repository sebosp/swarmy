# Top players of a race

## Purpose

The players table of Race Detail: the ten players with the most games as the race and their
win rate, so that a player finds whose replays of the race to study.

## Inputs

The `players` of the snapshot (see the Query); the tables are the ones of
[`player_games`](../player_games/README.md). The race is the parameter, `'Zerg'` below.

## Output

At most 10 rows, the most games first, then the name.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `player_name` | str | the name as stored in the replay, with its clan tag | never |
| `games` | u32 | the games as the race | never |
| `wins` | i32 | the games won | never |

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
)
SELECT player_name,
       COUNT(*) AS games,
       SUM(CASE WHEN player_result = 'Win' THEN 1 ELSE 0 END) AS wins
FROM players
WHERE race = 'Zerg'
GROUP BY player_name
ORDER BY games DESC, player_name
LIMIT 10
```

## Edge cases

- The name is the one stored in the replay: a player with two accounts or a changed clan tag is
  two rows (the Player view groups the accounts; this table does not yet).
- Team games count as games of the race.

## Performance

`details` from memory, one group by: milliseconds.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): as Zerg, Bob
played games 1, 4, 5 and 6 (won 4, 5, 6), Dave games 3 and 5 (no win), Alice game 5 (won).
Expected: Bob 4 games 3 wins, Dave 2 games 0 wins, Alice 1 game 1 win.
