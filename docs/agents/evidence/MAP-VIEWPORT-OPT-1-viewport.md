# MAP-VIEWPORT-OPT-1 — bounded codec measurement

## Source, authority and method

- Date: 2026-10-22. Continuation of WIP `da32209a86bebae5851c3278e75375ab75b10b52`
  over original task base `59eee9429228d50cbc2e3f5fa959509eb00c4a1c`.
  The packet's `23a036fa9` paths were reverified after ADR-0022.
- Bound META policy refreshed read-only at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
  No publication, external review, paid API invocation or production mutation.
- AMD Ryzen 7 9800X3D 8-Core Processor, 16 vCPU; shared host with other cargo
  lanes, not the reference 4-vCPU capacity node. All runs use the same content.
- Run-1 blocker: ADR-0022 moved the fixture's sole compiling consumer to
  `crates/gameplay/dispatch`; its two-level root join resolved to `crates/`.
  Owner/integrator option A authorizes only `repo_root()`'s `../../..` join
  and the stale mount-location comment. That is the entire fixture change.
- Baseline B1–B3: original codec plus the authorized fixture correction.
  Candidate C1–C3: final private codec refactor and adapted existing assertions.
  Codec SHA-256: `ba7eab1326ffc94bb7b187903864d05c0203e9a589d09c8a0c28222d4198b49a`.
- P is an additional preliminary candidate run before formatting the private
  `encoded_len` signature onto multiple lines; semantics were identical.
  Preliminary codec SHA-256:
  `f79260b934e87caf1a46b2c4b96687f67b52617997ffb7c878395acdc147ea89`.
  Formatting caused a rebuild, so three further consecutive stable-source runs
  C1–C3 were taken. P is reported and excluded only from the designated
  three-run aggregates, not hidden as an outlier.
- Sequence: B1, B2, B3, candidate edit/build, P, formatting rebuild, C1, C2,
  C3. No other local test/build workload overlapped timed measurements.
  Every run uses exactly:

```sh
cargo +1.95.0 test --locked --release -p oteryn-gameplay-dispatch --lib map_viewport_real_measure -- --ignored --nocapture
```

PROVEN: all seven invocations selected and passed one real measurement, zero
failures/ignored, 910 filtered out. Each passed the existing 400-step staged
wire equality comparison against `SessionMapView::update`. Nothing in the
measurement, its registration, map compiler or staged comparison was copied,
replaced or weakened. The existing ignored measurement was explicitly run.

## Identical workload

Bundle: 26,078,850 bytes; digest
`bb67df67d1605c42f9fbddcf18ab70abd86ee7087c17da9233d4b2c8c36bad06`;
25,984 palette keys; 19,373,519 tiles; 24,983,331 entries; 5,818,415 enterable
tiles. Seeds: `0x5eed0b01`, `0x5eed0b02`, `0x5eed0b03` for 1/4/8 threads.
Each profile has 504 views and 100,800 walk steps, 200 steps per walk. CPU
projection uses five updates/player/second and 500 players. CPU excludes
scheduling time; stage percentiles and maxima are wall-clock durations.

| Threads | Snapshots | Delta updates | Total updates | Admission/FloorChange/Teleport/Resync/Delta mean bytes | Anchor starts | Surface steps |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 1,857 | 100,800 | 102,657 | 26,659 / 30,367 / 26,001 / 25,973 / 2,313 | 309/504 | 29,313 |
| 4 | 1,895 | 100,800 | 102,695 | 27,646 / 29,872 / 26,378 / 26,369 / 2,397 | 282/504 | 29,800 |
| 8 | 1,872 | 100,800 | 102,672 | 26,505 / 30,970 / 26,139 / 26,106 / 2,318 | 324/504 | 30,600 |

All profiles had zero walk snapshots and stuck relocations. Scenario snapshots
were 1.81% / 1.85% / 1.82% of updates. Counts and mean byte sizes were identical across
all versions/runs. Production consumers remain `map_view_diff.rs`'s existing
encoders, reached through dispatch `world_map.rs`; actual domain-17 connection
tests cover join/step/resume and committed door use. The measurement's final
“not composed” diagnostic predates #1916 and is stale; it does not establish
current composition or Channel writer hold time. This harness measures
SessionMapView, not a full live Channel or network/persistence workload.

## Every run

All durations below are microseconds. Snapshot/delta triples are p50 / p99 /
max; encode pairs are p50 / p99. CPU and core figures retain the harness's
independent rounding. Snapshot means all scenario kinds; delta includes empty
walk updates (none occurred here).

| Run | Threads | Snapshot | Delta | Snapshot encode | Delta encode | CPU/update | Cores at 500 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| B1 | 1 | 137.8 / 296.5 / 638.9 | 83.9 / 171.9 / 2371.6 | 54.7 / 129.8 | 11.1 / 32.6 | 85.9 | 0.215 |
| B1 | 4 | 140.7 / 322.0 / 461.7 | 88.9 / 191.4 / 352.1 | 56.9 / 146.6 | 11.9 / 36.4 | 93.0 | 0.232 |
| B1 | 8 | 156.5 / 369.7 / 513.0 | 96.9 / 218.6 / 678.1 | 63.4 / 179.1 | 13.1 / 42.6 | 102.9 | 0.257 |
| B2 | 1 | 139.0 / 289.4 / 502.7 | 84.7 / 178.4 / 890.0 | 55.0 / 131.7 | 11.2 / 34.0 | 87.6 | 0.219 |
| B2 | 4 | 141.9 / 334.8 / 510.5 | 89.4 / 193.1 / 2391.8 | 57.2 / 146.4 | 11.9 / 36.6 | 93.3 | 0.233 |
| B2 | 8 | 168.0 / 388.2 / 1272.9 | 101.5 / 227.1 / 499.1 | 69.2 / 177.3 | 13.7 / 44.5 | 107.8 | 0.270 |
| B3 | 1 | 139.3 / 286.4 / 596.8 | 84.7 / 173.3 / 1870.5 | 54.3 / 124.5 | 11.2 / 33.0 | 86.4 | 0.216 |
| B3 | 4 | 150.4 / 366.8 / 1378.8 | 95.3 / 214.4 / 2314.0 | 60.1 / 155.0 | 12.8 / 40.7 | 100.2 | 0.250 |
| B3 | 8 | 167.7 / 373.4 / 802.9 | 102.7 / 236.9 / 1218.8 | 68.4 / 178.5 | 14.0 / 46.5 | 109.6 | 0.274 |
| P | 1 | 126.2 / 258.7 / 402.3 | 83.3 / 168.8 / 762.4 | 42.0 / 96.6 | 10.2 / 30.1 | 84.8 | 0.212 |
| P | 4 | 134.8 / 312.1 / 712.9 | 91.4 / 202.2 / 1851.3 | 45.2 / 117.4 | 11.4 / 35.7 | 95.9 | 0.240 |
| P | 8 | 151.5 / 337.8 / 481.4 | 102.7 / 222.8 / 623.0 | 52.8 / 134.2 | 13.0 / 41.1 | 107.5 | 0.269 |
| C1 | 1 | 199.4 / 931.2 / 7300.9 | 130.0 / 466.1 / 20271.7 | 68.9 / 224.2 | 17.2 / 70.1 | 140.2 | 0.351 |
| C1 | 4 | 197.5 / 608.5 / 16241.1 | 131.7 / 529.2 / 13990.2 | 69.2 / 230.9 | 17.5 / 78.4 | 145.5 | 0.364 |
| C1 | 8 | 197.2 / 943.8 / 18453.2 | 129.0 / 536.4 / 36233.2 | 69.2 / 224.6 | 17.0 / 64.7 | 139.2 | 0.348 |
| C2 | 1 | 128.9 / 268.8 / 451.0 | 84.7 / 173.0 / 1231.3 | 42.5 / 96.9 | 10.4 / 30.9 | 86.2 | 0.215 |
| C2 | 4 | 134.6 / 294.6 / 441.2 | 90.7 / 200.6 / 2506.6 | 44.8 / 110.2 | 11.3 / 35.2 | 94.8 | 0.237 |
| C2 | 8 | 148.2 / 355.4 / 726.1 | 98.7 / 216.6 / 667.6 | 50.8 / 138.7 | 12.4 / 40.5 | 104.3 | 0.261 |
| C3 | 1 | 128.9 / 268.6 / 606.4 | 84.3 / 172.6 / 1711.9 | 42.9 / 98.0 | 10.5 / 30.7 | 86.6 | 0.217 |
| C3 | 4 | 130.9 / 293.7 / 416.4 | 90.1 / 185.9 / 366.5 | 44.4 / 111.6 | 11.1 / 33.0 | 92.9 | 0.232 |
| C3 | 8 | 147.1 / 342.8 / 738.0 | 97.1 / 222.7 / 4281.3 | 50.0 / 137.8 | 12.2 / 40.3 | 103.6 | 0.259 |

## Repetition minima and medians

B aggregates B1–B3; C aggregates C1–C3, including the heavily contended C1.
Each pair is minimum / median across repetitions; triples of pairs are p50;
p99; maximum. These are repetition aggregates, not pooled sample percentiles.

| Version | Threads | Snapshot min/median: p50; p99; max | Delta min/median: p50; p99; max | CPU/update min/median | Cores min/median |
| --- | --- | --- | --- | --- | --- |
| B | 1 | 137.8 / 139.0; 286.4 / 289.4; 502.7 / 596.8 | 83.9 / 84.7; 171.9 / 173.3; 890.0 / 1870.5 | 85.9 / 86.4 | 0.215 / 0.216 |
| C | 1 | 128.9 / 128.9; 268.6 / 268.8; 451.0 / 606.4 | 84.3 / 84.7; 172.6 / 173.0; 1231.3 / 1711.9 | 86.2 / 86.6 | 0.215 / 0.217 |
| B | 4 | 140.7 / 141.9; 322.0 / 334.8; 461.7 / 510.5 | 88.9 / 89.4; 191.4 / 193.1; 352.1 / 2314.0 | 93.0 / 93.3 | 0.232 / 0.233 |
| C | 4 | 130.9 / 134.6; 293.7 / 294.6; 416.4 / 441.2 | 90.1 / 90.7; 185.9 / 200.6; 366.5 / 2506.6 | 92.9 / 94.8 | 0.232 / 0.237 |
| B | 8 | 156.5 / 167.7; 369.7 / 373.4; 513.0 / 802.9 | 96.9 / 101.5; 218.6 / 227.1; 499.1 / 678.1 | 102.9 / 107.8 | 0.257 / 0.270 |
| C | 8 | 147.1 / 148.2; 342.8 / 355.4; 726.1 / 738.0 | 97.1 / 98.7; 216.6 / 222.7; 667.6 / 4281.3 | 103.6 / 104.3 | 0.259 / 0.261 |

Median stage p50/p99 across the same three repetitions (microseconds):

| Version | Threads | Stage | Snapshot p50 | Snapshot p99 | Delta p50 | Delta p99 |
| --- | --- | --- | --- | --- | --- | --- |
| B | 1 | plan | 60.7 | 127.3 | 46.3 | 94.3 |
| B | 1 | handles | 0.1 | 3.8 | 0.1 | 0.9 |
| B | 1 | tiles | 16.9 | 39.3 | 15.5 | 37.0 |
| B | 1 | encode | 54.7 | 129.8 | 11.2 | 33.0 |
| C | 1 | plan | 62.2 | 139.4 | 47.1 | 94.2 |
| C | 1 | handles | 0.2 | 3.6 | 0.1 | 1.0 |
| C | 1 | tiles | 17.1 | 37.7 | 15.8 | 37.2 |
| C | 1 | encode | 42.9 | 98.0 | 10.5 | 30.9 |
| B | 4 | plan | 61.5 | 147.4 | 49.4 | 103.4 |
| B | 4 | handles | 0.2 | 3.4 | 0.1 | 0.9 |
| B | 4 | tiles | 17.4 | 45.2 | 16.5 | 40.0 |
| B | 4 | encode | 57.2 | 146.6 | 11.9 | 36.6 |
| C | 4 | plan | 64.3 | 155.6 | 50.3 | 109.7 |
| C | 4 | handles | 0.2 | 3.5 | 0.1 | 1.0 |
| C | 4 | tiles | 17.7 | 41.5 | 16.9 | 44.3 |
| C | 4 | encode | 44.8 | 111.6 | 11.3 | 35.2 |
| B | 8 | plan | 71.7 | 168.1 | 56.3 | 120.9 |
| B | 8 | handles | 0.2 | 3.7 | 0.1 | 1.2 |
| B | 8 | tiles | 20.3 | 51.2 | 18.5 | 49.4 |
| B | 8 | encode | 68.4 | 178.5 | 13.7 | 44.5 |
| C | 8 | plan | 70.4 | 166.8 | 55.4 | 120.7 |
| C | 8 | handles | 0.2 | 3.5 | 0.1 | 1.2 |
| C | 8 | tiles | 19.5 | 51.8 | 18.2 | 49.0 |
| C | 8 | encode | 50.8 | 138.7 | 12.4 | 40.5 |

## Retention decision and residual gap

DERIVED: retain the bounded codec change for the real-path snapshot benefit.
Snapshot encode p50 min/median falls from 54.3/54.7 to 42.5/42.9 us at one
thread; median reductions at 1/4/8 threads are 21.6% / 21.7% / 25.7%.
Whole-snapshot median p50 falls 139.0→128.9, 141.9→134.6 and 167.7→148.2 us
(7.3% / 5.1% / 11.6%). The one-thread 10.1-us median snapshot saving exceeds
the baseline p50 repetition range of 1.5 us; the unaffected plan stage's
median is slightly slower, 60.7→62.2 us. This supports a localized codec
benefit rather than a globally faster host during candidate runs.

C1 is retained in every aggregate: its map compile took 29.2 s instead of
baseline 11.3/11.6/11.4 s; plan and total costs also rose sharply. Candidate
C2/C3 compile times were 11.7/12.4 s (P: 11.5 s). Its 36,233.2-us delta
maximum illustrates why maxima are not a capacity acceptance claim here.

DERIVED: no demonstrated aggregate delta or CPU improvement above host noise.
Median CPU/update B→C is 86.4→86.6, 93.3→94.8 and 107.8→104.3 us.
Candidate median core estimates 0.217 / 0.237 / 0.261 exceed the 0.200 budget
by 0.017 / 0.037 / 0.061 cores (8.5% / 18.5% / 30.5%). The full
MAP-VIEWPORT-OPT-1 capacity row stays open. No production limit, registry,
capability offer or budget changes are justified or made. Further composition,
overlay or capacity work requires its own allocation; none was attempted.

## Implementation, byte checks and reuse

The existing private tile sizer now validates item/tile byte bounds while
computing lengths; `encoded_len` propagates those checks in its existing
allocation-sizing pass. The old standalone `check_tile_bytes` pass is deleted.
Serialization reuses the same checked tile sizer. Valid-path item sizing falls
from five evaluations to three, with no new allocation, dependency or public
API. Whole-message semantic validation still precedes byte bounds, preserving
rejection precedence, ordering, exact output bytes and every registered limit.

The WIP's existing `encoded_lengths_match_raw_fields_across_varint_boundaries`
test is retained: 150 snapshot and 150 delta vectors compare generic raw fields
against actual bytes and lengths, including varint boundaries and all origin
forms. Existing malformed/max/max+1 assertions remain, with explicit success
checks where private sizing/serialization now returns Result.

Reused: `varint_len`, `field_len`, `framed`, `item_len`, `tile_len`,
`encoded_len`, `push_*`, `check_snapshot`, `check_delta`; existing test
builders `position`, `header`, `item_ref`, `largest_tile`, `far_origin`,
`raw_position`, `raw_item`, `snapshot_with_raw_tile`, `snapshot_round_trip`
and generic `push_message_field`/`push_varint_field`; existing
`compile_real_map`, `staged_update`, `staged_equals_update`,
`map_viewport_real_measure` and actual connection tests, all registered once.
No wrapper/re-export, harness, helper, pub fn, enum variant, struct field or
trait method was added. No file grew by more than 200 lines over the original
task base. Exact correctness/check/clippy/governance results are in the task card.
