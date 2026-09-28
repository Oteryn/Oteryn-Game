# OTV2-20260928-native-entry-door-m2a

```yaml
task_id: OTV2-20260928-native-entry-door-m2a
title: Native entry-room door M2a - one door, CW4-bindable
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: 5865792400
base_branch: main
branch: claude/native-entry-door-m2a
pr: 1075
base_sha: 13576c4463f51184c8a96f1bde6d536fe2f16c12
head_sha: 2f79cffc0716f7a03b112f90c3b357d91e3bbcd2
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world runtime" (Claude Code)
created_at: 2026-09-28T08:00:00Z
updated_at: 2026-09-28T09:30:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/content/project/native_entry_room.json
  - apps/game-server/src/world_runtime.rs  # the bind split only
  - apps/game-server/tests/content_native_entry.rs
  - apps/game-server/tests/content_native_entry_room.rs
  - docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_NATIVE_ENTRY_QUALIFICATION_AMENDMENT_01.md
  - docs/agents/tasks/active/OTV2-20260928-native-entry-door-m2a.md
public_contracts: []
depends_on: []
blocks:
  - OTV2 native-entry-door-m2b (Server Seam composition of the door bind, excluded here)
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

M2a of the native entry-room door (#162 comment 5865792400, owner decision A4-a): the native
entry room served by the Server Seam gets exactly one usable door, and the CW4 kernel can bind it.

- `native_entry.rs` / `native_entry_room.json`: adds a 4th walkable Terrain cell
  (`oteryn:cell/entry-door`, adjacent to `east`/`north`) plus a typed door overlay
  (`native_first_entry.doors`, exactly one element): one LocalObject, closed
  (collision-Present)/open (collision-Absent) states, two transitions (open, close).
  `NativeEntryProject::door()` exposes the door's own genuine Reference-profile content (real
  `REFERENCE_PLAYABLE_CONTENT_PROFILE_ID`, linked by the real `link_reference_playable`; never
  faked, never skipped), sharing the qualified project's own package/Content-Lock identity.
  FirstProduction still carries no LocalObject record — this is a second, coherent content
  alongside the FirstProduction source, not an addition to it.
- Validator: still exactly the three room Terrain cells (`NATIVE_ENTRY_CELLS`) bijectively, plus
  exactly one door cell (`NATIVE_ENTRY_DOOR_CELLS = 1`) and exactly one door. Everything else
  fails closed: zero/two doors, a door cell not a unit step from a room cell, a door cell outside
  the World envelope, an unknown declared state, a mismatched transition — each its own reason.
  The envelope grew by one otherwise-unused column (`accepted::BOUNDS`, now `[0,3) x [-1,1)`) so
  "not adjacent" and "outside the World" are distinct, both-tested refusals.
- `world_runtime.rs`: `LocalObjectRuntime::bind` split into per-source validation
  (`resolve_local_object_placement`) plus one shared private constructor (`construct`); `bind`
  itself is a thin wrapper, byte-for-byte unchanged (26/26 pre-existing tests pass unchanged). New
  `bind_native_entry_door` is the native-entry source function: derives the door's own
  `ReferenceContentGeneration` (the one Reference-profile CW4 generation type this seam has; no
  separate native-entry generation type exists to reuse) and a fence derived directly from that
  same content (new `ScopeContentGenerationFence::for_native_entry_door`), then calls the same
  shared `construct`. No profile faking, no semantic-validation skip, in either path.
- Tests: `content_native_entry.rs` gains `door_admission_refuses_every_invariant_mutation` (no
  door, two doors, not adjacent, off frame, unknown state x2, bad transition) plus door assertions
  in the deterministic-pair test. `content_native_entry_room.rs` updates its cell-count assertion.
  `world_runtime.rs` gains `native_entry_door_binds_opens_closes_and_is_blocking_only_when_closed`
  against the real committed room (closed+blocking start, open clears blocking, close re-blocks).

Excluded (M2b): wiring `bind_native_entry_door` into the Server Seam (`gameplay_transport/**`,
`content/activation.rs`) so a live Channel dispatches open/close commands to the door, plus the
end-to-end use/occupied/stale-state proof. Separately allocated.

## Architecture and source of truth

- `PROVEN`: #162 comment 5865792400 (owner decision A4-a) is this task's exact allocation/scope.
- `PROVEN`: readback facts remain true — FirstProduction still has no LocalObject record (the
  door is a second Reference-profile content, not an addition to `FirstProductionContentSource`);
  `LocalObjectRuntime::bind`'s Reference-profile behavior is unchanged.
- `PROVEN`: amendment doc §3/§4/§7 updated to describe the door field and the
  3-cells-plus-1-door-cell bijection, same allocation comment.
- `PROVEN`: `project/v2.rs` is read-only/excluded and unchanged; the door adds no placement field
  there. Its `PlacementRef` (Reference profile) is built directly in `native_entry.rs` from the
  same coordinates as its Terrain placement, exactly as existing CW4 test fixtures in
  `world_runtime.rs` already build synthetic `PlacementRef`s (`EvidenceDisposition::Unknown`).
- `DERIVED`: `gameplay_transport/qualification.rs` and `content/activation.rs` (read-only) bind no
  `LocalObjectRuntime` for the native entry room yet (M2b), so there is no existing
  native-entry-specific generation type to reuse; `ReferenceContentGeneration` is the one
  Reference-profile CW4 generation type the seam has, reused unmodified. No `SHARED_LEASE_REQUIRED`;
  no excluded path written.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No production mutation, no session/lease/generation/authority-consuming write, no PREPARE or
  COMMIT, no controller install/restore, no persisted-recovery-evidence interpretation. Content
  admission (typed, in-memory, deterministic) plus a CW4 bind-time constructor split;
  `LocalObjectRuntime::bind`'s existing authority/generation-fence checks are exercised unchanged
  by the 26 pre-existing tests plus the one new door test.
```

## Acceptance criteria

- [x] Room content: 4th walkable cell adjacent to the accepted three, one LocalObject door
      (closed/open, blocking when closed), two transitions (open, close).
- [x] Validator: exactly 3 Terrain cells + 1 door cell + 1 door; every other shape fails closed.
- [x] `bind` split into per-source validation + one shared private constructor; Reference path
      byte-for-byte unchanged; native-entry source function added and tested.
- [x] No faked profile ID; no skipped semantic validation.
- [x] Content generation/fence is the active native-entry content's own.
- [x] Amendment doc updated: "exactly three" -> three room cells plus one door cell.
- [x] Minimal: exactly one door; no generalized multi-door machinery.
- [x] Full focused-validation suite green (see Validation).

## Implementation / findings

- `NATIVE_ENTRY_CELLS` (= 3) unchanged; new `NATIVE_ENTRY_DOOR_CELLS = 1` names the door's cell.
  `grep NATIVE_ENTRY_CELLS` found one consumer outside `native_entry.rs`:
  `tests/content_native_entry_room.rs`, updated to assert the summed total plus one door placement.
- The door's Reference content shares `package_manifest`/`content_lock` with the FirstProduction
  source (both from the same `project.manifest`/`project.lock`), so its
  `ReferenceContentGeneration` is inherently the qualified project's own generation.
- `tools/qualification/native_entry_room/run.sh`: exits `BLOCKED
  reason=exact_platform_checkout_missing` in this environment (no `_platform` checkout at the
  pinned Platform SHA) — the same environmental gate it documents for any worker without that
  checkout, not a regression here. Not included in the green result below.

## Validation

### Focused

- command/run: `cargo fmt --check -p oteryn-game-server`; `cargo clippy -p oteryn-game-server
  --all-targets -- -D warnings`; `cargo test -p oteryn-game-server`; `python3
  tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`
- result: fmt PASS; clippy PASS (no warnings); full `cargo test -p oteryn-game-server` PASS across
  every test binary (0 failed); `world_runtime` lib tests 27/27 (26 pre-existing unchanged + 1
  new); `content_native_entry` 6/6 (4 updated + 2 new); `content_native_entry_room` 2/2 (+1
  ignored, unchanged); governance validator PASS; repository-policy validator PASS.

### Component/integration

- `NOT_APPLICABLE` — no Server Seam composition in M2a.

### E2E

- `NOT_APPLICABLE` — no end-to-end door path before M2b wires Server Seam dispatch.

### Exact-head CI

- final head: pending — see the live PR for current head/checks.
- trigger source: push to `claude/native-entry-door-m2a`; pending.

## Self-review

- exact head: pending (filled once pushed).
- method/reviewer: implementing agent (this session).
- material findings: test-authoring fixes only (v2 placements/records are written key-sorted, not
  insertion-order; fixed before freeze, not deferred).
- verdict: ready to freeze.

## Independent review

- required: YES — content admission graph and CW4 kernel change, per root governance norm.
- exact head: pending.
- method/auditor: not yet requested; this worker does not trigger `@codex`.
- verdict: pending independent review.

## PR and closeout

- PR #1075 opened on head `2f79cffc0716f7a03b112f90c3b357d91e3bbcd2`. Changed-file review /
  unresolved threads / protected auto-merge / merge commit / ownership release: pending — see the
  live PR and #162 for current status, not tracked in this file.
- related/superseded PRs: none known; overlap check found no open PR touching these owned paths.

## Context checkpoint

```yaml
last_progress: PR #1075 opened on 2f79cffc0716f7a03b112f90c3b357d91e3bbcd2; full
  focused-validation suite green pre-push (fmt, clippy, cargo test full suite, governance,
  repository-policy).
status: validating
branch: claude/native-entry-door-m2a
head_sha: 2f79cffc0716f7a03b112f90c3b357d91e3bbcd2
pr: 1075
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: push to claude/native-entry-door-m2a
ci_checks_for_current_head: 0
runner_assignment_state: unknown
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: await CI/exact-head readback and independent review on PR #1075; no @codex trigger,
  no comment on #162, per this task's explicit instruction
```
