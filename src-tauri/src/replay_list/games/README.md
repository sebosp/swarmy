# Games of the Replay List

## Purpose

The table of the Replay List: the games of the snapshot that match the filters of the view (map,
player, dates, matchup, team size, protocol version, the notes and tags of the personal
database), one row per game with its players, sorted as the viewer asks. The view shows the
first 1,000 (`REPLAY_LIST_LIMIT`) and says how many match in all.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | `ext_fs_id`, `title`, `ext_datetime`, `player_name`, `player_result`, `player_race`, `player_team_id`, `player_observe`, `ext_fs_file_name`, `map_info_sha256`, `ext_fs_sha256`, `ext_protocol_version` | one per player per game |
| `game_players` | `ext_fs_id`, `game_seconds` | one per player per game |

and, for the player filter, the rows of [`player_rows`](../../player_detail/player_rows/README.md).
The filters are the parameters of the query; below: the map contains `pylon`, a player contains
`alice`, from 2026-02-01, the matchup `TvZ` (so 1v1 games), the newest first.

## Output

One row per game, in the order asked (`sort_by`, `descending`; the ties by `ext_fs_id`, the
nulls last).

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_fs_id` | u64 | the replay | never |
| `title` | str | the map | never |
| `ext_datetime` | datetime | when the game was played | never |
| `replay_date` | str `YYYY-MM-DD` | the day | never |
| `player_names` | list of str | the players, in the order of the replay | never |
| `player_results` | list of str | their results, aligned with the names | never |
| `player_races` | list of str | their races as stored, aligned with the names | never |
| `replay_location` | str | the path of the replay file | never |
| `map_info_sha256` | str | the sha256 of the map info | never |
| `replay_sha256` | str | the sha256 of the replay file, the notes are kept by it | never |
| `players_sort` | str | the lowercase names sorted and joined, to sort by players | never |
| `matchup` | str | the race letters sorted and joined with `v`: `PvT`, `TvZ`, `PvTvZvZ` in a 2v2 | never |
| `players` | u32 | the players of the game | never |
| `teams` | u32 | the teams | never |
| `game_seconds` | u32 | the length of the game | the game is not in `game_players` |

## Query

With the `players` of `player_rows` (its CTEs repeated here, the test runs them) for the player
filter; each `WHERE` line of `filtered` is one optional filter of the view, the notes and tags
filter is `ext_fs_sha256 IN (...)` with the sha256s of the personal database.

```sql
WITH rows AS (
    SELECT *,
           CASE WHEN player_toon_id = 0 THEN NULL
                ELSE CONCAT(CAST(player_toon_region AS VARCHAR), '-',
                            CAST(player_toon_realm AS VARCHAR), '-',
                            CAST(player_toon_id AS VARCHAR)) END AS account,
           CASE WHEN STRPOS(player_name, '<sp/>') > 0 THEN SPLIT_PART(player_name, '<sp/>', 2)
                ELSE player_name END AS name
    FROM details
    WHERE player_observe = 0
),
name_games AS (
    SELECT account, name, COUNT(DISTINCT ext_fs_id) AS games
    FROM rows WHERE account IS NOT NULL GROUP BY account, name
),
ranked AS (
    SELECT *, ROW_NUMBER() OVER (PARTITION BY account ORDER BY 0 - CAST(games AS BIGINT), name) AS rank
    FROM name_games
),
account_names AS (SELECT account, name AS account_name FROM ranked WHERE rank = 1),
players AS (
    SELECT r.*, COALESCE(g.grouped_as, a.account_name, r.name) AS player
    FROM rows r
    LEFT JOIN account_names a ON r.account = a.account
    LEFT JOIN player_groups g ON r.account = g.account
),
found AS (
    SELECT DISTINCT ext_fs_id
    FROM players
    WHERE STRPOS(LOWER(player), 'alice') > 0 OR STRPOS(LOWER(name), 'alice') > 0
),
filtered AS (
    SELECT d.*,
           CASE WHEN d.player_race = 'Terran' THEN 'T'
                WHEN d.player_race = 'Zerg' THEN 'Z'
                WHEN d.player_race = 'Protoss' THEN 'P'
                ELSE '?' END AS race_letter
    FROM details d
    JOIN found f ON d.ext_fs_id = f.ext_fs_id
    WHERE d.player_observe = 0
      AND STRPOS(LOWER(d.title), 'pylon') > 0
      AND CAST(d.ext_datetime AS DATE) >= DATE '2026-02-01'
),
games AS (
    SELECT ext_fs_id,
           FIRST(title) AS title,
           MAX(ext_datetime) AS ext_datetime,
           STRFTIME(FIRST(ext_datetime), '%Y-%m-%d') AS replay_date,
           ARRAY_AGG(player_name) AS player_names,
           ARRAY_AGG(player_result) AS player_results,
           FIRST(ext_fs_file_name) AS replay_location,
           FIRST(map_info_sha256) AS map_info_sha256,
           FIRST(ext_fs_sha256) AS replay_sha256,
           ARRAY_AGG(player_race) AS player_races,
           ARRAY_TO_STRING(ARRAY_AGG(LOWER(player_name) ORDER BY LOWER(player_name)), ',') AS players_sort,
           ARRAY_TO_STRING(ARRAY_AGG(race_letter ORDER BY race_letter), 'v') AS matchup,
           COUNT(*) AS players,
           COUNT(DISTINCT player_team_id) AS teams
    FROM filtered
    GROUP BY ext_fs_id
),
durations AS (
    SELECT ext_fs_id, MIN(game_seconds) AS game_seconds
    FROM game_players
    GROUP BY ext_fs_id
)
SELECT g.*, u.game_seconds
FROM games g
LEFT JOIN durations u ON g.ext_fs_id = u.ext_fs_id
WHERE g.matchup = 'TvZ' AND g.teams = 2 AND g.players = 2
ORDER BY g.ext_datetime DESC, g.ext_fs_id DESC
```

The Polars plan maps the localized race names too (`swarmy_common::TERRAN`, `ZERG`, `PROTOSS`).

## Edge cases

- Observers are not players: a casted 1v1 is a 1v1, its matchup `TvZ`, and the caster is not in
  its players.
- A matchup filter implies 1v1 games (`teams = 2`, `players = 2`) unless a team size is given;
  `PvT` and `TvP` are the same matchup (`normalize_matchup`).
- The player filter matches the player or any name of their accounts (`LiquidClem` finds Clem's
  games) and keeps every player of the games found.
- A game missing from `game_players` has no length and sorts last by duration.
- The arcade races are `?` in the matchup.

## Performance

`details` and `game_players` are hot tables, from memory; the player filter adds `player_rows`
(a semi join on the game ids). The group by runs over the filtered rows only. A search over the
36,000-game snapshot answers in 50-200 ms; the sort over all the games when nothing is filtered
is the most expensive part.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`), with the filters above:

| Game | Day | Map | Players | Kept |
|---|---|---|---|---|
| 1 | 2026-01-15 | Pylon LE | Alice (T) beats Bob (Z) | no: before 2026-02-01 |
| 2 | 2026-02-03 | Ephemeron LE | Carol (P) beats Alice (T) | no: not Pylon |
| 3 | 2026-02-10 | Pylon LE | AliceEU (T) ties Dave (Z) | yes: TvZ, AliceEU is Alice |
| 5 | 2026-02-20 | Pylon LE | Alice and Bob beat Carol and Dave | no: a 2v2 |
| 6 | 2026-03-01 | Pylon LE | Bob (Z) beats Alice2 (T) | yes: TvZ, Alice2 contains alice |

Expected: games 6 then 3, each with two players, matchup `TvZ`, 600 seconds.
