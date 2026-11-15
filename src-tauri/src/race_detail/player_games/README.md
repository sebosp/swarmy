# Games of the players of a race

## Purpose

The first line of Race Detail: how many games were played as the race, so that the shares of
the units ("Zerglings in 94% of the games") have their denominator.

## Inputs

The `players` of the snapshot (see the Query): the rows of `details` of the players (not the
observers) of a StarCraft II race, with the `patch` of their game and their tracker `player_id`.

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `player_name`, `player_observe`, `player_race`, `player_result`, `title`, `ext_protocol_version` | one per player per game |
| `game_players` | `ext_fs_id`, `player_name`, `player_id` | one per player per game |
| `patch_versions` | `ext_protocol_version`, `patch` | the patch of the balance data closest to each protocol version, computed by swarmy |

The race is the parameter of the query, `'Zerg'` below.

## Output

One row.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `player_games` | u32 | the rows of players of the race: a ZvZ counts twice, it is two players' games | never |

## Query

The `players` CTE is `data::snapshot_players`, the base of every query of this view.

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
SELECT COUNT(*) AS player_games
FROM players
WHERE race = 'Zerg'
```

The Polars plan maps the localized race names too (`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`).

## Edge cases

- The arcade and co-op races are not StarCraft II races: their players are left out.
- Observers are not players.

## Performance

`details` and `game_players` are hot tables, from memory: a filter and a count. Milliseconds.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): the Zerg
rows are Bob in games 1, 4, 5 and 6, Dave in games 3 and 5, Alice in game 5. Expected: 7.
