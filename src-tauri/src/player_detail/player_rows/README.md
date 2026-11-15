# The players of the snapshot

## Purpose

The base of the Player view, the Replay List search and the head to head: who played each game.
A player is their Battle.net accounts, not a name: an account keeps its games when its name or
clan tag changes, and the accounts grouped under a player in the Accounts panel of the Player view
(another region, a tournament account) count as that player.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `details` | all, with `player_observe`, `player_name`, `player_toon_region`, `player_toon_realm`, `player_toon_id` | one per player per game |
| `player_groups` | `account`, `grouped_as` | the accounts grouped under a player in the personal database (a parameter of the query, not a table of the snapshot) |

## Output

The rows of `details` of the players, with five columns more.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `account` | str | `region-realm-id` of the Battle.net account | a computer player (id 0) |
| `name` | str | the name without the clan tag (`&lt;TL&gt;<sp/>Alice` is `Alice`) | never |
| `account_name` | str | the name the account played the most games with, then the first by name | no account |
| `grouped_as` | str | the player the account is grouped under | not grouped |
| `player` | str | the player: `grouped_as`, else `account_name`, else `name` | never |

## Query

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
    FROM rows
    WHERE account IS NOT NULL
    GROUP BY account, name
),
ranked AS (
    SELECT *, ROW_NUMBER() OVER (PARTITION BY account ORDER BY 0 - CAST(games AS BIGINT), name) AS rank
    FROM name_games
),
account_names AS (
    SELECT account, name AS account_name FROM ranked WHERE rank = 1
)
SELECT r.*, a.account_name, g.grouped_as,
       COALESCE(g.grouped_as, a.account_name, r.name) AS player
FROM rows r
LEFT JOIN account_names a ON r.account = a.account
LEFT JOIN player_groups g ON r.account = g.account
```

## Edge cases

- Observers are not players.
- Computer players have no account (`player_toon_id` 0): their player is their name, `A.I. 1
  (Very Easy)`.
- A name used by two accounts is two players, unless both accounts are grouped under one.
- A clan tag is `&lt;tag&gt;<sp/>` before the name; the name is what follows the `<sp/>`.

## Performance

`details` is a hot table, from memory; two group bys over all its rows and two joins: tens of
milliseconds on the 36,000-game snapshot. Every query of the Player view starts from this plan,
Polars computes it once per query.

## Example

The games of the test (`tests.rs`):

| Game | Players |
|---|---|
| 1 | `&lt;TL&gt;<sp/>Alice` (account 2-1-1001, Terran) beats Bob (Zerg) |
| 2 | Carol (Protoss) beats Alice (account 2-1-1001, Terran) |
| 3 | AliceEU (account 2-1-777, grouped under Alice) ties Dave |
| 4 | Bob beats A.I. 1 (Very Easy), a computer |

Expected: the account 2-1-1001 is named `Alice` (2 games as Alice, 1 as `&lt;TL&gt;<sp/>Alice`),
so game 1's row has `name` `Alice` and `player` `Alice`; AliceEU's row has `player` `Alice`
(grouped); the computer has no account and `player` `A.I. 1 (Very Easy)`.
