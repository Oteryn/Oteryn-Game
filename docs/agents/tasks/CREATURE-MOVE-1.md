# CREATURE-MOVE-1

```yaml
task_id: CREATURE-MOVE-1
title: "CREATURE-MOVE-1: residue: speed step timer, A*, walk back, leash, dance, near-target, relocate (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/creature-move-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md App. 7 LOOP-MONSTER-1 graph, CREATURE-MOVE-1 residue; wave 2"
class: "hard (movement); movement and performance review"
migration: no
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ai/path_proposal.rs
  - apps/game-server/src/ai/resolution.rs
  - apps/game-server/src/movement/path_search.rs
  - apps/game-server/src/movement/speed.rs
  - apps/game-server/tests/creature_move_*.rs (new)
slot_only_paths:
  - RESOURCE_LIMITS_REGISTRY.json (path budget rows, if new)
  - mod.rs (only if the step timer lane changes composition)
public_contracts: []
depends_on:
  - "CREATURE-AI-1: MERGED #1813"
  - "SPEED-1: MERGED #1715"
  - "CREATURE-MOVE-1a/1b: MERGED #1977, #1987"
  - "CHASE-1 path search: MERGED #2113, #2125"
  - "CREATURE-MOVE-1 packet: ACCEPTED on merge (CREATURE-MOVE-1-PACKET-1)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Close the CREATURE-MOVE-1a deferrals: speed-based creature step timer, A* paths (reusing CHASE-1 path search), walk back, leash, dance, near-target x2, `relocate_creature` (packet §1.1-§1.5).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CREATURE-AI-1 | MERGED #1813 |
| SPEED-1 | MERGED #1715 |
| CREATURE-MOVE-1a/1b | MERGED #1977, #1987 |
| CHASE-1 path search | MERGED #2113, #2125 |
| CREATURE-MOVE-1 packet | ACCEPTED on merge (CREATURE-MOVE-1-PACKET-1) |

All dependencies merged; the task may start.

## Owned paths (builder edits freely)

- `apps/game-server/src/ai/path_proposal.rs`
- `apps/game-server/src/ai/resolution.rs`
- `apps/game-server/src/movement/path_search.rs`
- `apps/game-server/src/movement/speed.rs`
- apps/game-server/tests/creature_move_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- RESOURCE_LIMITS_REGISTRY.json (path budget rows, if new)
- mod.rs (only if the step timer lane changes composition)
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `movement.rs` `step_cardinal`
- `foundation/owner_timer.rs` DEADLINE_STATE
- `gameplay_transport/monster_ai_cycle.rs`

## Inseparable bundles

none (no wire, no migration).

## Acceptance criteria

- [ ] Creature steps at its own speed; A* path bounded per MOVE-RL budgets; leash/walk back deterministic; replay identical.
- [x] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue'); inspection used the allocated head documented below.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

OTERYN_GAME_CREATURE_MOVE1_STEPS_PATHS_AND_GOALS_PACKET_2026-10-04.md §1.1-§1.5; archive OTERYN-20261011-creature-move-1.md l.44. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` App. 7 LOOP-MONSTER-1 graph, CREATURE-MOVE-1 residue.

## Builder instructions

- Work on branch `skeleton/creature-move-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: local callable path and near-target subset prepared; focused validation passed
status: waiting
branch: skeleton/creature-move-1
blocker: runtime integration crosses unowned paths and awaits the CP slot
next_action: CP reviews the local diff and allocates the cross-domain integration slot
```

## Local preparation (2026-10-15, unpublished)

Allocated draft: #2184, `skeleton/creature-move-1`, starting head
`b2bf32341faf70c73b3bd264529e66b11bb4eab7`. Authoring only: no commit, publication,
content generation, registry edit or runtime wiring is authorized in this preparation.

Residue confirmed before implementation: #2113 already supplies the shared bounded cardinal
A* search and player chase; #2125 supplies canonical oldest-window/actor-id chase search
ordering and the existing per-window drain. `movement/speed.rs` already
reads native base speed and the real active condition delta, refuses base speed zero and offers
the canonical moved-only cadence. These mechanisms must be reused.

The prepared callable subset extends that search to the exact spawn cell (50-tile walk-back
distance, existing 1,024-expansion/128-step limits) and supplies the packet §1.3 near-target
counter and duration multiplier. Searches retain the caller's Movement admission callback;
there is no second walkability table. The near counter is not yet held by the runtime AI state.
No timer, queue, path-adoption store or budget mechanism is added.

`cross_domain_findings` for the integration slot:

- `ai_think.rs::CreatureAiState` and `ai_think/channel_owner.rs::run_taken_thinks` still choose
  and commit a step per think. They need accepted goals, per-creature near-counter state,
  generation/goal/content-bound `PathProposal` adoption, refusal invalidation and target
  re-search. These paths are outside this allocation; `ai/` bootstrap resolution is not the
  runtime goal owner.
- The existing owner timer lane (`foundation/owner_timer.rs`) and creature AI table need one
  `DEADLINE_STATE` step family beside think and idle cancellation.
  `combat/attack/chase.rs::ChaseSearchBudget` and
  `gameplay_transport/attack.rs::drain_chase_searches` currently drain up to 64 player searches;
  integration must compose the accepted shared RL-09 work budget and 16-search waiting-player
  reserve, retaining #2125 ordering. This preparation does not redefine any budget or build a
  parallel queue.
- `movement.rs` lacks the accepted `relocate_creature` entry and a shared read-only creature
  destination admission entry covering the map, overlay and forbidden tile kinds. Leash and
  walk-back integration also need the authoritative spawn cell, rather than treating a generic
  admission anchor as spawn authority.
- `gameplay_transport/monster_ai_cycle.rs::creature_cell_admissible` currently supplies the
  playable cell restrictions. Wiring the search and execution to the same Movement authority
  requires the slot; this preparation does not duplicate that predicate or touch the BOOT-RAT
  worker's files. `monster_combat_lane.rs` also consumes the existing cadence and must retain
  its summon/ordinary-creature ownership boundary.
- Dance, keep-distance fallback, flee/wander pacing, relocation and the integrated RL-17
  measurement remain pending with that runtime composition. No whole-task completion claim.

Local validation:

- Focused RED: expected missing-helper compile failures before implementation (25 errors).
- `cargo test --locked -p oteryn-game-server --lib movement:: --quiet`: 98 passed,
  0 failed, 1 existing ignored benchmark, including eight new regressions. The existing
  native-slot condition test also pins source speed zero despite a live SPEED condition.
- Final `movement::path_search::` rerun after strengthening the field stub test: 15 passed,
  0 failed. The test now compares occupancy alone against occupancy plus 180 with a detour
  expensive enough to distinguish the two cases. No production code changed after the
  98-pass Movement run.
- `cargo fmt --all --check`: passed.
- `cargo clippy --locked -p oteryn-game-server --lib --quiet -- -D warnings`: passed;
  no all-target/full-package claim. Cargo compile/test/lint used the shared lock/target.
- Governance validator: passed (22 documents, 9 lanes); governance unit tests: 124 passed.
- Whole-diff self-review and `git diff --check`: passed. Only the two allocated Movement
  files and this task record changed. Head is unchanged; no commit or external write.

Logs are retained locally under `/tmp/creature-move-1-*.log`.
