# OTV2-20260928-native-entry-door-m2a

```yaml
task_id: OTV2-20260928-native-entry-door-m2a
title: Native entry-room door M2a - one door, content side only
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: 5865792400
base_branch: main
branch: claude/native-entry-door-m2a
pr: 1075
base_sha: 13576c4463f51184c8a96f1bde6d536fe2f16c12
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world runtime" (Claude Code)
created_at: 2026-09-28T08:00:00Z
updated_at: 2026-09-28T10:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/project/native_entry.rs
  - apps/game-server/src/content/project/native_entry_room.json
  - apps/game-server/tests/content_native_entry.rs
  - apps/game-server/tests/content_native_entry_room.rs
  - docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_NATIVE_ENTRY_QUALIFICATION_AMENDMENT_01.md
  - docs/agents/tasks/active/OTV2-20260928-native-entry-door-m2a.md
public_contracts: []
depends_on: []
blocks:
  - OTV2 native-entry-door-m2b (Server Seam builds the fence and calls the unchanged
    LocalObjectRuntime::bind; excluded here)
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

M2a of the native entry-room door (#162 comment 5865792400, owner decision A4-a): the native
entry room gets exactly one usable door on the content side. `world_runtime.rs` is **not
touched** in this task (control-plane review found the original bind-split attempt made the
active-generation fence a tautology when derived from the same content it validates; reverted to
`origin/main` entirely, no bind split, no new `LocalObjectRuntime` entry point).

- `native_entry.rs` / `native_entry_room.json`: a 4th walkable Terrain cell
  (`oteryn:cell/entry-door`, adjacent to `east`/`north`) plus a typed door overlay
  (`native_first_entry.doors`, exactly one element): one LocalObject, closed
  (collision-Present)/open (collision-Absent) states, two transitions (open, close).
  `NativeEntryProject::door()` exposes the door's own genuine Reference-profile content (real
  `REFERENCE_PLAYABLE_CONTENT_PROFILE_ID`, linked by the real `link_reference_playable`; never
  faked, never skipped), sharing the qualified project's own package/Content-Lock identity.
  FirstProduction still carries no LocalObject record.
- Validator: still exactly the three room Terrain cells (`NATIVE_ENTRY_CELLS`) bijectively, plus
  exactly one door cell (`NATIVE_ENTRY_DOOR_CELLS = 1`) and exactly one door. Fails closed on
  zero/two doors, a door off the World frame, an unknown declared state, a mismatched transition.
  The World envelope is unchanged (`accepted::BOUNDS` stays `[0,2) x [-1,1)`, exactly fitting the
  four placed cells) — not grown to make refusal tests distinct.
- Tests: `content_native_entry.rs` gains `door_admission_refuses_every_invariant_mutation` (no
  door, two doors, off frame, unknown state x2, bad transition) plus door assertions in the
  deterministic-pair test. `content_native_entry_room.rs` updates its cell-count assertion.

Excluded from M2a (this delivery): any change to `world_runtime.rs`, `LocalObjectRuntime::bind`,
or a native-entry bind entry point. M2b (Server Seam owner) builds the
`ScopeContentGenerationFence` at activation time from the activated room's door and calls the
unchanged `LocalObjectRuntime::bind` — see Implementation / findings for the exact fence
constructor that exists today and its visibility.

## Architecture and source of truth

- `PROVEN`: #162 comment 5865792400 (owner decision A4-a) is this task's exact allocation/scope.
- `PROVEN`: control-plane review (this session, before freeze): `bind_native_entry_door` deriving
  its `ScopeContentGenerationFence` from the same `door` content it then validates makes
  `validate_candidate` a tautology (any Reference-profile content would bind), removing the
  active-generation fencing `bind` enforces. Rejected; `world_runtime.rs` reverted to
  `origin/main` entirely.
- `PROVEN`: readback facts remain true — FirstProduction still has no LocalObject record;
  `LocalObjectRuntime::bind` is completely unchanged from `origin/main`.
- `PROVEN`: amendment doc §3/§4/§7 updated to describe the door field and the
  3-cells-plus-1-door-cell bijection, same allocation comment; envelope wording reverted to the
  unwidened `[0,2) x [-1,1)`.
- `PROVEN`: `project/v2.rs` is read-only/excluded and unchanged; the door adds no placement field
  there.
- `PROVEN`: `world_runtime`'s module declaration in `lib.rs` is `pub(crate) mod world_runtime;` —
  `LocalObjectRuntime`, `bind` and `ScopeContentGenerationFence` are not reachable from integration
  tests (`tests/content_native_entry*.rs` link only the crate's public API), so the
  bind-through-`bind`-with-matching/foreign-generation test the control plane asked for cannot be
  written there; reported instead of forced. See Implementation / findings.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No production mutation, no session/lease/generation/authority-consuming write, no PREPARE or
  COMMIT, no controller install/restore, no persisted-recovery-evidence interpretation, and no
  world_runtime.rs change at all in this delivery. Content admission only (typed, in-memory,
  deterministic).
```

## Acceptance criteria

- [x] Room content: 4th walkable cell adjacent to the accepted three, one LocalObject door
      (closed/open, blocking when closed), two transitions (open, close).
- [x] Validator: exactly 3 Terrain cells + 1 door cell + 1 door; every other shape fails closed.
- [x] No faked profile ID; no skipped semantic validation.
- [x] Amendment doc updated: "exactly three" -> three room cells plus one door cell.
- [x] Minimal: exactly one door; World envelope not grown to make tests distinct.
- [x] `world_runtime.rs` unchanged from `origin/main`; no bind split, no native-entry bind entry
      point (control-plane blocker addressed).
- [x] Full focused-validation suite green (see Validation).
- [ ] M2a binding-behavior test (door() binds through unchanged `bind` with a matching-generation
      fence, refused with a foreign one): NOT WRITTEN — `bind`/`LocalObjectRuntime` are
      `pub(crate)` and unreachable from `tests/content_native_entry*.rs`; reported per control
      plane's own fallback instruction rather than forced into an unsuitable location.

## Implementation / findings

- **Exact fence constructor for M2b**: `ScopeContentGenerationFence::for_test` — the *only*
  constructor in `world_runtime.rs`, gated `#[cfg(test)] pub(crate)`
  (`apps/game-server/src/world_runtime.rs:93-105`, unchanged from `origin/main`). It is not usable
  from non-test crate code, so M2b's Server Seam owner has no crate-visible way today to construct
  a `ScopeContentGenerationFence` in production code; they will need to add a real (non-test)
  constructor themselves as part of that composition. Not added here per instruction.
- `NATIVE_ENTRY_CELLS` (= 3) unchanged; new `NATIVE_ENTRY_DOOR_CELLS = 1` names the door's cell.
  `grep NATIVE_ENTRY_CELLS` found one consumer outside `native_entry.rs`:
  `tests/content_native_entry_room.rs`, updated to assert the summed total plus one door placement.
- The door's Reference content shares `package_manifest`/`content_lock` with the FirstProduction
  source (both from the same `project.manifest`/`project.lock`), so its
  `ReferenceContentGeneration` (computed by M2b from `door()`, using the existing `bind`) is
  inherently the qualified project's own generation.
- World envelope: reverted to `[0,2) x [-1,1)`, exactly fitting the four placed cells. The
  in-bounds "not adjacent" and out-of-bounds "off the room frame" refusals collapse into one test
  in this fully-packed 2x2 room (every in-bounds coordinate is already occupied); the adjacency
  check itself (`native_entry.rs`, "native entry door cell must be adjacent to the entry room")
  stays in the code — every passing admission still has to satisfy it — but is not independently
  exercised by a distinct in-bounds negative case, per the control plane's explicit instruction not
  to grow geometry for that purpose.
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
  every test binary (0 failed), including `world_runtime` lib tests unchanged from `origin/main`;
  `content_native_entry` 6/6 (4 updated + 1 new merged door test); `content_native_entry_room` 2/2
  (+1 ignored, unchanged); governance validator PASS; repository-policy validator PASS.

### Component/integration

- `NOT_APPLICABLE` — no Server Seam composition, no `world_runtime.rs` change, in M2a.

### E2E

- `NOT_APPLICABLE` — no end-to-end door path before M2b.

### Exact-head CI

- final head: pending — see the live PR for current head/checks.
- trigger source: push to `claude/native-entry-door-m2a`; pending.

## Self-review

- exact head: pending (filled once pushed).
- method/reviewer: implementing agent (this session), after a control-plane review rejected the
  original `bind_native_entry_door` design (tautological fence) before freeze.
- material findings: the rejected fence design above; repaired by fully reverting
  `world_runtime.rs` rather than patching it, per control-plane instruction.
- verdict: ready to freeze for the content-only scope; the bind-side composition remains M2b's.

## Independent review

- required: YES — content admission graph change, per root governance norm.
- exact head: pending.
- method/auditor: not yet requested; this worker does not trigger `@codex`.
- verdict: pending independent review.

## PR and closeout

- PR #1075 updated on this corrected head; changed-file review / unresolved threads / protected
  auto-merge / merge commit / ownership release: pending — see the live PR and #162 for current
  status, not tracked in this file.
- related/superseded PRs: none known; overlap check found no open PR touching these owned paths.

## Context checkpoint

```yaml
last_progress: control-plane blocker addressed - world_runtime.rs reverted to origin/main
  entirely, World-envelope widening reverted, PR/task record updated; full focused-validation
  suite green post-fix.
status: validating
branch: claude/native-entry-door-m2a
head_sha: null
pr: 1075
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: push to claude/native-entry-door-m2a
ci_checks_for_current_head: 0
runner_assignment_state: unknown
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: push the fix commit (no force), update PR body, await CI/exact-head readback and
  independent review on PR #1075; no @codex trigger, no comment on #162
```
