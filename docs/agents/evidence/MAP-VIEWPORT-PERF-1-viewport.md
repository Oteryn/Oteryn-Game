# MAP-VIEWPORT-PERF-1 viewport evidence

Task `OTV2-20261005-map-viewport-perf-1`, packet §2.2. Gate: `MAP01-VIEWPORT-US` = 100 us p99 for one
18x14 domain-17 snapshot over all floors in view.

## Outcome

Gate not met. Round 1 optimised the owned paths (wire bytes unchanged). Round 2 (CP D757, owner consent for
`apps/game-server/src/gameplay_transport/item_view.rs`) removed the per-update `ItemHandleTable` clone from
`SessionItemView::map_view` (in-place replace with an undo log, set-difference passes). Snapshot p99 is still
above 100 us; per packet §1.1 the remaining stage split is reported to the CP.

## Machine and method

- 4 vCPU Intel Xeon @ 2.10GHz, rustc 1.94.0, `cargo test --release --locked -p oteryn-game-server map_viewport_measure -- --ignored --nocapture`.
- 20,000 seeded viewports, floors -7..=0. World seed 7, viewport rng `0x5eed_0007`. Golden seeds 11, 12, 13, 14.
- Delta = one-tile step from the previous viewport.

## Byte identity

`seeded_walks_send_the_bytes_recorded_before_the_viewport_optimisation` pins 324 updates, 5,431,963 bytes and
a sha256 digest over tag, revision, length and payload of every update. The constant was captured on the
unmodified code (commit 2d0aaff4) and passes unchanged after the refactor (1baacf09).

## Numbers

| | snapshot p50 | snapshot p99 | delta p50 | delta p99 |
|---|---|---|---|---|
| Before (packet baseline) | 2.04 ms | 3.47 ms | n/a | n/a |
| Before (old test, unmodified code, 2,000 samples, fixed origin 48,48,-7) | 2.108 ms | 4.009 ms (max 6.79 ms) | n/a | n/a |
| After round 1 (20,000 seeded viewports) | 1.628 ms | 3.117 ms | 1.193 ms | 2.726 ms |
| After round 2 (item_view.rs, same harness) | 1.195 ms | 2.098 ms | 0.888 ms | 1.505 ms |

The "before" run of the new 20,000-viewport test could not be completed (release build interrupted by
container restarts); the two before rows above are the available baselines.

## Stage split after

| stage | p50 | p99 |
|---|---|---|
| plan (compose + budget rank) | 343 us | 644 us |
| handle table (clone + replace) | 822 us | 1.554 ms |

Packet baseline split: planning 1.09 ms, handles and tiles 0.74 ms, encode 0.43 ms.

Round 2 stage rows (same harness): plan p50 313 us / p99 540 us; the harness row "handle table (clone +
replace)" p50 835 us / p99 1.445 ms still clones the table itself, so it measures the old pattern and not
`map_view`. Snapshot minus plan is about 0.88 ms p50: BTree inserts and removals of about 1,000 handles in
`by_key` and `by_handle`, the `BTreeSet` build, tile and item `Vec` allocations, and encode.

## Remaining cost (after round 1)

- Handle table: `SessionItemView::map_view` clones a BTree-based `ItemHandleTable` and runs `replace` per
  update (`item_view.rs`, not owned). About half of the snapshot.
- Per-tile `Vec<MapTile>` / `Vec<MapItem>` allocations are required by the protocol types, not owned.
- Even the plan stage alone (343 us p50) exceeds 100 us.

After round 2 the plan stage alone (313 us p50) and the BTree handle maps (about 1,000 issued and dropped handles
per snapshot) both exceed the 100 us gate. Reaching it needs a flat or hashed handle table plus a cheaper plan
stage, or a budget decision.
