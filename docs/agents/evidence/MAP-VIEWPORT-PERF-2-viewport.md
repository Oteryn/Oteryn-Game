# MAP-VIEWPORT-PERF-2 viewport evidence

Task `OTV2-20261006-map-viewport-perf-2`, packet
`docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md` §2.2, follow-up of
MAP-VIEWPORT-PERF-1 (#1839). Gate: `MAP01-VIEWPORT-US` = 100 us p99 for one 18x14 domain-17 snapshot over
all floors in view.

## Outcome

Gate not met. The snapshot p99 drops from 3.26 ms to 1.68 ms (-48%) on the same machine and harness, with
the same wire bytes; it is still about 17x the gate. Reaching 100 us needs work outside this task's owned
paths or a budget decision (BLOCKER to the control plane, options below). The budget is unchanged.

## Changes

- `ItemHandleTable` (`item_view.rs`): the `BTreeMap` key and handle maps and the per-view `BTreeSet`s are
  replaced by a slot table (key, handle, a view bit set) found through two hash maps with a fixed
  multiply-rotate hasher, a free-slot list and per-view slot lists. `replace` marks the next content in one
  pass, issues handles in the given order and drops a key that left every view. Handles stay monotonic and
  never reused; `LimitExceeded` keeps precedence over `Exhausted`; the undo log of `map_view` keeps the
  table unchanged when encoding fails. The callers that restore a previous view get its keys in key order,
  as the `BTreeSet` gave them.
- Plan stage (`world_map.rs`, `view.rs`): the stack cut is an index range (`view::cut_indices`) instead of a
  `Vec` per tile; a view keeps the plan's buffers (pool, slots, roof facts, ranking, entries, tiles, keys)
  between updates; the snapshot encode moves the tiles into the snapshot instead of cloning them.
- The budget ranking keeps `sort_unstable_by_key` over the same `(key, entry)` input. Entries on floors
  `f + k` and `f - k` can share a budget key, so a bucket or partial selection could order such ties
  differently and change handle issuance; it is not used.

## Machine and method

- 4 vCPU Intel Xeon @ 2.10GHz, rustc 1.94.0,
  `cargo test --release --locked -p oteryn-game-server map_viewport_measure -- --ignored --nocapture`.
- 20,000 seeded viewports, floors -7..=0, world seed 7, viewport rng `0x5eed_0007`; each snapshot starts
  from the table state the previous sample's step left. Delta = one-tile step.
- Before: unmodified `origin/main` 86f704d8 in a separate worktree, same machine, same session.

## Byte identity

`seeded_walks_send_the_bytes_recorded_before_the_viewport_optimisation` passes unchanged: 324 updates,
5,431,963 bytes, sha256 `d9f1866f0a425c14443e5b16339cc4856be3fb4dc29679ae4da1f2efc80b5a01`.

## Numbers

| | snapshot p50 | snapshot p99 | delta p50 | delta p99 | plan p50 | plan p99 |
|---|---|---|---|---|---|---|
| Before (main 86f704d8) | 1.931 ms | 3.264 ms | 0.883 ms | 1.568 ms | 309 us | 521 us |
| After (this PR) | 0.949 ms | 1.683 ms | 0.542 ms | 0.982 ms | 304 us | 509 us |

The plan row times `plan()` alone with fresh buffers, as the harness calls it; inside an update the plan
reuses the view's buffers.

## Stage split after the change

Instruction counts (valgrind callgrind, the same harness at 400 samples, before the buffer reuse) per
snapshot, about 8.2 M instructions:

| Stage | Share | Owner |
|---|---|---|
| Wire encode (`encode_world_map_snapshot`: `tile_len`, `item_len`, `push_tile`) | about 50% | `crates/protocol-oteryn`, not owned |
| Plan: compose (base tile and facts reads) and budget sort | about 27% | owned; compose about 15%, sort about 4% |
| Wire tiles (`Vec<MapTile>`, one `Vec<MapItem>` per tile, handle reads) | about 11% | owned; shape fixed by the protocol types |
| Freeing the previous sent tiles | about 5% | follows from the protocol types |
| Handle table (`replace`) | about 4% | owned; was about half of the snapshot before |

Even a free plan, table and tile build leave the wire encode alone near 0.5 ms p50 on this machine.

## Options (for the control plane)

- A. Optimise `encode_world_map_snapshot` in `crates/protocol-oteryn` (one length pass, a pre-sized buffer,
  encode from borrowed tiles) under its owning contract and review; expected at most about 2x.
- B. Restate `MAP01-VIEWPORT-US` (budget decision) for the full 8-floor window, for example as a per-floor or
  per-tile figure, or a snapshot p99 measured on the delivery path; this task does not change the budget.
- C. Reduce snapshot frequency rather than cost (deltas already carry steps; delta p99 is now under 1 ms).

Recommendation: B, with A as a later bounded slice if the restated budget still needs it.
