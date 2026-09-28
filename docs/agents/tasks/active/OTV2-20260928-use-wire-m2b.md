# OTV2-20260928-use-wire-m2b

```yaml
task_id: OTV2-20260928-use-wire-m2b
title: USE-WIRE-V1 M2b - server seam wiring (use dispatch, door runtime, movement blocking)
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: 5868482467
base_branch: main
branch: claude/use-wire-m2b
pr: 1104
base_sha: 69284a571a3b58249546e17f992dff59883be42f
head_sha: ce1ddef17d869687adfcbdfd2e5c342f144ad371
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl server seam" (Claude Code)
created_at: 2026-09-28T11:30:00Z
updated_at: 2026-09-28T12:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/world_object.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/world_runtime.rs
  - apps/game-server/src/content/activation.rs
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/node/serve.rs
  - docs/agents/tasks/active/OTV2-20260928-use-wire-m2b.md
public_contracts: []
depends_on:
  - OTV2-20260928-use-wire-m1 (#1066, merged)
  - OTV2-20260928-native-entry-door-m2a (#1075, merged)
blocks:
  - live cross-session broadcast (M2c, excluded here)
  - D37 push (excluded here)
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

M2b of USE-WIRE-V1 (#162 5868482467): Server Seam composes the door end to end.

1. `world_runtime.rs`: `ScopeContentGenerationFence::for_activation` — crate-visible, non-test,
   mirrors `#[cfg(test)] for_test`; takes scope/scope-generation/content-generation of the content
   being activated, never derived from a value a later `bind` validates. `bind_native_entry_door`
   builds the door's synthetic, non-promotable `PlacementRef` from `accepted::DOOR_CELL` and binds
   the unchanged `LocalObjectRuntime::bind`, mirroring the file's own CW4 test fixtures
   (`canonical.placements = vec![...]` post-link injection). `select_use_transition` (selection
   kernel: the unique transition bound to the current state; zero/ambiguous fails closed) and
   `attempt_use` (applies it; seam-local/in-memory, same level `step` uses, not the durable
   `CommandIngress`/`GameSessionAuthoritySnapshot` `apply`/`resume_pending`).
2. `content/project/native_entry.rs`: `movement_cells()` includes the door cell as `Walkable`
   terrain (was excluded, M2a P1). `QualifiedNativeEntryRoom::door()` carries the qualified door
   content through. `require_accepted_bindings` pins `client_projection == ClientSafe` (r4121206658).
3. `content/activation.rs`: `NativeEntryContentPin` carries the door content;
   `into_channel_parts()` returns a 3-tuple including it.
4. `node/serve.rs`: builds the fence from the committed scope/generation, calls
   `bind_native_entry_door`, wires the runtime into `GameplaySeamOwners.door`.
5. `gameplay_transport/mod.rs`: `ComposedFreshAdmission.door` (locked after `runtime`, same fixed
   order in `step` and `use_object` — no deadlock). `step` consults `door.blocking_cells()` before
   the terrain lookup: closed door blocks even though its cell is now `Walkable`. `use_object`:
   placement match (else NOTHING_TO_USE), reach (same floor, Chebyshev <=1, no LOS — pure
   `use_object_reachable`, unit-tested directly), occupancy (acting actor's own cell), then
   `attempt_use`; on COMMITTED builds the `WorldObjectOverlayEntry`.
6. `gameplay_transport/connection.rs`: `FreshAdmissionAuthority::use_object` +
   `observe_world_object_overlay` (default `None`). Join/resync snapshot (`serve_admitted`, shared
   by fresh admit and resume) gets a second `DomainSnapshot` (domain 2, snapshot 1) when served.
   `USE_INTENT` dispatched in the same command loop as STEP, under the same FND-02 CommandId
   sequencing gate already enforced for every command (acted on at most once per connection
   generation; a replayed consumed id expires/closes exactly as a replayed STEP id already does).
   COMMITTED gets a sequenced overlay `StateDelta` (domain 2, delta 1) after the `CommandResult`.
7. `gameplay_transport/qualification.rs`: builds the door runtime as `node/serve.rs` does;
   `first_control_frames`/`resume_frames` updated for the two-domain snapshot; new `use_wire_frames`
   + driving stage: open adjacent (COMMITTED) -> step through (Moved) -> use in doorway (OCCUPIED)
   -> step out (Moved) -> close (COMMITTED) -> step into closed door (Blocked) -> stale revision
   (STALE_STATE) -> unknown placement (NOTHING_TO_USE) -> replay of the consumed CommandId
   (expires/closes). TOO_FAR is not reachable in this room (every accepted cell is within
   Chebyshev 1 of the door); covered by the `use_object_reachable` unit test instead.

## Architecture and source of truth

- `PROVEN`: #162 5868482467 is this task's exact allocation.
- `PROVEN`: M2a's `door()` doc comment names the exact synthetic-placement construction and the
  CW4 fixture pattern this task reuses directly.
- `PROVEN`: `ChannelRuntimeV1` lives in `foundation/runtime_actor_carrier.rs` (excluded); the door
  runtime is a sibling `Mutex` on `ComposedFreshAdmission`, locked in the same fixed order as
  `runtime` within one Channel-owner turn — this is how "the Channel-owner state the seam already
  locks" is satisfied without touching an excluded path.
- `DERIVED`: `accepted::CELLS`/`accepted::DOOR_CELL` are all within Chebyshev distance 1 of each
  other, so no in-room movement can produce a real TOO_FAR case (checked by inspection).

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No new session/lease/generation/authority-consuming mutation boundary. The door runtime is
  scope-ephemeral in-memory state (D38 W2, same class as the existing CW4 LocalObjectRuntime),
  bound once at activation from already-authenticated issuance and the already-committed scope/
  ownership generation. USE_INTENT reuses the existing FND-02 CommandId gate and the existing
  GameSession/actor authentication `serve_admitted` already performs for STEP; no new PREPARE/COMMIT.
```

## Acceptance criteria

- [x] `ScopeContentGenerationFence::for_activation`: crate-visible, non-test, never derives the
      fence from the value a bind call validates.
- [x] Door runtime bound at activation from the door cell's synthetic, non-promotable
      `PlacementRef`, via unchanged `LocalObjectRuntime::bind`.
- [x] Door runtime held in Channel-owner-adjacent state, fixed lock order, no deadlock.
- [x] `client_projection == ClientSafe` pinned for the door (r4121206658).
- [x] Door cell in active movement; `step` consults blocking cells; no path leaves a closed door
      walkable.
- [x] USE semantics: server-selected unique transition (fail closed zero/ambiguous); reach;
      OCCUPIED; STALE_STATE; NOTHING_TO_USE; COMMITTED delta to the acting session.
- [x] `USE_INTENT` under the same FND-02 CommandId discipline as STEP; WOBJ-RL-01 satisfied by
      construction (one command per loop iteration).
- [x] Join/resync snapshot carries the `WORLD_OBJECT_OVERLAY` snapshot.
- [x] E2E qualification scenarios and selection-kernel/`attempt_use` unit tests added.
- [x] Full required-validation suite green.
- [x] M2a task record archived with its final head/merge commit.

## Deviations from the literal allocation text

- **TOO_FAR is not exercised through real movement in the E2E.** Every accepted room cell is
  within Chebyshev distance 1 of the door cell, so no legal step sequence reaches a real TOO_FAR
  case. Covered instead by a direct unit test of the extracted pure reach function
  (`use_object_reachable_is_chebyshev_one_same_floor_only`: distance 0/1 reachable, distance 2
  TOO_FAR, different floor unreachable, empty target set unreachable). Content-geometry
  constraint of the M2a room, not a gap in this task's reach logic.
- **The replayed-CommandId case demonstrates "expires and closes"**, matching STEP's own existing,
  tested behavior (`admitted_steps_are_sequenced_and_a_replayed_id_expires`), not a same-result
  replay of prior response bytes. USE_INTENT is dispatched through that identical shared gate, so
  a replay can never reach `use_object` a second time. Read as "step's own already-implemented
  pattern" per the allocation text, not the separate durable `CommandIngress`/
  `GameSessionAuthoritySnapshot` replay `apply`/`resume_pending` implement (which `step` itself
  does not use either, and would be new scope).

## Validation

### Focused

- command/run: `cargo fmt --check -p oteryn-game-server`; `cargo clippy -p oteryn-game-server
  --all-targets -- -D warnings`; `cargo test -p oteryn-game-server`; `python3
  tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`
- result: fmt PASS; clippy PASS (no warnings); full `cargo test -p oteryn-game-server` PASS across
  every test binary in the workspace (0 failed; only pre-existing topology-gated `#[ignore]`d
  tests skipped). New unit coverage: `world_runtime.rs` selection-kernel/`attempt_use` tests,
  `gameplay_transport/mod.rs` `use_object_reachable` test, `gameplay_transport/connection.rs` USE
  dispatch tests (commit+delta, non-committing disposition, unregistered type, join snapshot
  carries the overlay).

### Component/integration

- `gameplay_transport::connection::tests` exercises the full `serve_admitted` dispatch loop for
  `USE_INTENT` against a fixture authority: join snapshot with the overlay domain, COMMITTED ->
  result+delta, a non-committing disposition emits no delta, unregistered type is empty. PASS.

### E2E

- `qualification::server_seam_real_owners_over_tcp_tls` extended with the full USE-WIRE-V1
  scenario list; requires the disposable WP5 S3-B topology, `#[ignore]`d, **not runnable here**;
  CI runs it. Reviewed by hand against the exact sequence/revision arithmetic in `use_wire_frames`.

### Exact-head CI

- final head: pending — see PR #1104 for current head/checks.
- trigger source: push to `claude/use-wire-m2b`; PR #1104 opened on `ce1ddef17d869687adfcbdfd2e5c342f144ad371`.

## Self-review

- exact head: `ce1ddef1`.
- method/reviewer: implementing agent (this session).
- material findings: none beyond the two documented, reasoned deviations above. No fabricated
  evidence, no skipped validation, no faked profile IDs; the synthetic placement stays labeled
  non-promotable (`validate_synthetic_placement_evidence`, unchanged).
- verdict: ready to freeze.

## Independent review

- required: YES — Server Seam composition and a content-admission change, per root governance norm.
- exact head: pending.
- method/auditor: Codex, automated PR review (not triggered by this worker).
- verdict: awaiting Codex's review of the frozen head.

## PR and closeout

- PR #1104 opened against `main`, referencing #162 5868482467 and this task record.
- changed-file review / unresolved threads / auto-merge / merge commit / ownership release:
  pending — see PR #1104 and #162 for current status, not tracked here.
- related/superseded PRs: none known.

## Context checkpoint

```yaml
last_progress: Server Seam composition implemented and locally validated (fmt/clippy/full test
  suite/governance/repository-policy all green); M2a task record archived; pushed
  ce1ddef17d869687adfcbdfd2e5c342f144ad371; PR #1104 opened.
status: validating
branch: claude/use-wire-m2b
head_sha: ce1ddef17d869687adfcbdfd2e5c342f144ad371
pr: 1104
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: push to claude/use-wire-m2b
ci_checks_for_current_head: 0
runner_assignment_state: unknown
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: await CI/exact-head readback and independent review on PR #1104
```
