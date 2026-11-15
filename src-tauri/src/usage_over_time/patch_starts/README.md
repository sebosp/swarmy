# First period of each protocol version

## Purpose

The dashed lines of the charts over time: when each patch arrived in the games of the snapshot,
so that a change of a curve is read against the patch that caused it. `shape` groups the
protocol versions by the patch of the balance data closest to them.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `player_usage` of the grain | `ext_protocol_version`, `period` | one per period, protocol version, race, account and player |

The grain is the parameter; below the months, the table `month_player_usage`.

## Output

One row per protocol version, by version.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `ext_protocol_version` | u32 | the base build of the replays | never |
| `period` | str | the first period with a game of that version | never |

## Query

```sql
SELECT ext_protocol_version, MIN(period) AS period
FROM month_player_usage
GROUP BY ext_protocol_version
ORDER BY ext_protocol_version
```

## Edge cases

- Several protocol versions belong to one patch of the balance data: `shape` keeps the earliest
  period of the patch.
- The periods compare as text, which orders them in time (`2026-01` < `2026-02`).

## Performance

A group by over the usage table of the grain: a millisecond.

## Example

The games of the test (`tests.rs`, `player_detail::test_games::alice_and_friends`): version
95248 was played from January (games 1, 2, 5), version 96000 from February (games 3, 4, 6).
Expected: 95248 `2026-01`, 96000 `2026-02`; `shape` gives their patches in that order.
