# Uses of the tracked abilities per player game

## Purpose

The base of the ability usage of the snapshot (the Patch Overview, Race Detail and the ability
comparisons of a replay): how many times each player used each tracked ability (Abduct, Fungal
Growth, Yamato Cannon...) in each game, under the patch of the game and once more under patch 0,
the games of all the patches. [`totals`](../totals/README.md) and
[`by_player`](../by_player/README.md) add the rows up; the rows themselves rank a game among the
games of its patch.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `players` (`data::snapshot_players`: `details`, `game_players`, `patch_versions`) | `ext_fs_id`, `player_name`, `race`, `patch` | one per player per game, no observers, StarCraft II races only |
| `game_abilities` | `ext_fs_id`, `player_name`, `ability`, `uses` | one per player, game and ability used |
| `tracked_abilities` | `ability`, `race` | the tracked abilities and the race that casts them; a frame swarmy builds, `ability_usage::game_uses::tracked_abilities` |

## Output

One row per player game and ability used, twice (the patch and patch 0), by patch, race,
ability, game and player.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `patch` | u32 | the balance data version of the game, or 0 for all the patches | never |
| `race` | str | the race of the player | never |
| `ability` | str | the ability and command, `SpawnLarva.Execute` | never |
| `ext_fs_id` | u64 | the game | never |
| `player_name` | str | the player, with the clan tag as in the replay | never |
| `uses` | u32 | the uses in the game | never |

## Query

The `players` CTE is `data::snapshot_players`; `both` has the players under their patch and
under patch 0.

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
)
SELECT b.patch, b.race, u.ability, u.ext_fs_id, u.player_name, u.uses
FROM game_abilities u
JOIN both b ON u.ext_fs_id = b.ext_fs_id AND u.player_name = b.player_name
JOIN tracked_abilities t ON u.ability = t.ability AND b.race = t.race
ORDER BY b.patch, b.race, u.ability, u.ext_fs_id, u.player_name
```

The Polars plan maps the localized race names too (`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`).

## Edge cases

- An ability of another race (a mind controlled unit's) is not a use of the player's race.
- An ability that is not tracked (every command with no energy or cooldown) is not here.
- A game not in `players` (an arcade race, an unknown protocol version) has no rows.

## Performance

The players joined with the uses, twice: a hundred milliseconds on the 36,000-game snapshot,
once per snapshot (`ability_usage_cached`).

## Example

The games of the test (`tests.rs`, `usage_over_time::test_games::with_abilities`): Bob used
Spawn Larva 20 times in game 1 (patch 95248) and Alice 15 times in game 5 (95248); Alice called
down 5 MULEs as Terran in game 1. Six rows: each of the three under its patch and under 0.
