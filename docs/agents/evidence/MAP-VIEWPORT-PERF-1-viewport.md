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

Re-pin after merging main (3b753cba): the digest changed from `a7df36eb…c0fc` to `d9f1866f…5a01` while the update
count (324) and byte count (5,431,963) stayed equal. The same test applied to unmodified origin/main (3b753cba,
without this PR's code) yields the identical `d9f1866f…5a01`, so the difference comes from content or world changes
merged to main, not from the optimisation; the PR and unmodified main emit the same bytes.

## Numbers

| | snapshot p50 | snapshot p99 | delta p50 | delta p99 |
|---|---|---|---|---|
| Before (packet baseline) | 2.04 ms | 3.47 ms | n/a | n/a |
| Before (old test, unmodified code, 2,000 samples, fixed origin 48,48,-7) | 2.108 ms | 4.009 ms (max 6.79 ms) | n/a | n/a |
| After round 1 (20,000 seeded viewports) | 1.628 ms | 3.117 ms | 1.193 ms | 2.726 ms |
| After round 2 (item_view.rs, same harness) | 1.195 ms | 2.098 ms | 0.888 ms | 1.505 ms |

The "before" run of the new 20,000-viewport test could not be completed (release build interrupted by
container restarts); the two before rows above are the available baselines.

## Measurement correction (review, D742)

The harness preloaded the handle table with the planned keys before the snapshot timer, so the snapshot only
measured a warm no-change replace. The preload and the clone-based handle row are removed: each sample now
snapshots from the table state the previous sample's step left (a different viewport), the realistic case.
Re-measured with the final harness (20,000 seeded viewports, same machine, release):

| Measure | p50 | p99 |
|---|---|---|
| snapshot | 1.851 ms | 2.945 ms |
| one-tile-step delta | 0.881 ms | 1.426 ms |
| plan stage (compose + budget rank) | 307 us | 516 us |

The earlier rows above (round 1 and 2) were taken with the warm preload and understate the snapshot. The
round-1 and round-2 rows are therefore not comparable with the packet baseline; against it (snapshot p99 3.47 ms) the
re-measured snapshot p99 is 2.94 ms.
Snapshot minus plan is about 1.5 ms p50: BTree inserts and removals of about 1,000 handles in `by_key` and
`by_handle`, the `BTreeSet` build, tile and item `Vec` allocations, and encode. The gate is not met.

## Remaining cost (after round 1)

- Handle table: `SessionItemView::map_view` clones a BTree-based `ItemHandleTable` and runs `replace` per
  update (`item_view.rs`, not owned). About half of the snapshot.
- Per-tile `Vec<MapTile>` / `Vec<MapItem>` allocations are required by the protocol types, not owned.
- Even the plan stage alone (343 us p50) exceeds 100 us.

After round 2 the plan stage alone (313 us p50) and the BTree handle maps (about 1,000 issued and dropped handles
per snapshot) both exceed the 100 us gate. Reaching it needs a flat or hashed handle table plus a cheaper plan
stage, or a budget decision.
