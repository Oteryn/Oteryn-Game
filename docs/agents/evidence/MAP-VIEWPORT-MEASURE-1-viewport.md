# MAP-VIEWPORT-MEASURE-1 viewport evidence

Task `OTV2-20261006-map-viewport-measure-1`, decision `ARCH-MAP-VIEWPORT-BUDGET-V1` §1.2, §1.3 and §2, amended
by CP decision D823 (owned paths). A measurement: nothing fails on the numbers, no budget, registry, wire or
runtime change.

Command: `cargo test --release --locked -p oteryn-game-server --lib map_viewport_real -- --ignored --nocapture`
(`apps/game-server/src/gameplay_transport/world_map_real_tests.rs`).

## Machine and method

- Machine: Intel Xeon @ 2.80 GHz, 4 vCPU (the reference node class), release profile, one run.
- Bundle: the checked-in World Project, every placement region, compiled like `map_load_budget_compile_the_real_map`
  (`tests/support/real_map_bundle.rs`, a copy); 26,078,766 bytes, digest
  `06fa0c7ac71c9476d234e728865faacf74a5b01249f222082bdfebbf3e3632ff`, 25,984 palette keys, 19,373,519 tiles,
  24,983,331 entries, 5,818,415 enterable tiles.
- Seeds: `0x5eed0b01` (1 thread), `0x5eed0b02` (4), `0x5eed0b03` (8); each session derives its seed from it.
- Measured path: the server's own `SessionMapView` / `SessionItemView` over a `ChannelOverlay` of the loaded
  `WorldBase`, one shared base per run. Timings come from a staged copy of `SessionMapView::update` with a clock
  between the stages; the run first checks that it sends the same bytes as `update` over 400 steps.
- 504 views per concurrency level (D128: at least 500), each its own session: an admission snapshot, a walk of
  200 steps (one delta per step; a teleport tile ends the walk with a snapshot at its destination), then a floor
  change, a teleport and a resync (snapshot of a sent view).
- Positions: 60 of 100 starts from towns and spawn areas (spawn centres and cells, house entrances; 134,364 of
  143,490 sit on an enterable tile), the rest uniform over enterable tiles; walks keep a heading and turn one step
  in four. 309, 282 and 324 of 504 starts were anchored. Walk steps: 32% on the surface (-7..=0), 68%
  underground.

## Snapshot and delta (us)

| threads | update | n | p50 | p99 | max |
|---|---|---|---|---|---|
| 1 | snapshot, all kinds | 1885 | 267.3 | 646.0 | 2479.1 |
| 1 | delta (walk step) | 100800 | 152.7 | 387.9 | 4306.7 |
| 4 | snapshot, all kinds | 1869 | 280.1 | 790.8 | 6423.8 |
| 4 | delta (walk step) | 100800 | 158.0 | 445.3 | 12466.2 |
| 8 | snapshot, all kinds | 1862 | 269.9 | 4587.3 | 8755.3 |
| 8 | delta (walk step) | 100800 | 153.7 | 4355.0 | 16596.6 |

Snapshots by kind, p50 / p99 us (1 thread / 4 threads): admission 272 / 662 and 283 / 806; floor change 284 / 645
and 297 / 774; teleport 277 / 646 and 291 / 836; resync 242 / 578 and 253 / 753.

The 8-thread p99 (4.4-4.6 ms) is not work: 8 busy threads share 4 vCPU, so a thread is descheduled in about 1 of
100 updates. The p50 does not move (154 us), and the thread CPU time per update (below) is the same at 1, 4 and 8
threads. Read 1 and 4 threads as the latency, 8 threads as the oversubscription. Isolated maxima of 4-17 ms at 1
and 4 threads are scheduling too (one update in 100,000); they are reported, not filtered.

## Stage split (p50 / p99 us, 1 thread)

| stage | snapshot p50 | snapshot p99 | delta p50 | delta p99 |
|---|---|---|---|---|
| plan (compose the view from base and overlay) | 106.9 | 238.1 | 79.4 | 190.0 |
| handles | 0.2 | 9.4 | 0.2 | 2.1 |
| tiles | 35.5 | 99.5 | 31.6 | 94.0 |
| encode | 113.6 | 318.7 | 23.9 | 79.2 |
| total | 267.3 | 646.0 | 152.7 | 387.9 |

The stage p99s are of each stage on its own, so they do not add to the total p99. For a snapshot, plan and encode
are about 40% each; for a delta the plan is about 52% and the encode 16%: the delta cost is the composition of
the viewport that the delta then mostly discards.

## Bytes and snapshot share

- Mean payload: admission 27.8 kB, floor change 31.5 kB, teleport 26.2 kB, resync 26.3 kB (1 thread); delta
  2.3-2.4 kB.
- No walk step produced a snapshot: 0 of 100,800 walk updates. With the scenario snapshots (admission, floor
  change, teleport, resync), snapshots are 1.8% of all updates and, at about 270 us against 155 us each, about 3%
  of the update time.
- Floor change was unavailable for 152, 156 and 155 sessions at 1, 4 and 8 threads (no enterable tile one floor
  up or down within 3 tiles), so those sessions have no floor-change snapshot.

## CPU per player per second

Thread CPU time (`/proc/thread-self/schedstat`), one update every 200 ms (5 per second, an upper bound: walking
at base speed is slower):

| threads | CPU per update | per player-second | 500 players |
|---|---|---|---|
| 1 | 162.5 us | 813 us | 0.406 core |
| 4 | 174.0 us | 870 us | 0.435 core |
| 8 | 167.6 us | 838 us | 0.419 core |

## Channel writer (§1.3)

Not composed: `observe_world_map` is the trait default on `main` (`connection.rs`), so no authority composes a
bundle World's map yet and there is no Channel writer hold to measure. The harness reads the immutable base and
the overlay by shared reference only and holds nothing across the encode, so the writer cost of this path is zero
by construction; MAP-CUTOVER-1b has to confirm it on the composed path.

## Proposed production limits (§1.2)

20% of one core at 500 players is 400 us per player-second, 80 us per update at 5 updates per second. Measured:
about 165-175 us per update (0.41-0.44 core). **The map views do not fit the 20% default at 5 steps per second;
they fit at up to 2.4 steps per second per player** (400 / 165), which is a slower pace than a player at base speed
holds continuously but not an upper bound anyone should assume.

Proposal for the architect, derived from the 1 and 4 thread runs (the 4 vCPU node, not oversubscribed) with about
1.5x headroom over the 4-thread p99:

- `MAP01-VIEWPORT-SNAPSHOT-US` production: p99 1,200 us (measured 646-791; today's temporary row is 2,000).
- Delta row: p99 700 us (measured 388-445), plus a mean of 80 us per update only after an optimisation slice.

Under §1.2 the next step is a bounded optimisation slice, not a wire change. The split points at the plan stage
(composition, about half of a delta), not at the protocol encode, which is 16% of a delta; PERF-2 option A (the
encode) alone cannot halve the delta cost. A budget question to the owner (raise the share above 20%, or count
only moving players) is the alternative.

## Limitations

- Entry facts are approximate: the terrain kind and appearance come from the palette and Terrain records,
  `pickupable` from the item definitions; `bound`, `blocks_projectile` and `house_tile` are false and the overlay is
  empty (no added items, no object revisions). A live Channel adds overlay entries, so this is a floor on the cost.
- Floor-change tiles are not identifiable in the compiled bundle (the resolver reports none), so a floor change is
  an enterable tile one floor up or down within 3 tiles, not a real stair.
- `tests/support/real_map_bundle.rs` is a copy of the `map_load_base.rs` compile with dense ids (palette index +
  1); `map_load_base.rs` is untouched (D823), so the helper still exists twice.
- One run on one node; no repeat for variance.
