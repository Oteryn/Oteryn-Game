# OTV2-20260928-use-wire-m2b

```yaml
task_id: OTV2-20260928-use-wire-m2b
title: USE-WIRE-V1 M2b - server seam wiring (use dispatch, door runtime, movement blocking)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: 5868482467
base_branch: main
branch: claude/use-wire-m2b
pr: 1104
base_sha: 69284a571a3b58249546e17f992dff59883be42f
head_sha: 0a7feafa4c4fe7d06762f1259abf25be261c59eb
final_head_sha: 0a7feafa4c4fe7d06762f1259abf25be261c59eb
final_head_frozen_at: 2026-09-28T13:44:40Z
owner: "Oteryn: impl server seam" (Claude Code)
created_at: 2026-09-28T11:30:00Z
updated_at: 2026-09-28T15:00:00Z
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

1. P1 r4121956127: occupancy set held only the issuer's cell. Fixed with
   `ChannelRuntimeV1::committed_player_positions()` (shared-lease, `foundation/runtime_actor_carrier.rs`;
   no prior production enumeration existed). Reuses the existing per-slot position store, same
   runtime lock/work item as `attempt_use` (TOCTOU-free), no parallel registry. New test
   `use_object_occupancy_includes_every_committed_actor_not_only_the_issuer`.
2. Codex 5869579920: `attempt_use` now checks `expected_revision` before selecting a transition,
   always. New tests `stale_revision_wins_over_an_ambiguous_current_state` /
   `..._a_terminal_current_state`.
3. CI: use-wire stage's 4th admission was never released. Fixed by releasing it and updating the
   expected admissions count.

## Repair round 2 (`d220ec3` -> `1d1032c`)

1. P1 r4122215795: reconnect fence only covered domain 1. `SessionContinuity` gains
   `overlay_revision: u64` (`connection.rs`), written from the live overlay entry whenever the
   join/resync snapshot or a committed-use delta sends it. `resume_lost`'s fence construction
   (`resume.rs:237-248`) was outside every owned/leased path — reported `SHARED_LEASE_REQUIRED`
   rather than editing it. New tests: `admitted_use_commits_and_the_join_snapshot_carries_the_door_overlay`
   now also asserts `overlay_revision == 1`; new `admitted_join_snapshot_alone_records_the_overlay_revision_it_sent`.
2. CI companion: after round 1's release fix, CI showed `committed=0` (expected `1`) — the
   release-wait, run before character[0]'s final re-admission, let that re-admission's own
   loss-timer also fire before `shutdown` cut it short. Fixed by reordering: use-wire's full
   admission-through-release now runs entirely before character[0]'s final re-admission, which
   stays the literal last action. Per-stage `committed_admissions` checks renumbered (3rd, then
   4th); outer invariant unchanged (`committed_players==1`, `pending==0`, `committed_admissions==4`).

Replied once on the r4122215795 thread covering both findings.

## Repair round 3 (`1d1032c` -> `1d1032c` push): reconnect fence domain 2, shared lease used

Shared lease granted (#162 5870253781), scoped to the `Fnd02ReconciliationFenceV1`
domain-revision `vec` in `resume.rs` (~lines 237-248). Nothing else in that file changed: added
the `STATE_DOMAIN_WORLD_OBJECT_OVERLAY` import and one `StateDomainRevisionV1::new(...)` entry
after the existing spatial one (ascending domain id, per the fence's own strict-order check).
New `resume.rs` test module: both-domains-present/ordered, and a descending-order case proving
the ordering is load-bearing.

## Repair round 4 (`1d1032c` -> this push): overlay/spatial revision write ordering

P1 r4122508665 (Codex): `continuity.overlay_revision`/`spatial_revision` were written *before*
the frame(s) carrying that revision were confirmed sent, so a write failure after encoding could
still leave continuity claiming a revision the peer never received. Fixed in `connection.rs`
only, mirroring the existing sequencing exactly:
- join/resync snapshot path: `overlay_revision` is now set only after every snapshot frame,
  including `SnapshotCommit`, has been written successfully.
- USE delta path: `overlay_revision` is now set only after that delta's `write_frame` succeeds.
- STEP delta path: found the identical bug for `spatial_revision` (set before its `write_frame`)
  and fixed it the same way, set only after that `write_frame` succeeds.

New test `admitted_use_commit_write_failure_does_not_record_the_overlay_revision`: a
`FailNthWrite<S>` wrapper (deterministic — fails a specific `poll_write` call, not
buffer/timing-dependent like a dropped-reader duplex) lets the join snapshot's 3 frames and the
`USE_INTENT`'s own `CommandResult` (8 `poll_write` calls total: 2 per frame) through, then fails
the 5th frame's (the committed `WORLD_OBJECT_OVERLAY` delta) first write; asserts
`continuity.overlay_revision` stays at the pre-connection value, never advancing to the committed
entry's revision. Replied once on the r4122508665 thread.

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
- [x] `overlay_revision`/`spatial_revision` write-after-send ordering (round 4); regression test.
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
- result (round 4 head): fmt PASS; clippy PASS (no warnings); full `cargo test -p
  oteryn-game-server` PASS across every test binary (0 failed, only pre-existing topology-gated
  `#[ignore]`s); governance PASS; repository-policy PASS.

### Component/integration

- `gameplay_transport::connection::tests` (dispatch loop, overlay-continuity tests, and the new
  deterministic write-failure ordering test) PASS.
  `gameplay_transport::tests::use_object_occupancy_includes_every_committed_actor_not_only_the_issuer`
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

- exact head: pending push (round 4).
- method/reviewer: implementing agent (this session), addressing Codex's P1 r4122215795 (round
  2), the CI committed-actor-count finding (round 2), the domain-2 reconnect-fence entry under
  the granted shared lease (round 3, #162 5870253781), and P1 r4122508665's write-after-send
  ordering fix for `overlay_revision`/`spatial_revision` (round 4).
- material findings: all above, all fixed and unit-tested. The domain-2 fence entry was correctly
  deferred with `SHARED_LEASE_REQUIRED` until the control plane granted the exact-scoped lease,
  then made minimally. Round 4's write-ordering test uses a deterministic `poll_write`-counting
  wrapper after an earlier duplex-buffer/drop-based attempt proved non-deterministic.
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

## Terminal integration

- final frozen head `0a7feafa4c4fe7d06762f1259abf25be261c59eb`; independent review: Codex rounds on #1104: occupancy of all actors (r4121956127), revision-first ordering (5869579920), reconnect fence domain 2 (r4122215795, shared lease resume.rs #162 5870253781), post-transmission continuity update (r4122508665) repaired; final @codex review on the exact head found no issues; shared lease foundation/runtime_actor_carrier.rs committed_player_positions granted retroactively (#162 5869911385).
- exact-head CI green; merged through the Merge Queue as PR #1104, merge commit `0ff661e` (protected-main readback by the control plane, #162).
- owned paths released.

## Context checkpoint

```yaml
last_progress: Repair round 4 complete: overlay_revision/spatial_revision now written only after
  the frame(s) carrying them are confirmed sent (join snapshot, USE delta, and the same bug fixed
  for STEP's spatial_revision); new deterministic FailNthWrite-based regression test;
  fmt/clippy/full tests/both validators green; pushing.
status: completed
branch: claude/use-wire-m2b
head_sha: 0a7feafa4c4fe7d06762f1259abf25be261c59eb
pr: 1104
final_head_sha: 0a7feafa4c4fe7d06762f1259abf25be261c59eb
final_head_frozen_at: 2026-09-28T13:44:40Z
ci_trigger_source: push to claude/use-wire-m2b
ci_checks_for_current_head: 0
runner_assignment_state: unknown
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none (terminal)
```
