# CREATURE-MOVE-1 — creature step timer, paths, goals and relocation (consolidated task card)

```yaml
task_id: CREATURE-MOVE-1
title: "CREATURE-MOVE-1: speed step timer, A* paths, walk back, leash, dance, near-target, relocate"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 2
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "App. 7 LOOP-MONSTER-1 graph, CREATURE-MOVE-1 residue"
brief: "docs/architecture/reviews/OTERYN_GAME_CREATURE_MOVE1_STEPS_PATHS_AND_GOALS_PACKET_2026-10-04.md (§1.1–§1.7)"
class: "hard (movement); movement and performance review"
base_branch: main
branch: w2/creature-move-1
pr: 2305
needs_migration: false
needs_registry: true    # accepted RL-09/10/11/17 rows were absent from the baseline
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **DONE**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2305](https://github.com/Oteryn/Oteryn-Game/pull/2305). Native creature speed timers, bounded paths, movement goals and measured owner-window work are merged. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs`.

Tracking closeout: delivered on main; no active implementation ownership is retained by this historical card.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2184, branch skeleton/creature-move-1

## Goal

Close the CREATURE-MOVE-1a/1b deferrals (packet §1.1–§1.5): creatures step on their own
speed-based step timer instead of one step per think, follow bounded A* paths (reusing the
CHASE-1 search), and run the chase / keep-distance / dance / flee / wander / walk-back /
stand goals, the near-target ×2 duration, the leash and `relocate_creature`. Player and
creature searches share one per-Channel work budget. Deterministic and replay-identical.

## History (archived records)

- CREATURE-MOVE-1a (#1977) and -1b (#1987): `docs/agents/tasks/archive/OTERYN-20261011-creature-move-1.md`
  and `docs/agents/tasks/archive/OTERYN-20261011-creature-move-1b.md`. They admit one greedy
  cardinal step per think (1 s) onto a walkable, non-PZ, non-floor-change, door-free cell
  through `monster_ai_cycle.rs::creature_cell_admissible`, and refresh position after the
  think pass. Deferred: everything in this card; the bundle-World monster cycle went to
  SPAWN-1b (#1981) / BOOT-RAT-POLICY-1 (#2185).
- Packet record: `docs/agents/tasks/archive/OTV2-20261004-creature-move-1-packet.md` (#1765).

## Accepted rules (packet, binding)

- **§1.1** The step timer replaces the per-think step; at most two pending AI timers per
  creature (one think, one step). The CREATURE-AI-1 §1.1 greedy step and §1.4 wander step are
  removed; their goals move to §1.5.
- **§1.2** Creature speed: no draw; native base speed plus the active condition delta; base
  speed 0 is immobile (no manufactured floor).
- **§1.3** Step duration: the same speed table, ground provider and 50 ms beat as players.
  Each chase or keep-distance evaluation within 1 tile of the target adds 1 to a near counter
  (max 2), doubling the duration; fleeing never applies ×2.
- **§1.4** Paths: four cardinal neighbours N/E/S/W, cost 10 per step, +40 for a creature on
  the tile, ties by total, estimate, neighbour order, discovery order; ≤ 1,024 expansions
  (`AI01-PATH-SEARCH-WORK`), ≤ 128 steps; search distance 12 (chase) and 50 (walk back).
  Walkability is the Movement owner's own read-only step-admission predicate (base map,
  overlay, never PZ/floor-change/teleport; creatures never open doors) — no second table.
  Harmful field cost +180 waits for a field owner (term is 0; a test pins +180 with a stub).
  One request per think (RL-10), one pending per creature; a request binds (creature
  `ExactActorRef`, bundle revision); an adopted path binds (creature, goal, bundle revision);
  dropped when the target changes or a step is refused. A failed chase search means "cannot
  be reached" and triggers a target search next think. `ai/path_proposal.rs` stays the
  immutable route carrier.
- **Budget (RL-09, R3):** 65,536 work units per Channel per 50 ms window, one unit per
  expansion; order: pending creature searches by deadline then `ExactActorRef`, then player
  chase (RANGED0-RL-03). **RANGED-0 §8 amendment:** 16,384 units (16 of 64 searches) reserved
  for player chase while a chase search is waiting; unused reserve returns to creatures in
  that window. Over-budget requests wait; the creature keeps a still-valid path or holds.
- **§1.5** Goals: chase (path to an adjacent tile; adjacent with a clear step holds), keep
  distance (Canary `getDistanceStep`), dance (`AI_DANCE` draw, chance
  1 − `static_attack_chance_ppm`, side step keeping adjacency), flee (local step away within
  11 tiles; cornered side step), wander (`AI_WANDER` within `radius_tiles`, every
  `interval_ms`, through the step timer), walk back (path to spawn cell, distance 50; on
  arrival idle; failure idles until next wake, R5), stand (speed 0).
- **Leash:** more than 50 tiles in x or y, or 2 floors, from the spawn cell → relocated at the
  think to the spawn cell, then idle; if the cell cannot admit it, idle in place. Creatures
  without a spawn point (AI-2 test path, summons) have no leash and no walk back.
- **`relocate_creature`** in Movement: moves the creature's own `ExactActorRef` to one cell,
  admits the cell with the step predicate, refuses a player actor, commits through the same
  owner commit path as one position change. No other caller.
- **§1.7** RL-17 measurement of the integrated loop.
- CP allocations for the draft: D1123, D1212, D1219, D1229 (player drain and shared ledger
  under one owner); D1380 holds #2200's AI wiring until this task merges.

## Reference behaviour from the draft (re-implement, do not copy)

- Residue confirmed: #2113 supplies the bounded cardinal A* and player chase; #2125 the
  canonical oldest-window/actor-id ordering and per-window drain; `movement/speed.rs` already
  has native base speed, the condition delta, speed-0 refusal and `creature_step_duration`
  (moved-only cadence). Reuse all of these.
- Search extended to the exact spawn cell (50-tile walk back, existing 1,024/128 limits);
  the near counter and duration multiplier per §1.3, threaded into cadence.
- Creature chase uses the native Movement cadence after a bounded route search; actual
  expansions are charged with player/creature refunds; declared creature work is protected;
  walk-back searches are counted; a budget-stopped player drain reopens while keeping its
  64-search count.
- **Review fix (planned occupancy, PR thread r4303886928):** a resumed player drain must
  keep the +40 occupancy of first steps already selected earlier in the same window. The
  chase retains its search window and start cell so the resumed drain recovers the original
  first destination, visible while stored and after `next_step` returns it until
  `paced_step` commits; then standing occupancy applies. An older window contributes no plan.
  Regression: uninterrupted vs suspended drains in both caller orders, observing both
  returned steps and the second chaser's following waypoint.
- Draft tests: eight Movement regressions (incl. the +180 field stub comparing occupancy
  alone vs occupancy + 180 on a detour), a six-cell chase fixture, the resumed-drain case.

## Cross-domain findings (integration slot)

- `ai_think.rs::CreatureAiState` and `ai_think/channel_owner.rs::run_taken_thinks` still
  choose and commit a step per think. They need accepted goals, per-creature near-counter
  state, generation/goal/content-bound `PathProposal` adoption, refusal invalidation and
  target re-search. `ai/resolution.rs` bootstrap resolution is not the runtime goal owner.
- The owner timer lane (`foundation/owner_timer.rs`) and the creature AI table need one
  `DEADLINE_STATE` step family beside think and idle cancellation.
- `combat/attack/chase.rs::ChaseSearchBudget` and
  `gameplay_transport/attack.rs::drain_chase_searches` drain up to 64 player searches;
  integration composes the shared RL-09 budget and reserve, keeping #2125 ordering. No
  parallel queue.
- `movement.rs` lacks `relocate_creature` and a shared read-only creature destination
  admission entry (map, overlay, forbidden tiles); leash/walk back need the authoritative
  spawn cell, not a generic admission anchor.
- `monster_ai_cycle.rs::creature_cell_admissible` holds the playable restrictions; search and
  execution must use the same Movement authority. `monster_combat_lane.rs` consumes the
  cadence and keeps its summon/ordinary-creature boundary.
- Dance, keep-distance fallback, flee/wander pacing, relocation and RL-17 remain unbuilt in
  the draft.

## Dependencies (re-verified against main)

| Dependency | State |
|---|---|
| CREATURE-AI-1 | MERGED (#1813) |
| SPEED-1 | MERGED (#1715) |
| CREATURE-MOVE-1a / 1b | MERGED (#1977, #1987) |
| CHASE-1 path search / ordering | MERGED (#2113, #2125) |
| CREATURE-MOVE-1 packet | MERGED (#1765) |
| SPAWN-1b, BOOT-RAT-POLICY-1 | MERGED (#1981, #2185) |
| Field owner (+180 cost, `walks_on_*`) | not started; term stays 0 |

No blockers. LOOP-MONSTER-1 (#2200 draft) consumes this task's caller.

## Target layout (ADR-0022)

| Concern | Path |
|---|---|
| Search profile, walk-back target, field term | `crates/gameplay/world/src/movement/path_search.rs` (417 lines), with compact labels in `path_search/path_labels.rs` and bounded tile slots in `path_search/path_tiles.rs` |
| Step duration, near ×2 | `crates/gameplay/world/src/movement/speed.rs` (469 lines) |
| `relocate_creature`, creature destination admission | `crates/gameplay/world/src/movement/relocate.rs` (new), registered from `crates/gameplay/world/src/movement.rs` (1,365 lines: wiring only) |
| Goals and near counter | `crates/gameplay/monster-ai/src/ai/movement_goal.rs` (new); state field in `crates/gameplay/monster-ai/src/ai_think.rs` (1,120 lines: call sites only) |
| Route carrier / adoption | Existing immutable `ai/path_proposal.rs` reused; adoption in `ai_think/creature_steps/creature_paths.rs`, goal/content/map fencing in `creature_goal_adoption.rs` |
| Step timer family | Existing `foundation/owner_timer.rs` reused unchanged; `StepFamily` and `DEADLINE_STATE` policy in `monster-ai/src/ai_think/creature_steps{,/creature_step_timer}.rs` |
| Shared budget / player drain | `crates/gameplay/combat/src/combat/attack/chase.rs` (581 lines), `crates/gameplay/dispatch/src/gameplay_transport/attack.rs` (1,925 lines: call sites only) |
| Cycle composition | `crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs` |
| Tests | Mechanic child suites under the existing owner; World relocation tests mount from `movement.rs::tests` to reuse its private runtime fixture |

## Acceptance criteria

- [x] A creature steps at its own speed on the step timer; near ×2 per §1.3; speed 0 stands.
- [x] A* paths bounded per §1.4 limits; walkability from the Movement predicate only; +180 field stub pinned.
- [x] Shared RL-09 window with the 16,384-unit player reserve; resumed drains keep planned occupancy.
- [x] Goals chase / keep distance / dance / flee / wander / walk back / stand per §1.5; leash and `relocate_creature` per packet.
- [x] Deterministic draws (`AI_DANCE`, `AI_WANDER`), replay identical; integrated release RL-17 p99 13.583111 ms in the latest continuation, below the unchanged 15 ms budget.
- [x] Structure: one mechanic per module file in its ADR-0022 layer.
- [x] Structure: new tests live in their own child files, not appended to existing files.
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines (`movement.rs`, `ai_think.rs`, `attack.rs` take call sites only).

## Local validation — 2026-10-22

Base: `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`; continuation from committed WIP `71b57e42103c4746e9873d1d9141a851c7f1d87e`, one writer on the allocated worktree. No commit, push, PR publication, migration or capability change. Every command below used `/home/mole/oteryn-cache/codex-bin/cargo` (+1.95.0, locked/offline resolution and wrapper PostgreSQL); the new PG scenario asserts configuration instead of silently skipping.

`cargo +1.95.0 fmt --all -- --check`: PASS. `cargo +1.95.0 check --locked -p oteryn-gameplay-world -p oteryn-gameplay-monster-ai -p oteryn-gameplay-dispatch -p oteryn-gameplay-combat -p oteryn-gameplay-combat-rules -p oteryn-gameplay-entities --quiet`: PASS. `cargo +1.95.0 clippy --locked -p oteryn-gameplay-world -p oteryn-gameplay-monster-ai -p oteryn-gameplay-dispatch -p oteryn-gameplay-combat -p oteryn-gameplay-combat-rules -p oteryn-gameplay-entities --all-targets --quiet -- -D warnings`: PASS. Cargo manifests did not change, so no workspace clippy/check was run. The unchanged vendored Tokio missing-doc warning is outside these packages; Clippy exits 0.

Each row is the exact `cargo +1.95.0 test --locked <package/flags> <filter> --quiet` invocation:

| Package / flags | Filter | Result |
|---|---|---|
| `-p oteryn-gameplay-world` | `movement::` | 64 pass, including the existing eight Movement regressions and two compact-label/cache regressions |
| `-p oteryn-gameplay-world` | `creature_path_tests` | 4 pass after final assertion cleanup |
| `-p oteryn-gameplay-world` | `map::spell_tiles` | 5 pass, 1 existing measurement ignore |
| `-p oteryn-gameplay-monster-ai` | `ai` | 69 pass, 2 measurement ignores; RL-17 explicitly run below |
| `-p oteryn-gameplay-combat -p oteryn-gameplay-combat-rules` | `chase` | 10 + 5 pass |
| `-p oteryn-gameplay-entities --features test-support` | `movement` | 7 pass |
| `-p oteryn-gameplay-entities --features test-support` | `spawn` | 40 pass |
| `-p oteryn-gameplay-dispatch` | `attack` | 55 pass; both caller orders and following waypoint across suspended drains |
| `-p oteryn-gameplay-dispatch` | `monster_ai_cycle` | 12 pass |
| `-p oteryn-gameplay-dispatch` | `creature_step_tests` | 1 pass after fixture borrow cleanup |
| `-p oteryn-gameplay-dispatch` | `movement_speed_runtime_tests` | 2 pass; real condition delta/expiry, unknown ground and stale generation |
| `-p oteryn-gameplay-dispatch` | `creature_movement_dispatch_steps_between_thinks_on_postgres` | 1 pass; real composed dispatch, source spawn, native speed deadline and leash relocation |

`python3 tools/agents/validate_governance.py`: PASS. `python3 -m unittest discover -s tools/agents/tests`: 124 pass. `python3 tools/repository/validate_repository_policy.py`: PASS. `git diff --check`: PASS. Whole-diff D1462 review against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and constructor/match/impl search over `apps/`, `crates/`, `tools/`: complete; old runtime think-step commits and their callers were removed. Tests moved to child files retain one registration each.

## RL-17 release qualification — 2026-10-22

Integrator decision: the **15 ms** budget and every think/search limit stay unchanged. Debug timings do not qualify RL-17.

The unchanged integrated test measures 200 windows on the 128×128 bundle fixture with obstacles; every window asserts 1,024 thinks and exactly **65,536 charged expansions** (64 full 1,024-expansion searches). Host: AMD Ryzen 7 9800X3D, 16 logical CPUs. All timing commands exit 0; timing remains on-demand evidence rather than a CI timing assertion.

| Candidate | Build | p99 |
|---|---|---:|
| Committed WIP (previous run) | debug | 750.348712 ms |
| Committed WIP (this continuation, before repairs) | release | 66.124154 ms |
| Final implementation | debug | 174.784910 ms |
| Final implementation | release | **13.667385 ms — PASS ≤15 ms** |

Exact final commands (the release command was also used before repairs):

```sh
/home/mole/oteryn-cache/codex-bin/cargo +1.95.0 test --release --locked -p oteryn-gameplay-monster-ai creature_movement_rl17_integrated_window_p99 --quiet -- --ignored --nocapture
/home/mole/oteryn-cache/codex-bin/cargo +1.95.0 test --locked -p oteryn-gameplay-monster-ai creature_movement_rl17_integrated_window_p99 --quiet -- --ignored --nocapture
```

`perf record -e cpu-clock -F 997 --call-graph dwarf -o .rl17-perf.data -- <release command above>` profiled the integrated window. Self samples identified A* bookkeeping (30.9% in `measured_search`, plus 10.4% in B-tree insertion) and repeated map admission/index lookups. Temporary local profiling traces were removed after analysis.

World now stores step-aware labels and predecessors contiguously, with direct tile slots for the accepted 12/50-tile searches and sparse storage for wider existing callers. The same A* heap still orders by total, estimate, cardinal neighbour and discovery; the old tree-based implementation was removed. Tile admission/cost is read once per tile per search, including refusals. The creature path drain reuses the authoritative Movement admission result across requests only while map/position facts are immutable; the next drain and every step commit read fresh facts. Existing adopted paths, invalidation, shared budget/reserve and expansion charges are retained. No dependency or public API was added.

New child regressions reuse `at`/`open`/`walk`: repeated labels read each tile once; the next search sees a formerly refused target reopen; route ties and checked coordinates hold at both `i32` plane edges. Existing shorter/dearer-label, 1,024/128 bounds, +180 field stub, resumed-player-occupancy, refusal and replay tests all pass. The actual dispatch/PG scenario reaches the optimized search through the production composition; existing `rust_linux` core unit execution (`not test(content::)`) includes it.

Independent movement/determinism/performance review, CI, freeze/publication and MQ remain CP-owned. The field-owner +180 production term remains 0 by the accepted deferral; the stub regression pins +180 against occupancy alone. No RL-17 owner decision remains pending.

## Delivered layout and reuse

World owns shared A*, occupancy, work ledger and destination admission/relocation. Monster AI owns goal selection/adoption and the independent step family. Entities adds only exact creature compare-commit and spawn-origin reads. Combat extends its existing chase state/count; Dispatch composes those owners. Shared slot files receive wiring only.

- Reused the existing cardinal A* bounds, tie ordering and step-aware dominance rules; extended its goal/measurement parameters without a second search. The continuation replaces its allocation-heavy state storage in World child modules, preserving the existing search API and callers.
- Reused `creature_step_duration`, native effective speed/condition reads, `StepSpeedTable`, ground providers and `PacingTime::checked_after`; base speed 0 admits neither a search nor a step.
- Reused `OwnerTimerLane`, `DeadlineState`, current owner stamps/fences and the existing one-think-per-creature table; no bespoke timer queue.
- Reused immutable `PathProposal`/`PathRequest`/`RouteStep`, native profile qualification, the existing sight trace, carrier compare-commit and `locate_spawn_cell`/`spawn_definition`.
- Moved the old creature admission rule into World and the existing standing census collector into shared Movement code; removed their former implementations. Existing player queue/order/count remains the only player drain.
- Reused private `at`/`open`/`walk` and Movement runtime fixtures by mounting child tests under their owner, including the new compact-label/cache suite. No fixture body was copied and the continuation needs no wrapper or re-export. Reused the existing AI `Channel`, `rat`/`abilities` and actor fixture via the four-line `movement_fixtures.rs` wrapper.
- Reused the existing bundle builder/palette/boot through `movement_arena.rs`; existing native speed fixture moved from Dispatch to World test support, with the old body deleted. Reused `two_chasers` for resumed drains and PG `Harness`/`Dependencies`/`prepare_current_session`, activated content and `with_spawns_for_test` for the real dispatch scenario.

## Changed files and final sizes

Paths below are relative to the repository; sizes are lines. This task record was updated last.

| File | Lines |
|---|---:|
| `crates/gameplay/combat-rules/src/attack/chase.rs` | 121 |
| `crates/gameplay/combat/src/combat/attack/chase.rs` | 581 |
| `crates/gameplay/dispatch/src/gameplay_transport/attack.rs` | 1925 |
| `crates/gameplay/dispatch/src/gameplay_transport/attack/attack_chase_search.rs` | 115 |
| `crates/gameplay/dispatch/src/gameplay_transport/attack_chase_budget_tests.rs` | 90 |
| `crates/gameplay/dispatch/src/gameplay_transport/channel_owner_auto_attack_tests.rs` | 1727 |
| `crates/gameplay/dispatch/src/gameplay_transport/creature_move_postgres_tests.rs` | 231 |
| `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs` | 1368 |
| `crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs` | 1140 |
| `crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle/tests/creature_step_tests.rs` | 209 |
| `crates/gameplay/dispatch/src/lifted_tests/movement_speed_runtime_tests.rs` | 224 |
| `crates/gameplay/entities/src/carrier.rs` | 12099 |
| `crates/gameplay/entities/src/carrier/creature_relocation.rs` | 51 |
| `crates/gameplay/monster-ai/src/ai/movement_goal.rs` | 188 |
| `crates/gameplay/monster-ai/src/ai/movement_goal_tests.rs` | 106 |
| `crates/gameplay/monster-ai/src/ai/tests.rs` | 155 |
| `crates/gameplay/monster-ai/src/ai/tests/path_request_tests.rs` | 11 |
| `crates/gameplay/monster-ai/src/ai_think.rs` | 1120 |
| `crates/gameplay/monster-ai/src/ai_think/channel_owner.rs` | 328 |
| `crates/gameplay/monster-ai/src/ai_think/channel_owner/channel_owner_creature_ai_tests.rs` | 876 |
| `crates/gameplay/monster-ai/src/ai_think/channel_owner/channel_owner_creature_ai_tests/creature_cadence_tests.rs` | 275 |
| `crates/gameplay/monster-ai/src/ai_think/channel_owner/channel_owner_creature_ai_tests/creature_movement_measure.rs` | 75 |
| `crates/gameplay/monster-ai/src/ai_think/channel_owner/channel_owner_creature_ai_tests/creature_movement_tests.rs` | 155 |
| `crates/gameplay/monster-ai/src/ai_think/channel_owner/channel_owner_creature_ai_tests/creature_path_tests.rs` | 167 |
| `crates/gameplay/monster-ai/src/ai_think/channel_owner/channel_owner_creature_ai_tests/creature_return_tests.rs` | 81 |
| `crates/gameplay/monster-ai/src/ai_think/channel_owner/channel_owner_creature_ai_tests/movement_timer_fixture.rs` | 47 |
| `crates/gameplay/monster-ai/src/ai_think/creature_steps.rs` | 124 |
| `crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_goal_adoption.rs` | 101 |
| `crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_paths.rs` | 146 |
| `crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_step_choice.rs` | 68 |
| `crates/gameplay/monster-ai/src/ai_think/creature_steps/creature_step_timer.rs` | 236 |
| `crates/gameplay/monster-ai/src/ai_think/think_tests.rs` | 783 |
| `crates/gameplay/monster-ai/src/ai_think/think_tests/movement_fixtures.rs` | 4 |
| `crates/gameplay/world/src/map/spell_tiles.rs` | 820 |
| `crates/gameplay/world/src/map/spell_tiles/tests/movement_arena.rs` | 45 |
| `crates/gameplay/world/src/movement.rs` | 1365 |
| `crates/gameplay/world/src/movement/path_budget.rs` | 76 |
| `crates/gameplay/world/src/movement/path_budget/path_budget_tests.rs` | 46 |
| `crates/gameplay/world/src/movement/path_search.rs` | 417 |
| `crates/gameplay/world/src/movement/path_search/path_labels.rs` | 122 |
| `crates/gameplay/world/src/movement/path_search/path_tiles.rs` | 44 |
| `crates/gameplay/world/src/movement/path_search/tests/path_labels_tests.rs` | 40 |
| `crates/gameplay/world/src/movement/path_search/path_occupancy.rs` | 15 |
| `crates/gameplay/world/src/movement/path_search/tests/creature_path_tests.rs` | 79 |
| `crates/gameplay/world/src/movement/relocate.rs` | 111 |
| `crates/gameplay/world/src/movement/speed.rs` | 469 |
| `crates/gameplay/world/src/movement/speed/creature_speed_fixture.rs` | 26 |
| `crates/gameplay/world/src/movement/tests/relocate_tests.rs` | 47 |
| `docs/agents/tasks/archive/CREATURE-MOVE-1.md` | 343 |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | 12537 |

No continuation source file grows by more than 200 lines; the existing search parent shrinks by 65 lines. Across the complete task, files growing by more than 200 lines (all new child files):

- `creature_step_timer.rs` (236): L4 Monster AI owns current-fence deadline drain/rearm and submits steps through World.
- `creature_cadence_tests.rs` (275): L4 tests cover actual timer/condition/near/zero-speed/clock and replay behavior using the existing Channel harness.
- `creature_move_postgres_tests.rs` (231): L5 owns the real composed dispatch/PG acceptance; it reuses the existing durability harness and lower-layer owners.
- `monster_ai_cycle/tests/creature_step_tests.rs` (209): moved existing accepted-room regression plus timer/relocation assertions belongs beside its Dispatch composition owner; the original registration/body was removed.


## Integrator guard repair — 2026-10-22

Continuation base: committed WIP `051a91f079c4f191b363208f35bf33f552070b7d`, branch `w2/creature-move-1`. Repair returned to AUTHORING in the exclusively allocated worktree; no commit, push, freeze, publication or external write. The historical validation above belongs to the earlier continuation; this section records the latest worktree results.

Planned layout: reuse Dispatch's existing PostgreSQL seam owner in `monk_save_postgres_tests.rs`, configure it from `creature_move_postgres_tests.rs`, and keep any measured search optimization in World's existing `movement/path_search/path_tiles.rs`. No new module, helper, wrapper, re-export or test registration was needed.

`Dependencies` now owns an optional combat room, initialized to `None` in its existing constructor. The creature test moves its existing bundle view into that field before calling the existing `Dependencies::owner`; the approved construction still always supplies `None` for the qualified room. Other callers retain their previous defaults. The extra construction in the creature test was removed; `bundle_gates_tests.rs` and its approved construction list are unchanged. Reused `Harness`, `prepare_current_session`, `Dependencies::owner`, the existing bundle view and real dispatch scenario without copying any fixture body.

The requested fresh release measurement exceeded 15 ms, so the same integrated window was profiled with `perf record -e cpu-clock -F 997 --call-graph dwarf`. Resolved self samples attributed 10.87% to `TileIndex::index`, 13.68% to `SearchLabels::visit`, and 23.97% to the existing heap's push/pop. The only search edit is `#[inline]` on the existing tile-index method, making this hot lookup eligible for inlining across compilation units. The cardinal A*, tile cache, route ties, admission rules, work charges and every limit remain unchanged.

| Latest continuation measurement | Build | p99 |
|---|---|---:|
| Before the inlining hint | release | 40.402791 ms |
| Profiling run, including sampling overhead | release | 15.560902 ms |
| Final repair | release | **13.583111 ms — PASS ≤15 ms** |
| Final repair, informational only | debug | 341.758378 ms |

Both exact RL-17 commands in the preceding qualification section were rerun through `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`. Each executed the existing 200-window test, asserting 1,024 thinks and 65,536 charged expansions per window. Debug remains nonqualifying; the release budget stays 15 ms.

### Final worktree validation

Every Cargo command used `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`, with locked/offline dependency resolution and the wrapper's PostgreSQL configuration. The following are exact arguments after that prefix:

| Arguments | Result |
|---|---|
| `test --locked -p oteryn-gameplay-dispatch --lib --quiet -- --test-threads=4` | **904 passed**, 14 existing ignores; seam construction guard passes |
| `test --locked -p oteryn-game-server --lib --quiet -- --test-threads=4` | **306 passed**, 3 existing ignores |
| `test --locked -p oteryn-gameplay-dispatch creature_movement_dispatch_steps_between_thinks_on_postgres --quiet -- --nocapture` | **1 passed**, 0 ignored; the required-configuration assertion and actual database/dispatch body ran |
| `test --locked -p oteryn-game-server --test durability_postgres --quiet -- --nocapture --test-threads=4` | **525 passed, 2 failed**, 0 ignored; complete run, 1467.36 s |

The full durability failures are outside this repair's changed paths:

- `postgres::durability_contract_tests::migration_lock_interruption_releases_before_any_ddl_and_allows_fresh_retry`: `Elapsed(())` from its existing 5-second migration retry timeout in `apps/game-server/tests/support/postgres.rs`.
- `owning_fresh_loss_is_atomic_and_raw_prepare_does_not_supply_authority`: the existing committed-outcome assertion at `apps/game-server/tests/durability_postgres.rs:559`.

Each failing case was then run alone with `test --locked -p oteryn-game-server --test durability_postgres <filter> --quiet -- --nocapture --test-threads=1`. The exact filters were `migration_lock_interruption_releases_before_any_ddl_and_allows_fresh_retry` (**1 passed**, 8.79 s) and `owning_fresh_loss_is_atomic_and_raw_prepare_does_not_supply_authority` (**1 passed**, 242.61 s). These isolated passes do not make the full-suite result green. No test, timeout, assertion or production rule was changed to answer those failures.

PostgreSQL execution is confirmed by the creature scenario's mandatory configuration assertion and successful database body, and by all three full-suite `ADMIT0_PG17_6_PASS` markers: `current_runtime_replacement`, `fresh_atomic_refusal`, `v2_prepare_commit_refusal`. No missing-configuration skip marker occurred. The initial default-concurrency Dispatch run had seven availability/progression failures; both subsequent complete four-thread runs passed all 904 tests. Initial default-concurrency durability also finished 525/527, with migration timeout and legacy-claim availability failures; the final run's legacy-claim case passed.

Every exact package/filter invocation in the earlier local-validation table was also rerun on the final source: World `movement::` **64**, `creature_path_tests` **4**, `map::spell_tiles` **5** (1 existing ignore); Monster AI `ai` **69** (2 existing measurement ignores); Combat/Combat Rules `chase` **10 + 5**; Entities with `test-support`, `movement` **7** and `spawn` **40**; Dispatch `attack` **55**, `monster_ai_cycle` **12**, `creature_step_tests` **1**, `movement_speed_runtime_tests` **2**. All passed. RL-17 was separately executed explicitly in both builds as recorded above.

`check --locked` and `clippy --locked --all-targets --quiet -- -D warnings`, selecting each of the six packages in the earlier validation plus `-p oteryn-game-server`, both **PASS**. `fmt --all -- --check`: **PASS**. No manifest or lockfile changed, so no workspace check/clippy was run. Governance validation, 124 governance unit tests and repository-policy validation passed; `git diff --check` passed.

The complete repair diff was reread against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Workspace-wide searches over `apps/`, `crates/`, `tools/` confirmed the one new private fixture field's constructor and use sites, and the single existing test registration. No public function, enum variant or trait method was added. No continuation file grew by more than 200 lines: the fixture grew by two lines, tile-index module by one, and the creature test retained its 231-line size. Existing search and RL-17 harnesses were reused. Temporary logs/profiling data were removed before this final task-card edit; file sizes above include this repair.

**Unfinished:** a clean full `durability_postgres` run remains required despite both failing cases passing in isolation. Resolving that full-suite validation failure must preserve its current timeouts and assertions; no changes to other lanes' paths were attempted. Independent review, repository CI, freeze/publication and MQ remain CP-owned. This task is not marked ready.
