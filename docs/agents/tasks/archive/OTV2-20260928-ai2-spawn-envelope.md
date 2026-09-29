# OTV2-20260928-ai2-spawn-envelope

```yaml
task_id: OTV2-20260928-ai2-spawn-envelope
title: GAME-AI-01 §5 AI-2 -- creature envelope, spawn realization, respawn (D115/D116)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ai2-rat-behaviour
issue: 162
pr: 1193
allocation: "GAME-AI-01 §5 AI-2 row (control-plane correction: #162 comment 5879863781's label was in error); D115/D116 in comment 5879404970"
base_sha: 86116adfda4b4e7c1dc505c2e2af486db2370378
head_sha: b5ecb5fc8d8e8b89310ebca2913c4695abd36443
final_head_sha: b5ecb5fc8d8e8b89310ebca2913c4695abd36443
final_head_frozen_at: 2026-09-28
owner: "Oteryn: impl ai (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/mod.rs               # one `mod` line removed
  - apps/game-server/src/foundation/runtime_actor_carrier.rs   # creature envelope, spawn, respawn
  - apps/game-server/src/foundation/channel_owner_combat_death_tests.rs   # field rename, one assertion updated
  - apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs   # one test re-scoped
  - apps/game-server/src/movement.rs                     # Movement adoption test kept; rat-decide test removed
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json          # AI01-SPAWN-* rows
  - docs/agents/tasks/active/OTV2-20260928-ai2-spawn-envelope.md
public_contracts:
  - GAME-AI-01
depends_on:
  - OTV2-20260928-ai1-owner-timer-lane
blocks:
  - "AI-3 (ai/** into lib.rs; real perception/chase/wander over owner snapshots)"
  - "AI-4 (typed AI issuer via Ability; HP floor D54; real bite committer)"
cross_repository_coordination_id: null
external_repositories: []
```

## Plan (written before coding)

1. Correction received mid-task from the control plane: the `#162` allocation-comment label
   ("AI-2 rat behaviour") was in error; the written GAME-AI-01 §5 table governs the split
   (AI-2 = carrier envelope/spawn/respawn/Movement adoption; AI-3/AI-4 own the rest).
2. Remove `foundation/creature_think.rs` and its `mod` line; keep the Movement-adoption
   carrier fix and its test (genuinely §4.5/AI-2).
3. Design the carrier's creature envelope (per-actor corpse/reward tracking) and
   spawn/respawn state, reusing AI-1's `OwnerTimerLane`, binding items (a)/(b) applied.
4. Implement, test, validate, rename this record, push. No PR, no GitHub comment.

## Outcome

`ChannelActorCarrier` now admits more than one creature per Channel generation, bounded by
the D57 envelope (`AI01_SPAWN_*` constants) rather than a single-creature gate. Corpse
projections and reward occurrences move from a one-per-generation singleton to a
per-actor-keyed, FIFO-bounded collection, preserving every existing Combat D1/D2
replay/idempotency guarantee per actor while letting distinct creatures' deaths coexist.
`realize_spawn` admits D116's one spawn of 2 rats at declared cells, all-or-nothing;
`resolve_respawn_timer` applies one fired respawn timer (§4.3: admit on a free cell with a
fresh generation, D52; postpone up to 3x on an occupied cell; terminate `SKIPPED` and start a
successor chain on the 4th), also all-or-nothing. The Movement-adoption carrier fix is
unchanged from the prior head. No `ai/**`, perception/chase/wander/bite/Ability: AI-3/AI-4.

## Architecture and source of truth

- PROVEN: the GAME-AI-01 decision (`docs/architecture/reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md`)
  §4.1 (envelope), §4.3 (spawn/respawn), §4.5 (Movement adoption), §4.9 (D57 rows), §5 (the
  child delivery table this correction restores as authoritative for AI-2).
- PROVEN (live, not yet in `docs/`): `#162` issuecomment-5879404970 posts D115 (think 1000
  ms; perception 7 tiles; 25% wander/radius 2; respawn 60 s; occupied cell retried 3x/5s)
  and D116 (one spawn, 2 rats, in the existing starting room).
- PROVEN: `owner_timer.rs` (AI-1, unmodified): `OwnerTimerLane` is fully generic; this task
  supplies `RespawnFamily`/`RespawnOccurrence` and reuses the lane as AI-1's own tests do.
- PROVEN: only the carrier's *admission* and *death-record retention* were channel-wide
  singletons before this task; per-slot damage/death gating was already generic.
- PROVEN (discovered mid-task): a pre-existing Combat D1/D2 test requires a retained corpse
  projection to reconcile a lost-response retry *after* the actor's slot is removed, so
  per-actor keying never prunes on `remove` (only spawn-cell `live` occupancy is); FIFO
  eviction bounds the collections instead, sized to the D57 envelope (16x4=64).
- DERIVED: several `tests/*.rs` binaries path-include `foundation/mod.rs` without
  `crate::movement`/`crate::content`; spawn/respawn code stays foundation-self-contained.

## Acceptance criteria

- [x] A second, distinct creature admits successfully while the first is still live.
- [x] `realize_spawn` admits exactly `population` creatures at `population` declared cells
      (now required equal), with distinct actor-local generations, all-or-nothing on failure.
- [x] `AI01-SPAWN-POPULATION`/`-PLACEMENT-CELLS`/`-SOURCES-PER-SCOPE`: max accepted, max+1
      rejected, before any mutation; a population/cell-count mismatch rejected both ways.
- [x] `resolve_respawn_timer` admits a fresh generation on a free cell (D52) atomically with
      removal/positioning/cell update; an invalid context or any later failure mutates nothing.
- [x] An occupied cell postpones up to `AI01-SPAWN-OCCUPANCY-RETRIES` (3) times, then
      terminates `SKIPPED` and starts one successor chain at attempt 0/1.
- [x] A `RespawnOccurrence` is never scheduled twice in the lane's lifetime;
      `DeadlineState` never collapses two distinct cells' due occurrences into one fire.
- [x] A retained corpse projection / reward occurrence still reconciles a lost-response
      retry after its actor's slot is removed (pre-existing Combat D1/D2 behavior).
- [x] Combat D1/D2's existing tests stay green under the per-actor keying.
- [ ] Exact-head CI and required independent review (control plane, after freeze).

## Excluded scope

`ai/**` compilation into `lib.rs`, perception/chase/wander, the typed AI issuer via Ability,
the real bite committer and player HP floor D54 -- AI-3/AI-4. `durability/item_transfer*`,
migrations, `gameplay_transport/**` (B3-2's lease). No production caller wires
`realize_spawn`/`resolve_respawn_timer` into `ChannelRuntimeV1`'s live owner cycle yet (same
pattern as `owner_timer.rs`); complete, tested and uncalled, ready for that wiring.

## Content gap (§5: "the room revision goes to the Content/Seam owner")

No `content/**`/`native_entry_room.json` access in this task's owned paths, so D116's spawn
was realized and tested only against synthetic pre-production fixtures, never real
starting-room content. Placing the 2 cells in the real room, keeping the spawn/wander area
outside the accepted start/east Movement proof path (§4.5), needs its own Content/Seam-owner
allocation. Reported per instruction, not done here.

## Binding items carried over from AI-1, applied to respawn occurrences

- **(a) bounded replay evidence.** `SpawnCellState.attempt`/`.successor` are the only place a
  `RespawnOccurrence` tuple is derived from (`resolve_respawn_timer` alone advances them,
  monotonically, per cell); a `(source, cell_index, dead_actor, successor, attempt)` tuple
  can therefore never repeat in a lane's lifetime -- O(1) state per cell, never an unbounded
  log. Exercised by `respawn_timer_lane_binding_items_and_deadline_state_never_collapses_distinct_cells`.
- **(b) unforgeable stamp provenance.** Unchanged: `OwnerTimerLane::schedule` itself already
  enforces this (AI-1); this task's tests only ever forward a fence-issued
  `RuntimeWorkStamp`, exactly as AI-1's own tests do.

## Implementation / findings

- `runtime_actor_carrier.rs`: `has_creature: bool` removed; `corpse_projection`/
  `death_reward_occurrence` singletons become `corpse_projections`/
  `death_reward_occurrences` (`Vec`, per-actor, FIFO-bounded). New: `SpawnSourceId`,
  `SpawnDefinition`, `SpawnCellState`, `SpawnRealization`, `RespawnFamily`/
  `RespawnOccurrence`/`RespawnResolution`, `realize_spawn`, `cell_occupied`,
  `locate_spawn_cell`, `spawn_cell_state`, `spawn_definition`, `resolve_respawn_timer`, 4
  new `CarrierError` variants.
- `channel_owner_combat_death_tests.rs`/`channel_owner_ability_commit_tests.rs`: field-name
  and capacity-assumption updates for the lifted one-creature gate (see prior generation).
- `foundation/mod.rs`/`movement.rs`: the deleted `creature_think` module's mod line and
  dependent test removed; the Movement-adoption fix/test are unchanged.
- `RESOURCE_LIMITS_REGISTRY.json`: 4 new rows, `AI01-SPAWN-SOURCES-PER-SCOPE` (16),
  `-POPULATION` (4), `-PLACEMENT-CELLS` (4), `-OCCUPANCY-RETRIES` (3), max/max+1 tested.

## Validation

- `cargo fmt -p oteryn-game-server -- --check`: clean.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: clean, incl. the
  standalone Foundation test module and all 9 standalone `tests/*.rs` composition/Postgres
  crates path-including `foundation/mod.rs` (checked individually).
- `cargo test -p oteryn-game-server --lib`: whole-crate regression green (see report).
- `... -- foundation::runtime_actor_carrier::tests::`: 76 passed (spawn realization/bounds
  incl. the P1/P2 repairs' new tests, respawn admit/postpone/skip/successor, timer-lane).
- `python3 tools/agents/validate_governance.py`: passed (22 policy docs, 9 lanes).
- `git diff --check`: clean.

## Repair generation 1 (Codex review, PR #1193, frozen head `7955edb`)

- **P1 fixed:** `realize_spawn` validated the position context and staged/rolled back the
  whole admission on any later per-creature failure, so a rejected realization (or one that
  fails partway) leaves `slots`/`free_head`/`spawns` byte-identical instead of accumulating
  untracked, unpositioned creatures on retry.
- **P1 fixed:** `resolve_respawn_timer` validated the context first and staged the dead
  actor's removal + the replacement's admission + positioning + the cell's `live` update as
  one atomic step, rolled back together on any failure, so the cell can never end up naming
  a removed actor while an unpositioned replacement stays admitted.
- **P2 fixed:** `SpawnDefinition::new` now requires `population == placement_cells.len()`
  exactly (was `<=`), so a spawn can no longer declare more placement cells than its
  population and have `resolve_respawn_timer` respawn past it.
- **P2 accepted, not fixed (per control-plane disposition):** after `MAX_RETAINED_CORPSE_
  PROJECTIONS` FIFO eviction, a retry of an evicted death fails closed
  (`CommittedLethalUnavailable`) rather than duplicating loot/XP; it only loses the reward
  for a retry older than 64 more recent deaths. Deferred as a binding item for D/E admission
  (retention tied to the replay lifetime, not this slice's carrier).

## Self-review

- exact head: local candidate before freeze
- method/reviewer: implementing agent; one independent (Codex) review round, all findings
  disposed as above
- verdict: READY_FOR_FREEZE (pending push and control-plane routing)

## Independent review

Required: YES. Exact head/method/findings/verdict: pending.

## PR and closeout

Per instruction, this worker opens no PR, no GitHub comment, no review request; it pushes
`claude/ai2-rat-behaviour` (new commits, no force-push) and stops. Control plane owns
freeze, review and integration.

## Context checkpoint

```yaml
last_progress: PR #1193 merged via Merge Queue as b6bb50d; protected-main readback of owned code files matched b5ecb5f; record archived
status: completed
pr: 1193
head_sha: b5ecb5fc8d8e8b89310ebca2913c4695abd36443
final_head_sha: b5ecb5fc8d8e8b89310ebca2913c4695abd36443
blocker: "content gap: D116 spawn placement in the real starting room needs the Content/Seam owner (§5)"
next_action: none for this task; follow-ups are routed on #162
```

## Closeout

- merge commit/result: `b6bb50d` on protected `main` (#1193); readback of owned code files byte-identical to `b5ecb5f`
- ownership release: all leases released at merge
