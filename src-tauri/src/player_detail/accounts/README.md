# Accounts of a player

## Purpose

The Accounts panel of the Player view: the Battle.net accounts that are the player, with the
names each one played as and its games, so that the player sees what counts as them and removes
an account that is not theirs.

## Inputs

The rows of [`player_rows`](../player_rows/README.md): `player`, `account`, `grouped_as`,
`name`, `ext_fs_id`. The player is the parameter of the query, `'Alice'` below.

## Output

One row per account and name, the account with the most games first, within an account the
most played name first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `account` | str | `region-realm-id` of the account | never (the computers are left out) |
| `grouped_as` | str | the player the account is grouped under in the personal database | not grouped |
| `name` | str | a name the account played as, without the clan tag | never |
| `name_games` | u32 | the games under that name | never |
| `games` | u32 | the games of the account, all its names | never |

## Query

With the `players` of `player_rows` (its CTEs repeated here, the test runs them):

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
    SELECT r.*, g.grouped_as, COALESCE(g.grouped_as, a.account_name, r.name) AS player
    FROM rows r
    LEFT JOIN account_names a ON r.account = a.account
    LEFT JOIN player_groups g ON r.account = g.account
),
names AS (
    SELECT account, grouped_as, name, COUNT(DISTINCT ext_fs_id) AS name_games
    FROM players
    WHERE player = 'Alice' AND account IS NOT NULL
    GROUP BY account, grouped_as, name
)
SELECT *, SUM(name_games) OVER (PARTITION BY account) AS games
FROM names
ORDER BY games DESC, account, name_games DESC, name
```

## Edge cases

- A computer player has no account: nothing to show, their name is their identity.
- The names are without the clan tag: `&lt;TL&gt;<sp/>Alice` and `Alice` are one name.
- `shape` keeps the first 100 accounts; a player with more is a name shared by many accounts,
  `A.I.` aside.

## Performance

`player_rows`, a filter on one player and a group by over their rows: tens of milliseconds on
the 36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`): Alice played games 1, 2 and
5 with the account 2-1-1001 (once as `&lt;TL&gt;<sp/>Alice`) and game 3 with 2-1-777 as AliceEU,
grouped under Alice. Expected: 2-1-1001, name Alice, 3 games; 2-1-777, grouped as Alice, name
AliceEU, 1 game.
