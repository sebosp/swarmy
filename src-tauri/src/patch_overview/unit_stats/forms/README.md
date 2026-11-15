# The forms of the units (morphs), per patch

## Purpose

The other forms a unit takes in the games: a Roach burrows, a Siege Tank sieges, a Viking lands, a
Lurker burrows to attack, a Command Center becomes an Orbital Command, a Hatchery a Lair. The
tracker of the replay records these as a unit type change, and the snapshot writes them as the
same unit (its tag) born again with another type. Counting the consecutive types of every unit
gives the forms and how often each change happens; the Unit Overview shows the ones connected to
its unit, with the stats of each form from the balance data.

The morphs only change with a patch, once or twice a year: they are not queried at request time.
`generate` runs this query over a large snapshot when a patch comes out, keeps the pairs the
patch's balance data confirms, and the result is committed as `unit_forms.json`, compiled in
(`static_forms`). The Unit Overview reads the table of the newest patch (`latest_forms`).

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `unit_born` | `ext_fs_id`, `unit_tag_index`, `unit_tag_recycle`, `unit_type_name`, `ext_replay_loop` | one per unit born or changed |
| `details` | `ext_fs_id`, `ext_protocol_version` | one per player of each game |
| `patch_versions` | `ext_protocol_version`, `patch` | the patch of the balance data of each protocol version (`data::patch_versions`) |

## Output

One row per patch and pair of consecutive types, by patch, the most frequent first, then by names.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `patch` | u32 | the patch of the games (a version of the balance data) | never |
| `from` | str | the type the unit had | never |
| `to` | str | the type it changed into | never |
| `changes` | u32 | how many times in the games of the patch | never |

## Query

```sql
WITH games AS (
    SELECT DISTINCT d.ext_fs_id, v.patch
    FROM details d
    JOIN patch_versions v ON d.ext_protocol_version = v.ext_protocol_version
),
ordered AS (
    SELECT ext_fs_id, unit_tag_index, unit_tag_recycle, unit_type_name,
           ROW_NUMBER() OVER (PARTITION BY ext_fs_id, unit_tag_index, unit_tag_recycle ORDER BY ext_replay_loop) AS step
    FROM unit_born
)
SELECT g.patch, a.unit_type_name AS "from", b.unit_type_name AS "to", COUNT(*) AS changes
FROM ordered a
JOIN ordered b ON a.ext_fs_id = b.ext_fs_id
              AND a.unit_tag_index = b.unit_tag_index
              AND a.unit_tag_recycle = b.unit_tag_recycle
              AND b.step = a.step + 1
JOIN games g ON a.ext_fs_id = g.ext_fs_id
WHERE a.unit_type_name <> b.unit_type_name
GROUP BY g.patch, "from", "to"
ORDER BY g.patch, changes DESC, "from", "to"
```

## Edge cases

- A unit born again with the same type (a Command Center landing where it was) is not a change.
- A tag reused by the game for another unit has another `unit_tag_recycle`: two units.
- `validate` keeps a pair only if both types are units of the patch's balance data: a building
  of a custom map with a melee name (`Lair2`), a wreckage or a decoration is not a form. The
  balance data has a file for every form (`RoachBurrowed`, `SiegeTankSieged`, `VikingAssault`,
  `LurkerMPBurrowed`, `Lair`...), but not which type an ability leads to (`BurrowRoachDown` doesn't
  say `RoachBurrowed`): the replays name the pairs, the balance data confirms them.
- A patch without its own table takes the one of the newest patch before it, else the oldest
  (`forms_of_patch`): the morphs rarely change.
- The Unit Overview takes the types its unit changes into, directly or through another form
  (`forms_of`): `Lair` and `Hive` are morphs of the Hatchery, the Hatchery is not one of the
  Hive; the Hydralisk that becomes a Lurker is another unit, not a form of the Lurker. The pairs
  under 1 % of the most frequent change of the unit (a cancelled morph) are left out.

## When a patch comes out

The test `the_static_table_covers_the_newest_patch` fails once the balance data has a patch
without a table. On corvid, with a snapshot that has games of the new patch:

```sh
SWARMY_TEST_REPLAYS=~/SCReplaysOnNVMe cargo test --release --lib write_unit_forms -- --ignored --nocapture
```

then review the diff of `unit_forms.json` (a new form, a disappeared one) and commit it.

## Performance

At request time: none, a table parsed once. Measured on 2026-11-12, on a copy of the AI Arena
snapshot (58,444 games): the Unit Overview's query went from 12.8 GB of heap at its peak and
3.4 s to 0.54 GB and 0.3 s. The generation sorts one day partition of `unit_born` at a time and
adds the counts up: 24 s and 2.6 GB at its peak over the pro snapshot (39,944 games, the busiest
days set the peak), instead of the 13 GB of the sort of a whole snapshot; two runs write the same
file.

## Example

The games of the test (`tests.rs`, `patch_overview::unit_stats::test_games::forms`), of protocol
95248: in game 1 a Roach burrows and unburrows, a Siege Tank sieges; in game 2 a Roach burrows.
Expected, all in patch 95248: `Roach → RoachBurrowed` 2 changes, `RoachBurrowed → Roach` 1,
`SiegeTank → SiegeTankSieged` 1.
