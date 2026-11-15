# Games of a race by patch

## Purpose

The chart of Race Detail over the patches: the 1v1 games of the race against the other races
and how many it won on each patch, so that a player sees whether a patch helped or hurt it.

## Inputs

The `players` of the snapshot and their 1v1 games (`mine`, see the Query); the tables are the
ones of [`player_games`](../player_games/README.md). The race is the parameter, `'Zerg'` below.

## Output

One row per patch, the oldest first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `patch` | u32 | the patch of the balance data closest to the protocol version of the games | never |
| `games` | u32 | the non-mirror 1v1 games of the race on that patch | never |
| `wins` | i32 | the games the race won | never |

## Query

As the [matchups](../matchups/README.md), the mirrors left out:

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
SELECT patch,
       COUNT(*) AS games,
       SUM(CASE WHEN player_result = 'Win' THEN 1 ELSE 0 END) AS wins
FROM mine
WHERE opponent <> 'Z'
GROUP BY patch
ORDER BY patch
```

## Edge cases

- Mirrors are left out: they always have a winner of the race.
- The patch is the one of the balance data, several protocol versions share one.

## Performance

As the matchups: tens of milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): the four 1v1
games with a Zerg are all against another race, on the patches of the versions 95248 (game 1,
lost) and 96000 (games 3, 4 and 6: a tie and two wins). Expected: the two patches with 1 game 0
wins and 3 games 2 wins, the older first.
