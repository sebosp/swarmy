# Use of the tracked abilities per patch

## Purpose

The table of the ability usage: for each patch (and for all of them, patch 0) and each tracked
ability, in how many player games of its race it was used, how many times, out of how many games
of the race. The Patch Overview shows the uses per game of each patch, a replay is compared with
the games of its patch.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| [`game_uses`](../game_uses/README.md) | `patch`, `race`, `ability`, `uses` | one per player game and ability used, twice |
| `both` (the players of the games under their patch and under 0, `game_uses::players_of_both`) | `patch`, `race` | one per player per game, twice |

## Output

One row per patch, race and ability used, by patch, race and ability.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `patch` | u32 | the balance data version, or 0 for all | never |
| `race` | str | the race of the ability | never |
| `ability` | str | the ability and command | never |
| `games_used` | u32 | the player games it was used in | never |
| `uses` | u32 | the uses in those | never |
| `player_games` | u32 | the games of the players of the race in the patch | never |

## Query

The CTEs up to `both` are those of [`game_uses`](../game_uses/README.md).

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
both AS (
    SELECT ext_fs_id, player_name, race, patch FROM players
    UNION ALL
    SELECT ext_fs_id, player_name, race, CAST(0 AS UINT) AS patch FROM players
),
uses AS (
    SELECT b.patch, b.race, u.ability, u.uses
    FROM game_abilities u
    JOIN both b ON u.ext_fs_id = b.ext_fs_id AND u.player_name = b.player_name
    JOIN tracked_abilities t ON u.ability = t.ability AND b.race = t.race
),
race_games AS (
    SELECT patch, race, COUNT(*) AS player_games FROM both GROUP BY patch, race
)
SELECT u.patch, u.race, u.ability, COUNT(*) AS games_used, SUM(u.uses) AS uses, r.player_games
FROM uses u
JOIN race_games r ON u.patch = r.patch AND u.race = r.race
GROUP BY u.patch, u.race, u.ability, r.player_games
ORDER BY u.patch, u.race, u.ability
```

## Edge cases

- An ability nobody used in a patch has no row: the view shows nothing for it.
- The player games of a race count every game of its players, used or not: the uses per game
  are `uses / player_games`.

## Performance

A group by of the uses and of the players: a hundred milliseconds, once per snapshot.

## Example

The games of the test (`tests.rs`, `usage_over_time::test_games::with_abilities`): the Zerg
played 7 player games (6 of patch 95248... no: 3 of 95248 and 4 of 95841), Spawn Larva was used
in 2 of them, 35 times, both of patch 95248. Expected rows: patch 0 Terran MULE 1 game 5 uses of
4 Terran games; patch 0 Zerg Spawn Larva 2 games 35 uses of 7; patch 95248 Terran MULE 1, 5, of
2; patch 95248 Zerg Spawn Larva 2, 35, of 4.
