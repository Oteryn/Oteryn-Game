# CREATURE-MOVE-1 — creature step timer, paths, goals and relocation (consolidated task card)

```yaml
task_id: CREATURE-MOVE-1
title: "CREATURE-MOVE-1: speed step timer, A* paths, walk back, leash, dance, near-target, relocate"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 2
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "App. 7 LOOP-MONSTER-1 graph, CREATURE-MOVE-1 residue"
brief: "docs/architecture/reviews/OTERYN_GAME_CREATURE_MOVE1_STEPS_PATHS_AND_GOALS_PACKET_2026-10-04.md (§1.1–§1.7)"
class: "hard (movement); movement and performance review"
base_branch: main
branch: null
pr: null
needs_migration: false
needs_registry: false   # RESOURCE_LIMITS_REGISTRY rows only if a new path budget row is needed
merge_authority: CP_ONLY (builder never marks ready)
```

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
| Search profile, walk-back target, field term | `crates/gameplay/world/src/movement/path_search.rs` (exists, 437 lines) |
| Step duration, near ×2 | `crates/gameplay/world/src/movement/speed.rs` (exists, 436 lines) |
| `relocate_creature`, creature destination admission | `crates/gameplay/world/src/movement/relocate.rs` (new), registered from `crates/gameplay/world/src/movement.rs` (1,362 lines: wiring only) |
| Goals and near counter | `crates/gameplay/monster-ai/src/ai/movement_goal.rs` (new); state field in `crates/gameplay/monster-ai/src/ai_think.rs` (1,230 lines: call sites only) |
| Route carrier / adoption | `crates/gameplay/monster-ai/src/ai/path_proposal.rs`, `crates/gameplay/monster-ai/src/ai/resolution.rs` (exist) |
| Step timer family | `crates/gameplay/foundation/src/foundation/owner_timer.rs` (wiring); `crates/gameplay/monster-ai/src/ai_think/channel_owner.rs` |
| Shared budget / player drain | `crates/gameplay/combat/src/combat/attack/chase.rs` (exists, 549 lines), `crates/gameplay/dispatch/src/gameplay_transport/attack.rs` (2,013 lines: call sites only) |
| Cycle composition | `crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs` |
| Tests | child files beside each module, e.g. `crates/gameplay/world/src/movement/relocate_tests.rs` (new), `crates/gameplay/monster-ai/src/ai/movement_goal_tests.rs` (new), `crates/gameplay/dispatch/src/gameplay_transport/attack_chase_budget_tests.rs` (new) |

## Acceptance criteria

- [ ] A creature steps at its own speed on the step timer; near ×2 per §1.3; speed 0 stands.
- [ ] A* paths bounded per §1.4 limits; walkability from the Movement predicate only; +180 field stub pinned.
- [ ] Shared RL-09 window with the 16,384-unit player reserve; resumed drains keep planned occupancy.
- [ ] Goals chase / keep distance / dance / flee / wander / walk back / stand per §1.5; leash and `relocate_creature` per packet.
- [ ] Deterministic draws (`AI_DANCE`, `AI_WANDER`), replay identical; RL-17 measured on the integrated loop.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`movement.rs`, `ai_think.rs`, `attack.rs` take call sites only).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-world movement::
cargo test -p oteryn-gameplay-monster-ai ai
cargo test -p oteryn-gameplay-dispatch attack
python3 tools/agents/validate_governance.py
```
