# What wins against an opening

## Purpose

In the 1v1 games of the snapshot, the openings of a race (its first three tech structures, as
the opening of the build order) and how often they won against each opening of the enemy race:
a replay shows the answers that won the most against the opening of the opponent, and the record
of the player's own opening against it. Computed once per snapshot and kept in memory.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `player_name`, `player_race`, `player_result`, `player_observe` | one per player per game |
| `game_players` | `ext_fs_id`, `player_name`, `player_id` | one per player per game, the tracker ids |
| `unit_born` | `ext_fs_id`, `control_player_id`, `unit_type_name`, `ext_replay_loop` | one per unit born |
| `tech_structures` | `unit` | the tech structures of the balance data (not supply, gas, townhalls, defenses or variants); a frame swarmy builds, `opening_answers::answers::tech_structures` |

## Output

One row per race, enemy race, enemy opening and opening with at least 5 games
(`MIN_ANSWER_GAMES`), by those.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `race` | str | the race of the players who answered | never |
| `enemy_race` | str | the race of their opponents | never |
| `enemy_opening` | str | the opponents' first three tech structures, `SpawningPool → RoachWarren → EvolutionChamber` | never |
| `opening` | str | the players' first three tech structures | never |
| `games` | u32 | the games of the race with that opening against that enemy opening | never |
| `wins` | u32 | the ones won | never |

## Query

```sql
WITH ids AS (
    SELECT DISTINCT ext_fs_id, player_name, player_id FROM game_players
),
players AS (
    SELECT d.ext_fs_id, i.player_id,
           CASE WHEN d.player_race = 'Terran' THEN 'Terran'
                WHEN d.player_race = 'Zerg' THEN 'Zerg'
                WHEN d.player_race = 'Protoss' THEN 'Protoss'
                ELSE 'Other' END AS race,
           d.player_result
    FROM details d
    JOIN ids i ON d.ext_fs_id = i.ext_fs_id AND d.player_name = i.player_name
    WHERE d.player_observe = 0
),
one_vs_ones AS (
    SELECT ext_fs_id FROM players GROUP BY ext_fs_id HAVING COUNT(*) = 2
),
firsts AS (
    SELECT b.ext_fs_id, b.control_player_id AS player_id, b.unit_type_name, MIN(b.ext_replay_loop) AS first_loop
    FROM unit_born b
    JOIN tech_structures t ON b.unit_type_name = t.unit
    WHERE b.ext_replay_loop > 0
    GROUP BY b.ext_fs_id, b.control_player_id, b.unit_type_name
),
positioned AS (
    SELECT ext_fs_id, player_id, unit_type_name,
           ROW_NUMBER() OVER (PARTITION BY ext_fs_id, player_id ORDER BY first_loop, unit_type_name) AS position
    FROM firsts
),
openings AS (
    SELECT ext_fs_id, player_id, ARRAY_TO_STRING(ARRAY_AGG(unit_type_name), ' → ') AS opening
    FROM (SELECT * FROM positioned WHERE position <= 3 ORDER BY ext_fs_id, player_id, position)
    GROUP BY ext_fs_id, player_id
    HAVING COUNT(*) = 3
),
played AS (
    SELECT p.ext_fs_id, p.player_id, p.race, p.player_result, o.opening
    FROM players p
    JOIN one_vs_ones g ON p.ext_fs_id = g.ext_fs_id
    JOIN openings o ON p.ext_fs_id = o.ext_fs_id AND p.player_id = o.player_id
)
SELECT * FROM (
    SELECT a.race, a.enemy_race, a.enemy_opening, a.opening, COUNT(*) AS games,
           CAST(SUM(CASE WHEN a.player_result = 'Win' THEN 1 ELSE 0 END) AS UINT) AS wins
    FROM (
        SELECT p.race, e.race AS enemy_race, e.opening AS enemy_opening, p.opening, p.player_result
        FROM played p
        JOIN played e ON p.ext_fs_id = e.ext_fs_id
        WHERE p.player_id <> e.player_id
    ) a
    GROUP BY a.race, a.enemy_race, a.enemy_opening, a.opening
)
WHERE games >= 5
ORDER BY race, enemy_race, enemy_opening, opening
```

The Polars plan maps the localized race names too (`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`).

## Edge cases

- A game that ended before a player's third tech structure is not compared: no opening.
- Two structures finishing in the same game loop are ordered by name.
- A structure made twice (a second Barracks) is one tech structure, at its first finish.
- A game against a computer is a 1v1 like the others here.
- Fewer than five games of an answer against an opening: not shown.

## Performance

The tech structures of every game (a filter of the large `unit_born` table) and a few joins: a
second on the 36,000-game snapshot, once per snapshot; the view runs it with the streaming
engine.

## Example

The games of the test (`tests.rs`, `replay_detail::test_games::patch_games`): in the five human
games the Terran opened Barracks, Factory, Starport and won against the Zerg's Spawning Pool,
Roach Warren, Evolution Chamber; in game 6 the Terran computer opened the same and lost to Bob.
Expected: Terran vs that Zerg opening, `Barracks → Factory → Starport` 6 games 5 wins; Zerg vs
the Terran opening, `SpawningPool → RoachWarren → EvolutionChamber` 6 games 1 win. Game 7 has
no third structure.
