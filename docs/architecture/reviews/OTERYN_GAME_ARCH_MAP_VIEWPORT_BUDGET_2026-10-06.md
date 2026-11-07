# ARCH-MAP-VIEWPORT-BUDGET-0 Map viewport budget: assembly, a temporary snapshot gate, a real measurement

- Decision: `ARCH-MAP-VIEWPORT-BUDGET-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES**, after exact-head validation, the independent
  review on the frozen head and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane (2026-10-06, #1859 frozen at `ef73d927`): rule on
  `MAP01-VIEWPORT-US` after MAP-VIEWPORT-PERF-2 measured the whole snapshot at 1.683 ms p99.
- Owner answer **1a** (2026-10-06): a temporary 2 ms snapshot gate, on testing and
  preproduction nodes only; before production the limit comes from a measurement and D128.
- Amends: `ARCH-MAP-TRACK-PACKETS-V1` §1.1 and §2.4 (pointer note there), the capability 18
  `offer_gate` in `PROTOCOL_OTERYN_V1_REGISTRY.json`, `RESOURCE_LIMITS_REGISTRY.json`.
- Runtime, migration, production and protected-World authority: NONE. MAP-VIEWPORT-MEASURE-1
  needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **This PR.** `MAP01-VIEWPORT-US` stays 100 us p99, the assembly of one 18x14 viewport from
   base and overlay (owner answer 8a, ADR-0021 §4.8; MAP-LOAD-1 measured 72-83 us on the real
   map). A new row `MAP01-VIEWPORT-SNAPSHOT-US`, 2,000 us p99, gates the whole domain-17
   snapshot (compose, assign handles, encode) for the capability 18 offer on testing and
   preproduction nodes only. The capability 18 `offer_gate` names both.
2. **MAP-VIEWPORT-MEASURE-1 (impl), §2.** Measures snapshot and delta on the real map with
   realistic positions, walks and concurrency, and derives the production snapshot and delta
   limits from D128 (500 players per Channel). No production offer of capability 18 before
   those limits are registered and met.
3. **#1859 (MAP-VIEWPORT-PERF-2)** proceeds unchanged: its 1.683 ms p99 is within the new row.
   It does not claim `MAP01-VIEWPORT-US`.

## 1. Rulings

### 1.1 Two budgets, two resources

`MAP01-VIEWPORT-US` was set (owner answer 8a) and measured (MAP-LOAD-1) as the cost of
assembling a viewport from the base and the overlay. ARCH-MAP-WIRE §2.1 and the capability 18
`offer_gate` later bound the same 100 us to composition plus encode of a wire snapshot, a
different and larger resource: handle assignment and the wire encode were never in the 8a
figure. MAP-VIEWPORT-PERF-1 and PERF-2 show the whole snapshot at 2.9 ms and then 1.683 ms
p99, with the protocol encode about half of it (PERF-2 callgrind split). So:

- `MAP01-VIEWPORT-US` keeps its registered resource and value. Assembly stays at 100 us p99
  and is still checked by MAP-LOAD-1's method.
- `MAP01-VIEWPORT-SNAPSHOT-US` registers the whole snapshot. Its 2,000 us p99 is temporary
  and holds on testing and preproduction only (owner answer 1a). It is measured with the
  release `map_viewport_measure` test on the reference node class (MAP-SPIKE-0, 4 vCPU), the
  §1.1 method of `ARCH-MAP-TRACK-PACKETS-V1`.
- The wire bytes are not changed to meet either budget.

### 1.2 Production needs a measured limit

The 2 ms figure is not a production capacity statement. The synthetic harness (seed 7, every
third tile holding 3 items, uniform floors -7..=0, one thread) does not show what a Channel of
players costs. Before any production offer of capability 18:

- MAP-VIEWPORT-MEASURE-1 (§2) reports snapshot and delta p50/p99/max and the per-player CPU
  of map views on the real map;
- the architect derives `MAP01-VIEWPORT-SNAPSHOT-US` and a delta row from it and D128: at 500
  players per Channel, the map views of a Channel take at most **20% of one core** (architect
  default, a reversible detail; the rest belongs to movement, combat, AI and the other
  domains). The rows become production limits only through a decision that cites the evidence;
- if the measured cost does not fit, the next step is a bounded optimisation slice (the
  `crates/protocol-oteryn` encode, PERF-2 option A) or a budget question to the owner, never a
  wire change made only to pass.

### 1.3 Where the view work runs

`observe_world_map` composes and encodes for one session with that session's `SessionMapView`
and `SessionItemView`. MAP-CUTOVER-1b keeps this work off the Channel's single writer: the
connection task reads the immutable shared `WorldBase` and the Channel overlay through a read
that is not held across the encode, so a snapshot of one player never delays the tick of the
others. MAP-VIEWPORT-MEASURE-1 checks this on the composed path and reports the time the
Channel writer is held per update.

### 1.4 What #1859 claims

MAP-VIEWPORT-PERF-2 lowers the snapshot from 3.264 ms to 1.683 ms p99 with byte-identical
output. Under this decision it meets `MAP01-VIEWPORT-SNAPSHOT-US`; its evidence and record
need no change. The CP merges it on its own review state.

## 2. Packet MAP-VIEWPORT-MEASURE-1 (impl worker)

```yaml
task_id: OTV2-20261006-map-viewport-measure-1
decision: ARCH-MAP-VIEWPORT-BUDGET-V1 §1.2, §1.3
depends_on: [OTV2-20261006-map-viewport-perf-2, this decision]
worker: oteryn-impl-worker
review: Codex, on the frozen head
branch: agent/map-viewport-measure-1-20261006
base: main
owned_paths:
  - apps/game-server/tests/map_viewport_real.rs
  - apps/game-server/tests/support/real_map_bundle.rs        # shared real-map compile, moved from map_load_base.rs
  - apps/game-server/tests/map_load_base.rs                   # uses the moved helper only
  - docs/agents/evidence/MAP-VIEWPORT-MEASURE-1-viewport.md
  - docs/agents/tasks/archive/OTV2-20261006-map-viewport-measure-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server --test map_load_base
  - cargo test --locked -p oteryn-game-server --test map_viewport_real
  - cargo test --release --locked -p oteryn-game-server --test map_viewport_real -- --ignored --nocapture
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Scope:** a release, `#[ignore]` measurement over the real map, compiled the way
  `map_load_budget_compile_the_real_map` does (the helper moves to `tests/support/` unchanged).
  If a needed type is crate-private, the worker returns `QUESTION` with the smallest `pub`
  test seam instead of widening visibility on its own.
  - **Positions:** enterable tiles only, weighted by where players are: towns and spawn areas
    of the World Project (the spawn and house families the compiler reads) at least half the
    samples, the rest uniform over enterable tiles; floors as they fall, the surface (-7..=0,
    eight floors in view) and underground (-8 and below, up to five floors in view).
  - **Deltas:** seeded walks of at least 200 steps from those positions over enterable tiles,
    each step one delta; a floor change or a teleport tile ends a walk with a snapshot.
  - **Snapshots:** admission (first view), floor change, teleport, and a resync from a sent
    view, each counted separately.
  - **Concurrency:** 1, 4 and 8 threads, each with its own sessions and views over one shared
    base, at least 500 views in total (D128), on the 4 vCPU reference node class.
  - **Report:** p50, p99, max for snapshot and delta; the per-stage split (plan, handles,
    tiles, encode); bytes per update; the snapshot share of updates in the walks; CPU per
    player per second at a step every 200 ms (Tibia walking pace at base speed is slower, so
    this is an upper bound); and, if `observe_world_map` is composed on `main` by then, the
    time the Channel writer is held per update (§1.3), else `not composed`.
- **Acceptance:** the evidence file records the machine, the bundle digest, the seeds, the
  numbers above and a proposed snapshot and delta p99 for production under §1.2 (20% of one
  core at 500 players); `map_load_base` passes unchanged after the helper move. The run fails
  nothing on the numbers: it is a measurement. A later live-bot run on the MAP-CUTOVER-1a/1b
  node is a separate packet after MAP-CUTOVER-1b.
- **Not in scope:** any runtime, wire, registry or budget change; any optimisation.

## 3. Rejected options

- Restating `MAP01-VIEWPORT-US` to the snapshot at a higher value: it would change an owner
  budget for a resource 8a did not cover and lose the assembly check MAP-LOAD-1 proved.
- Waiting for the measurement before any bundle World is served (owner option 1b): blocks the
  playable path on testing for a number that only production needs.
- Lowering snapshot frequency instead of cost (PERF-2 option C): snapshots already occur only
  on admission, floor change, teleport and resync; it does not change their p99.
- Optimising `crates/protocol-oteryn` now (PERF-2 option A): no accepted requirement needs it
  before the measurement says so.

## 4. Decision test

The decision holds if: `MAP01-VIEWPORT-US` is still 100 us for assembly; capability 18 is
offered on a testing or preproduction bundle World only while the snapshot p99 is within
`MAP01-VIEWPORT-SNAPSHOT-US`; no production World offers capability 18 before a later decision
registers snapshot and delta limits derived from MAP-VIEWPORT-MEASURE-1 and D128; and no wire
byte changed to meet any of these.
