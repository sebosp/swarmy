# Accounts with a name like the player's

## Purpose

The second table of the Accounts panel of the Player view: the accounts of other players whose
name contains the player's name, to add the ones that are the same person (another region, a
tournament account) with one click.

## Inputs

The rows of [`player_rows`](../player_rows/README.md): `player`, `account`, `grouped_as`,
`name`, `ext_fs_id`. The player is the parameter of the query, `'Alice'` below.

## Output

As [`accounts`](../accounts/README.md): one row per account and name, the most games first.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `account` | str | `region-realm-id` of the account | never |
| `grouped_as` | str | the player the account is grouped under | not grouped |
| `name` | str | a name the account played as, containing the player's name | never |
| `name_games` | u32 | the games under that name | never |
| `games` | u32 | the games of the account | never |

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
    WHERE player <> 'Alice' AND account IS NOT NULL AND STRPOS(LOWER(name), 'alice') > 0
    GROUP BY account, grouped_as, name
)
SELECT *, SUM(name_games) OVER (PARTITION BY account) AS games
FROM names
ORDER BY games DESC, account, name_games DESC, name
```

## Edge cases

- The match ignores case and finds the name inside a longer one: `alice`, `Alice2`, `TheAlice`.
- A short name matches many accounts: `shape` keeps the first 20, the most games first.
- The accounts already grouped under the player are theirs, they are in `accounts`, not here.

## Performance

`player_rows`, a lowercase contains over the names and a group by: tens of milliseconds on the
36,000-game snapshot.

## Example

The games of the test (`tests.rs`, `test_games::alice_and_friends`): Alice2, another account,
lost game 6 to Bob. Expected: one account, name Alice2, 1 game; AliceEU is not here, it is
grouped under Alice.
