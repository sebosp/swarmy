# Games of the race of an ability, with its uses

## Purpose

The one query of the Ability view: every game of every player of the race of the ability, used
or not, with how many times the player used it and when first, the matchup, the map, the patch
and the day. `aggregate` in `ability_detail.rs` turns the rows into the totals, the use by patch,
month and matchup, the top players and games and the fastest first use.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `players` (`data::snapshot_players`: `details`, `game_players`, `patch_versions`) | `ext_fs_id`, `player_name`, `race`, `title`, `patch` | one per player per game, no observers, StarCraft II races only |
| `game_abilities` | `ext_fs_id`, `player_name`, `ability`, `uses`, `first_seconds` | one per player, game and ability used |
| `details` | `ext_fs_id`, `ext_datetime` | one per player per game |

The ability is the parameter, with its race (`swarmy_common::tracked_ability`):
`'SpawnLarva.Execute'` of the `'Zerg'` below.

## Output

One row per game of a player of the race, by game and player.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the game | never |
| `player_name` | str | the player, with the clan tag as in the replay | never |
| `title` | str | the map | never |
| `patch` | u32 | the balance data version of the game | never |
| `date` | str | `YYYY-MM-DD` of the game | never |
| `matchup` | str | `ZvT`: the race first, the opponent's letter; `Team games` when not a 1v1 | never |
| `uses` | u32 | the uses of the ability in the game | not used |
| `first_seconds` | u32 | the game second of the first use | not used |

## Query

The `players` CTE is `data::snapshot_players`; `sided` numbers the rows of a game, so that the
other row of a 1v1 is found even when two players have the same name.

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
sided AS (
    SELECT p.*, ROW_NUMBER() OVER (PARTITION BY p.ext_fs_id ORDER BY p.player_name) AS side
    FROM players p
),
opponents AS (
    SELECT s.ext_fs_id, s.side AS other_side, s.race AS opponent_race
    FROM sided s JOIN counts c ON s.ext_fs_id = c.ext_fs_id
    WHERE c.n = 2
),
mine AS (
    SELECT a.ext_fs_id, a.player_name, a.title, a.patch,
           CASE WHEN o.opponent_race IS NULL THEN 'Team games'
                ELSE CONCAT('Z', 'v', LEFT(o.opponent_race, 1)) END AS matchup
    FROM sided a
    LEFT JOIN opponents o ON a.ext_fs_id = o.ext_fs_id
    WHERE a.race = 'Zerg' AND (o.other_side IS NULL OR o.other_side <> a.side)
),
uses AS (
    SELECT ext_fs_id, player_name, uses, first_seconds
    FROM game_abilities
    WHERE ability = 'SpawnLarva.Execute'
),
dates AS (
    SELECT ext_fs_id, STRFTIME(MIN(ext_datetime), '%Y-%m-%d') AS date
    FROM details
    GROUP BY ext_fs_id
)
SELECT m.ext_fs_id, m.player_name, m.title, m.patch, d.date, m.matchup, u.uses, u.first_seconds
FROM mine m
LEFT JOIN uses u ON m.ext_fs_id = u.ext_fs_id AND m.player_name = u.player_name
LEFT JOIN dates d ON m.ext_fs_id = d.ext_fs_id
ORDER BY m.ext_fs_id, m.player_name
```

The Polars plan maps the localized race names too (`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`).

## Edge cases

- A mirror (`ZvZ`) is a matchup: both players are rows of the race.
- A game of a Zerg against a Primal Zerg (an arcade race) has one player of a StarCraft II race:
  `Team games`, there is no 1v1 opponent to name.
- A player that did not use the ability has null `uses` and `first_seconds`: `aggregate` counts
  the game and 0 uses.

## Performance

The players of the snapshot (a join of three hot tables), a self join for the opponents, two left
joins: a hundred milliseconds on the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `usage_over_time::test_games::with_abilities`): seven Zerg
player games; Bob used Spawn Larva 20 times from 1:00 in game 1 (`ZvT`, lost), Alice 15 times
from 1:10 in game 5 (`Team games`); Bob's game 4 against the Protoss computer is `ZvP`.
