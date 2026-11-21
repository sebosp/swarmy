# The density of the deaths of a replay

## Purpose

Where the units of a replay died, as a surface: the Deaths view of the map of the Replay view
draws it as a height map, the darker the more deaths around. A kernel density estimate of the
deaths that cost resources ([`deaths`](../deaths/README.md)) over the cells of the playable area,
with the conventions of the cartographic practice (`docs/research/replay-map-density-and-time.md`):
a Gaussian kernel with the Silverman and Scott bandwidth kept between 5 and 9 map units, a floor
at 5 % of the peak under which a cell is open ground, 6 classes of the same area. Computed when
the replay is opened, when its map cache is there.

## Inputs

| Table | Columns | Rows |
|---|---|---|
| `unit_died` | `ext_fs_id`, `unit_died_name`, `cause`, `x`, `y`, `ext_replay_loop` | one per unit died |
| `valued_units` | `variant` | the units and structures that cost resources, with their variants; a frame swarmy builds, `replay_detail::valued_units::valued_units` |
| `kernel_offsets` | `dx`, `dy` | the cells around a death, −14 to 14 each way (three times the largest bandwidth); a frame swarmy builds, `death_density::kernel_offsets` |

The parameters: the id of the replay (`1` below), the playable area of the map (`0` to `100`
both ways below, from the map cache) and the side of a cell, 2 map units (`DENSITY_CELL`).

## Output

One row per cell above the floor, by row and column.

| Column | Type | Meaning | Null when |
|---|---|---|---|
| `column` | u32 | the column of the cell from the left of the playable area | never |
| `row` | u32 | its row from the bottom | never |
| `density` | f32 | the deaths around, in deaths per cell: the kernels of all the deaths sum to their number | never |
| `class` | u32 | 1 to 6, the densest last; each class has the same number of cells (the lowest holds the remainder) | never |

## Query

```sql
WITH deaths AS (
    SELECT CAST(d.x AS REAL) AS x, CAST(d.y AS REAL) AS y
    FROM unit_died d
    JOIN valued_units v ON d.unit_died_name = v.variant
    WHERE d.ext_fs_id = 1
      AND d.cause IN ('killed', 'own_player')
),
bandwidth AS (
    -- The population variance written out, Polars SQL has no VAR_POP.
    SELECT CAST(GREATEST(5.0, LEAST(9.0,
               SQRT(((AVG(x * x) - AVG(x) * AVG(x)) + (AVG(y * y) - AVG(y) * AVG(y))) / 2)
               * POWER(COUNT(*), -1.0 / 6))) AS REAL) AS bandwidth
    FROM deaths
),
bins AS (
    SELECT CAST(FLOOR((x - 0) / 2) AS INT) AS "column", CAST(FLOOR((y - 0) / 2) AS INT) AS "row", COUNT(*) AS deaths
    FROM deaths
    GROUP BY "column", "row"
),
spread AS (
    SELECT b."column" + o.dx AS "column", b."row" + o.dy AS "row",
           b.deaths * (2 * 2 / (PI() * 2 * h.bandwidth * h.bandwidth))
             * EXP(-(POWER(o.dx * 2.0, 2) + POWER(o.dy * 2.0, 2)) / (2 * h.bandwidth * h.bandwidth)) AS weight
    FROM bins b
    CROSS JOIN bandwidth h
    CROSS JOIN kernel_offsets o
    WHERE POWER(o.dx * 2.0, 2) + POWER(o.dy * 2.0, 2) <= 9 * h.bandwidth * h.bandwidth
),
cells AS (
    SELECT "column", "row", CAST(SUM(weight) AS REAL) AS density
    FROM spread
    WHERE "column" >= 0 AND "column" < 50 AND "row" >= 0 AND "row" < 50
    GROUP BY "column", "row"
),
with_peak AS (
    SELECT "column", "row", density, MAX(density) OVER () AS peak
    FROM cells
),
above_floor AS (
    SELECT "column", "row", density
    FROM with_peak
    WHERE density > peak * 0.05
),
ranked AS (
    SELECT "column", "row", density,
           ROW_NUMBER() OVER (ORDER BY density, "row", "column") - 1 AS rank,
           COUNT(*) OVER () AS cells
    FROM above_floor
)
SELECT CAST("column" AS UINT) AS "column", CAST("row" AS UINT) AS "row", density,
       CAST(1 + (rank * 6) / cells AS UINT) AS class
FROM ranked
ORDER BY "row", "column"
```

## Edge cases

- No death that cost resources: no rows, the view shows the plain map.
- One death: the bandwidth is 5 (the rule needs two), a single bump of six rings.
- The deaths of a cell spread beyond the playable area are lost: the density near the edges is
  a little lower than the deaths there.
- A class boundary on equal densities is decided by the position of the cell, so that the
  classes keep the same area.

## Performance

A few hundred bins cross-joined with 841 offsets, filtered to the three bandwidths: tens of
milliseconds for the 2,000 deaths of a long game, once per replay opened.

## Example

The game of the test (`tests.rs`, `replay_detail::test_games::fight`, a 100 by 100 map): four
deaths around (50, 50), one at (30, 30) and one at (10, 90). Expected: the densest cells around
(50, 50) are of class 6, the cell of (30, 30) and the cell of (10, 90) hold a death each and are
denser than the cells between the fights, the densities of all the cells sum to about 6.
