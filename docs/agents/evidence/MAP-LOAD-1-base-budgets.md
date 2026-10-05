# MAP-LOAD-1 base model budgets (ADR-0021 §4.8)

- Task: MAP-LOAD-1 (decision `OTERYN_GAME_MAP_LOAD_PACKET1_BUNDLE_LOADER_DECISION_2026-10-04.md` §2.2)
- Date: 2026-10-04
- Rows: `MAP01-BASE-LOAD-MS`, `MAP01-BASE-RSS-BYTES`, `MAP01-VIEWPORT-US`
  (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`)
- Result: **all three confirmed at their ADR values; no revision.**

## Results

| Row | Budget | Measured (3 runs) | Verdict |
|---|---|---|---|
| `MAP01-BASE-LOAD-MS` | 5,000 ms | 3,426-4,006 ms per load (15 loads; median about 3,690) | pass |
| `MAP01-BASE-RSS-BYTES` | 1 GiB | base delta 375.4-375.5 MB (about 358 MiB); process RSS after load 405 MB; peak (VmHWM) 431 MB | pass |
| `MAP01-VIEWPORT-US` | 100 µs p99 | p99 72.4 / 83.1 / 82.4 µs; p50 about 25 µs | pass |

Every run loaded the same base: 19,373,519 tiles, 24,983,331 entries.

The viewport budget did not pass on the first model. A full-table binary search per tile and a
popcount over the preceding occupancy words gave p99 152 µs. Two changes brought it inside the
budget, and both are in this PR's `WorldBase`:

- a per-word rank in each sector's occupancy, which took p99 to 119 µs;
- a sector-row index (`rows`, 16 × 2,048 + 1 `u32`, 128 KiB) so that a lookup searches one row
  instead of all 31,119 sectors, which took p99 to 72-83 µs.

The load time and RSS above are for this final model.

## Bundle

- Source: the checked-in World Project at base `3af9994`, compiled by
  `map_load_budget_compile_the_real_map` (`apps/game-server/tests/map_load_base.rs`). This
  test reads the same inputs as the compiler binary: the placement index shards, the palette, the
  World extent, and the teleport, house, creature and spawn families. It compiles them as a
  `production` bundle.
- File: 26,174,538 bytes, sha256
  `e4b15ffb585774f0dc51451fd7c717925ab6262adab919b652f002b5030d12ae`. It is not checked in.
- Caveat: on `main`, the compiler binary cannot compile the real map yet.
  - A `production` compile refuses the provisional donor keys.
  - A `non-production` compile stops at `oteryn:terrain.tibia.i18566: kind is UNKNOWN`. 65
    Terrain records have an `UNKNOWN` kind. This is the content gap that the decision's §2.1 routes
    to the content lane before MAP-CUTOVER-1.
  - So the measurement uses an in-test resolver. Every key resolves, which puts every tile and
    entry of the real map in the bundle.
    - A key with a known Terrain kind takes that kind.
    - An `UNKNOWN` or provisional key loads as an item.
    - Compact ids are derived from each key's sha256.
  - The counts, and therefore the measured memory and time, are those of the full real map. Only
    the ground classification of the unclassified records may differ after classification.
    Classification changes neither the tile count nor the entry count.

## Method

- Build: `--release`, rustc 1.94.0, from the workspace lockfile.
- `map_load_budget_measure` runs in a fresh process.
  - It reads the bundle file into memory and records the RSS (`/proc/self/status` `VmRSS`).
  - It loads the bundle 5 times with `oteryn_game_server::map::load` and production pins, timing each
    load from the bytes to a verified `WorldBase`. That covers the digest, the sector checksums,
    decompression, cap checks and the build.
  - It drops the bundle bytes and records `VmRSS` and `VmHWM`. The base delta is the RSS after
    load minus the RSS before.
- Viewport: 20,000 viewports of 18 × 14 tiles.
  - Each is centred on a seeded sample of real tiles on native floor -7 (legacy z 7), so hotspots
    are sampled in proportion to their tile density.
  - Each covers every floor a client at z 7 sees: native -7..=0, each floor above shifted by its
    height.
  - For every present tile, the run reads the item ids and the ground speed. It times the whole
    viewport and reports p50, p99 and max.
- Commands:

  ```text
  MAP_LOAD_BUDGET_BUNDLE=<path> cargo test --locked --release -p oteryn-game-server \
      --test map_load_base -- --ignored --exact map_load_budget_compile_the_real_map
  MAP_LOAD_BUDGET_BUNDLE=<path> cargo test --locked --release -p oteryn-game-server \
      --test map_load_base -- --ignored --exact map_load_budget_measure --nocapture
  ```

## Node

- This session's cloud container: 4 vCPU (Intel Xeon @ 2.80 GHz), 15.7 GiB memory (MemTotal
  16,480,972 kB), Linux 6.18.
- It matches the reference node's shape (4 vCPU / 15.7 GiB). It is a shared virtual machine, so a
  few viewport outliers (max 0.2-1.5 ms) are scheduler noise. They fall above p99.
- A rerun on the production node class at MAP-CUTOVER-1 is recommended. It would use the bundle
  that CI builds once the `UNKNOWN` records are classified.
