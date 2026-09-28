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
head_sha: 55009bd0be44966fd20b8bde8d3fd9615f7b7875
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl server seam" (Claude Code)
created_at: 2026-09-28T11:30:00Z
updated_at: 2026-09-28T13:05:00Z
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
  - apps/game-server/src/foundation/runtime_actor_carrier.rs  # shared-lease (P1 r4121956127)
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

M2b of USE-WIRE-V1 (#162 5868482467): Server Seam composes the door end to end. `world_runtime.rs`
adds `ScopeContentGenerationFence::for_activation`, `bind_native_entry_door`, and the
selection-kernel/`attempt_use` pair. `native_entry.rs`/`activation.rs` put the door cell in active
movement and carry its qualified content through to activation, pinning `client_projection ==
ClientSafe` (r4121206658). `node/serve.rs` binds the door runtime at activation.
`gameplay_transport/mod.rs`'s `ComposedFreshAdmission.door` (locked after `runtime`, fixed order)
makes `step` respect the door's blocking cells and `use_object` implement USE-WIRE-V1 (placement
match, reach, occupancy, dispatch). `connection.rs` dispatches `USE_INTENT` under the same FND-02
CommandId gate as STEP and carries `WORLD_OBJECT_OVERLAY` in the join/resync snapshot and on a
committed use. `qualification.rs` adds full E2E USE-WIRE-V1 scenario coverage.

## Repair round (return to AUTHORING; frozen head `e6f753f` thawed)

Three findings addressed on the same push, all from PR #1104 review/CI:

1. **P1, r4121956127 (Codex)**: `use_object`'s occupancy set held only the issuing actor's own
   cell, so actor B (adjacent) could close the door on actor A (standing in the doorway) instead
   of getting OCCUPIED. Fixed: `ChannelRuntimeV1::committed_player_positions()` (new, production,
   non-test — a shared-lease addition to the excluded `foundation/runtime_actor_carrier.rs`,
   analogous to the M1 `world_spatial.rs` extension) reuses the existing per-slot position store
   `read()` already reads from, returning every committed actor's position under the pinned
   Movement context in one pass — including one retained during disconnect grace (only its
   `control_loss` mark changes, never `committed`/`position`). No parallel registry: it is a read
   over the existing `slots` store, mirroring the file's own existing test-only census methods.
   `use_object` now builds its occupied-cells set from this, under the same `runtime` lock and the
   same work item as `attempt_use`, so it is TOCTOU-free. New test
   `use_object_occupancy_includes_every_committed_actor_not_only_the_issuer`
   (`gameplay_transport/mod.rs`): two real actors in one real `ChannelRuntimeV1`, actor A in the
   door cell, actor B adjacent; B's close attempt reports OCCUPIED with no transition/revision
   change. E2E: not extended for a second admitted session — infeasible without materially
   restructuring the already-long WP5 seam scenario's account/character pairing (only two
   Platform accounts are provisioned); the integration-level unit test above exercises the real
   `ChannelRuntimeV1`/`LocalObjectRuntime` production code instead.
2. **Codex summary 5869579920 (ordering)**: `attempt_use` selected a transition before checking
   `expected_revision`, so a stale caller against an ambiguous or terminal current state got
   NOTHING_TO_USE/REJECTED instead of STALE_STATE. Fixed: `expected_revision` is now checked first,
   always. New tests `stale_revision_wins_over_an_ambiguous_current_state` and
   `stale_revision_wins_over_a_terminal_current_state` (`world_runtime.rs`).
3. **CI, "Server Seam over TCP+TLS" on `e6f753f`**: every USE_WIRE `SEAM_EVIDENCE` assertion
   passed, but the stage's fresh 4th admission (character[1], re-admitted after its own earlier
   release) was never released, breaking `seam_flow`'s final invariant (`committed_players == 1`,
   `committed_admissions == 3`). `committed_admissions` is a durable receipt count that never
   decreases, so the 4th admission permanently changes it; fixed by releasing the use-wire actor
   (control loss -> grace expiry -> TERMINAL, mirroring the existing `session`/`concurrent[0]`
   pattern exactly) and updating the final check's expected admissions from 3 to 4.

Replied once on the r4121956127 thread covering all three before this push.

## Architecture and source of truth

- `PROVEN`: #162 5868482467 is this task's exact allocation.
- `PROVEN`: no production, non-test enumeration of actor positions existed anywhere in the crate
  before this repair (checked `foundation/runtime_actor_carrier.rs` exhaustively); the new
  `committed_player_positions()` is the minimal addition that made one available without a
  parallel registry.
- `DERIVED`: `accepted::CELLS`/`accepted::DOOR_CELL` are all within Chebyshev distance 1 of each
  other, so no in-room movement can produce a real TOO_FAR case (checked by inspection).

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No new session/lease/generation/authority-consuming mutation boundary. The new foundation read
  (committed_player_positions) is read-only census over existing slot state, gated by the same
  runtime lock every other read already requires; it grants no new authority and mutates nothing.
  The door runtime remains scope-ephemeral in-memory state bound once at activation. USE_INTENT
  reuses the existing FND-02 CommandId gate and existing GameSession/actor authentication.
```

## Acceptance criteria

- [x] Fence/door-binding/selection-kernel/movement-blocking/USE-semantics criteria from the
      original allocation (see PR #1104 diff and prior revision of this record).
- [x] P1 r4121956127 (occupancy) fixed and unit-tested.
- [x] Stale-revision-before-selection ordering fixed and unit-tested (ambiguous + terminal state).
- [x] CI final-invariant break fixed (use-wire actor released; expected admissions updated).
- [x] Full required-validation suite green on the repaired head.

## Deviations from the literal allocation text

- **TOO_FAR** is not exercised through real E2E movement (every accepted room cell is within
  Chebyshev 1 of the door); covered by a direct unit test of the extracted pure reach function.
- **Replayed-CommandId** demonstrates "expires and closes" (STEP's own existing, tested pattern),
  not a same-result replay of prior response bytes (the separate durable `CommandIngress` lifecycle
  `step` itself does not use either).
- **Second-admitted-session E2E extension** for the P1 repair: not attempted (see repair item 1);
  covered by an integration-level unit test against the real `ChannelRuntimeV1`/`LocalObjectRuntime`
  instead.

## Validation

### Focused

- command/run: `cargo fmt --check -p oteryn-game-server`; `cargo clippy -p oteryn-game-server
  --all-targets -- -D warnings`; `cargo test -p oteryn-game-server`; `python3
  tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`
- result (post-repair): fmt PASS; clippy PASS (no warnings); full `cargo test -p oteryn-game-server`
  PASS across every test binary (0 failed, only pre-existing topology-gated `#[ignore]`s);
  governance PASS; repository-policy PASS.

### Component/integration

- `gameplay_transport::connection::tests` (dispatch loop, unchanged by this repair) PASS.
- `gameplay_transport::tests::use_object_occupancy_includes_every_committed_actor_not_only_the_issuer`
  (new, this repair): two-actor real-runtime OCCUPIED proof. PASS.

### E2E

- `qualification::server_seam_real_owners_over_tcp_tls`: requires the disposable WP5 S3-B
  topology, `#[ignore]`d, **not runnable here**; CI runs it. The prior push's CI run confirmed
  every USE_WIRE disposition assertion passed; this repair addresses only the final-invariant
  admissions-count break that same run reported.

### Exact-head CI

- final head: pending — see PR #1104 for current head/checks.
- trigger source: push to `claude/use-wire-m2b`.

## Self-review

- exact head: pending (this repair not yet pushed at record-write time).
- method/reviewer: implementing agent (this session), addressing Codex's P1 r4121956127, Codex's
  ordering summary 5869579920, and the CI final-invariant break, per coordinator direction.
- material findings: all three above, all fixed and unit-tested; no fabricated evidence, no
  skipped validation.
- verdict: ready to re-freeze.

## Independent review

- required: YES — Server Seam composition, a content-admission change, and now a
  `foundation/**` shared-lease read addition, per root governance norm.
- exact head: pending.
- method/auditor: Codex, automated PR review (not triggered by this worker).
- verdict: awaiting Codex's review of the repaired head.

## PR and closeout

- PR #1104 open against `main`, referencing #162 5868482467 and this task record.
- changed-file review / unresolved threads / auto-merge / merge commit / ownership release:
  pending — see PR #1104 and #162 for current status, not tracked here.
- related/superseded PRs: none known.

## Context checkpoint

```yaml
last_progress: Repair round complete (P1 occupancy via foundation shared-lease, stale-revision
  ordering, CI admissions-count invariant); fmt/clippy/full tests/both validators green; replying
  on the r4121956127 thread and pushing.
status: validating
branch: claude/use-wire-m2b
head_sha: 55009bd0be44966fd20b8bde8d3fd9615f7b7875
pr: 1104
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: push to claude/use-wire-m2b
ci_checks_for_current_head: 0
runner_assignment_state: unknown
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: await CI/exact-head readback and independent review on the repaired head
```
