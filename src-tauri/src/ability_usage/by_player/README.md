# Players ranked by uses per game

## Purpose

Who uses each tracked ability the most, per patch (and over all of them, patch 0): the uses per
game of the race of every player with at least 3 games (`swarmy_common::ABILITY_RANK_MIN_GAMES`),
the most first. The Patch Overview shows the top three, a replay says where its players rank.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| [`game_uses`](../game_uses/README.md) | `patch`, `race`, `ability`, `player_name`, `uses` | one per player game and ability used, twice |
| `both` (the players of the games under their patch and under 0, `game_uses::players_of_both`) | `patch`, `race`, `player_name` | one per player per game, twice |

## Output

One row per patch, ability and player with enough games, by patch and ability, the most uses
per game first, then by name.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `patch` | u32 | the balance data version, or 0 for all | never |
| `race` | str | the race of the ability | never |
| `ability` | str | the ability and command | never |
| `player_name` | str | the player, with the clan tag as in the replay | never |
| `uses` | u32 | the uses over the player's games | never |
| `games` | u32 | the games of the player as the race in the patch, used or not | never |
| `per_game` | f64 | `uses / games` | never |

## Query

The CTEs up to `uses` are those of [`totals`](../totals/README.md).

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
    SELECT b.patch, b.race, u.ability, u.player_name, u.uses
    FROM game_abilities u
    JOIN both b ON u.ext_fs_id = b.ext_fs_id AND u.player_name = b.player_name
    JOIN tracked_abilities t ON u.ability = t.ability AND b.race = t.race
),
player_games AS (
    SELECT patch, race,
           CASE WHEN STRPOS(player_name, '<sp/>') > 0 THEN SPLIT_PART(player_name, '<sp/>', 2)
                ELSE player_name END AS player_name,
           COUNT(*) AS games
    FROM both GROUP BY patch, race, 3
),
summed AS (
    SELECT patch, race, ability,
           CASE WHEN STRPOS(player_name, '<sp/>') > 0 THEN SPLIT_PART(player_name, '<sp/>', 2)
                ELSE player_name END AS player_name,
           SUM(uses) AS uses
    FROM uses
    GROUP BY patch, race, ability, 4
)
SELECT s.patch, s.race, s.ability, s.player_name, s.uses, g.games,
       CAST(s.uses AS DOUBLE) / CAST(g.games AS DOUBLE) AS per_game
FROM summed s
JOIN player_games g ON s.patch = g.patch AND s.race = g.race AND s.player_name = g.player_name
WHERE g.games >= 3
ORDER BY s.patch, s.ability, per_game DESC, s.player_name
```

## Edge cases

- A player with two games of the race in the patch is not ranked, however many uses.
- The games are the player's games as the race: a Random player's Zerg games for a Zerg
  ability.
- The names are without the clan tag: `&lt;TL&gt;<sp/>Alice` and `Alice` are one player, the
  rank follows them when the tag changes.

## Performance

Two group bys and a join: a hundred milliseconds, once per snapshot.

## Example

The games of the test (`tests.rs`, `usage_over_time::test_games::with_abilities`): Bob played
4 Zerg games (1, 4, 5, 6) and used Spawn Larva 20 times in all, 5.0 per game, the only player
with 3 games who used it (Alice has 1 Zerg game). Expected: one row, patch 0, Bob, 20 uses, 4
games, 5.0; nobody in patch 95248 (Bob has 2 games there).
