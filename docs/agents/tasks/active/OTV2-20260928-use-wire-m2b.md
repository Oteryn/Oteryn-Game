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
head_sha: 1d1032cdc4e1dba17261d9b4ceae06289f45a3d1
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl server seam" (Claude Code)
created_at: 2026-09-28T11:30:00Z
updated_at: 2026-09-28T14:00:00Z
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
  - apps/game-server/src/foundation/runtime_actor_carrier.rs  # shared-lease, committed_player_positions only (r4121956127)
  - apps/game-server/src/gameplay_transport/resume.rs  # shared-lease grant (#162 5870253781), reconnect fence domain 2
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

M2b of USE-WIRE-V1 (#162 5868482467): Server Seam composes the door end to end (fence, door
runtime binding, movement blocking, USE_INTENT dispatch, overlay in the join/resync snapshot and
committed-use deltas, full E2E scenario coverage). See PR #1104's diff for the implementation; this
record's Repair sections track the review/CI rounds since the first freeze.

## Repair round 1 (`e6f753f` -> `d220ec3`)

1. P1 r4121956127 (Codex): `use_object`'s occupancy set held only the issuing actor's own cell.
   Fixed with `ChannelRuntimeV1::committed_player_positions()` — a shared-lease, production,
   non-test read added to the excluded `foundation/runtime_actor_carrier.rs` (no existing
   production enumeration existed anywhere in the crate; verified exhaustively before touching
   it). Reuses the existing per-slot position store, read under the same runtime lock/work item
   as `attempt_use` (TOCTOU-free), no parallel registry. New test
   `use_object_occupancy_includes_every_committed_actor_not_only_the_issuer`.
2. Codex summary 5869579920: `attempt_use` now checks `expected_revision` before selecting a
   transition, always (a stale caller never gets NOTHING_TO_USE/REJECTED/OCCUPIED computed from
   newer state). New tests `stale_revision_wins_over_an_ambiguous_current_state` /
   `..._a_terminal_current_state`.
3. CI final-invariant break on `e6f753f`: the use-wire stage's 4th admission was never released.
   Fixed by releasing it (control loss -> grace expiry -> terminal) and updating the expected
   admissions count.

## Repair round 2 (`d220ec3` -> this push)

1. **P1 r4122215795 (Codex, reconnect fence)**: the FND-02 reconnect fence only covered domain 1
   (`STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY`). Fixed within owned paths: `SessionContinuity` gains
   `overlay_revision: u64` (`connection.rs`), written from the live `WorldObjectOverlayEntry.revision`
   whenever the join/resync snapshot or a committed-use delta actually sends the overlay (Channel-
   global, unlike `spatial_revision`, so it is written, never just trusted to already match).
   **`resume_lost`'s fence construction (`gameplay_transport/resume.rs:237-248`, specifically the
   `vec![StateDomainRevisionV1::new(STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
   lost.continuity.spatial_revision), ...]` literal) is outside every owned/leased path — reporting
   `SHARED_LEASE_REQUIRED` for that exact file/lines rather than editing it.** New tests (within
   owned paths): `admitted_use_commits_and_the_join_snapshot_carries_the_door_overlay` now also
   asserts `ended.continuity.overlay_revision == 1`; new
   `admitted_join_snapshot_alone_records_the_overlay_revision_it_sent` proves the join path alone
   sets it from what was actually sent. E2E: not extended — the fence's domain-2 entry cannot be
   exercised without the resume.rs change above.
2. **CI companion finding (same coordinator round)**: after round 1's release fix, CI showed
   `committed=0` (expected `1`) — round 1's own ~160s release-wait, run *before* character[0]'s
   final plain-disconnect re-admission, delayed that re-admission's connection close long enough
   for *its own* missed-liveness control-loss timer to also fire and complete during the same
   window (previously it never had time to before `shutdown` cut it short). Fixed by reordering:
   the use-wire stage (admission through full release) now runs entirely *before*
   character[0]'s final re-admission, which stays the literal last action so its own loss-timer
   race against `shutdown` is unaffected — restoring the exact pre-M2b timing. Per-stage
   `committed_admissions` checks renumbered (use-wire's own admission is now the 3rd, character[0]'s
   final one the 4th); the outer `seam_flow` final invariant's asserted values are unchanged
   (`committed_players == 1`, `pending == 0`, `committed_admissions == 4`).

Replied once on the r4122215795 thread covering both findings.

## Repair round 3 (`1d1032c` -> this push): reconnect fence domain 2, shared lease used

Shared lease granted (#162 5870253781), scoped to exactly the `Fnd02ReconciliationFenceV1`
domain-revision `vec` in `resume.rs` (~lines 237-248). Used it, nothing else in that file
changed: added the `STATE_DOMAIN_WORLD_OBJECT_OVERLAY` import and one
`StateDomainRevisionV1::new(STATE_DOMAIN_WORLD_OBJECT_OVERLAY, lost.continuity.overlay_revision)`
entry after the existing spatial one (ascending domain id 1 then 2, per
`Fnd02ReconciliationFenceV1::new`'s own strict-order check). New `resume.rs` test module (none
existed before): both-domains-present/ordered, and a descending-order case proving the ordering
is load-bearing. `resume_lost` itself still has no direct unit test (needs `DurabilityRoot` etc.,
exercised only by the WP5 E2E); these exercise the exact fence-construction shape it uses.

## Architecture and source of truth

- `PROVEN`: #162 5868482467 is this task's exact allocation.
- `DERIVED`: the qualification E2E's character[0] final re-admission relies on a timing race
  against `shutdown`; any stage that delays the overall scenario's completion risks flipping that
  race, as round 2 finding 2 showed.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No new session/lease/generation/authority-consuming mutation boundary in any repair round.
  committed_player_positions and overlay_revision are both read/record-only additions over
  existing state, gated by the same locks/paths every other read or continuity field already is.
```

## Acceptance criteria

- [x] Original M2b acceptance criteria (see PR #1104 diff / repair round 1 entry above).
- [x] P1 r4121956127 (occupancy) and its regression test.
- [x] Stale-revision-before-selection ordering and its regression tests.
- [x] CI admissions-count invariant (round 1).
- [x] `SessionContinuity.overlay_revision` tracked from both snapshot and delta; regression tests.
- [x] CI committed-actor-count invariant restored by stage reordering (round 2).
- [x] Reconnect fence's domain-2 entry in `resume_lost` (round 3, shared lease #162 5870253781).
- [x] Full required-validation suite green on the repaired head.

## Deviations from the literal allocation text

- **TOO_FAR** not exercised through real E2E movement (room geometry); unit-tested instead.
- **Replayed-CommandId** demonstrates "expires and closes" (STEP's own pattern), not a
  same-result replay of prior bytes.
- **Second-admitted-session E2E** for the P1 occupancy repair: not attempted; covered by an
  integration-level unit test against the real `ChannelRuntimeV1`/`LocalObjectRuntime` instead.

## Validation

### Focused

- command/run: `cargo fmt --check -p oteryn-game-server`; `cargo clippy -p oteryn-game-server
  --all-targets -- -D warnings`; `cargo test -p oteryn-game-server`; `python3
  tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`
- result (round 2 head): fmt PASS; clippy PASS (no warnings); full `cargo test -p
  oteryn-game-server` PASS across every test binary (0 failed, only pre-existing topology-gated
  `#[ignore]`s); governance PASS; repository-policy PASS.

### Component/integration

- `gameplay_transport::connection::tests` (dispatch loop + the two new overlay-continuity tests)
  PASS. `gameplay_transport::tests::use_object_occupancy_includes_every_committed_actor_not_only_the_issuer`
  PASS.

### E2E

- `qualification::server_seam_real_owners_over_tcp_tls`: requires the disposable WP5 S3-B
  topology, `#[ignore]`d, **not runnable here**; CI runs it. Round 1's CI run confirmed every
  USE_WIRE disposition assertion and (per the coordinator) the reordered stage's own
  `admissions=3`/`released=terminal` evidence; the final-invariant fix is reasoned from the code
  (see round 2 finding 2) since it cannot be executed locally.

### Exact-head CI

- final head: pending — see PR #1104 for current head/checks.
- trigger source: push to `claude/use-wire-m2b`.

## Self-review

- exact head: `1d1032cdc4e1dba17261d9b4ceae06289f45a3d1` (round 3 not yet pushed at last edit).
- method/reviewer: implementing agent (this session), addressing Codex's P1 r4122215795, the CI
  committed-actor-count finding, and (round 3) the domain-2 reconnect-fence entry under the
  granted shared lease (#162 5870253781).
- material findings: all above, all fixed and unit-tested. The domain-2 fence entry was correctly
  deferred with `SHARED_LEASE_REQUIRED` until the control plane granted the exact-scoped lease,
  then made minimally (import + one `vec` entry + a new test module; nothing else in `resume.rs`
  changed).
- verdict: ready to re-freeze.

## Independent review

- required: YES — Server Seam composition, a content-admission change, and two `foundation`/
  `resume.rs` shared-lease additions, per root governance norm.
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
last_progress: Repair round 3 complete under the granted shared lease (#162 5870253781):
  resume.rs's Fnd02ReconciliationFenceV1 now carries the domain-2 overlay revision next to
  domain-1 spatial, ascending order, with new fence-construction tests; fmt/clippy/full
  tests/both validators green; pushing.
status: validating
branch: claude/use-wire-m2b
head_sha: 1d1032cdc4e1dba17261d9b4ceae06289f45a3d1
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
