# OTV2-20260928-native-entry-door-m2a

```yaml
task_id: OTV2-20260928-native-entry-door-m2a
title: Native entry-room door M2a - one door, content side only
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: 5865792400
base_branch: main
branch: claude/native-entry-door-m2a
pr: 1075
base_sha: 13576c4463f51184c8a96f1bde6d536fe2f16c12
head_sha: bad4b3f96411e501d8dd7798e8fc7344955f92fd
final_head_sha: 8cf5ba5c4bc8f85802d0fc734b5d92dd9ad62b0f
final_head_frozen_at: 2026-09-28T10:42:39Z
owner: "Oteryn: content world runtime" (Claude Code)
created_at: 2026-09-28T08:00:00Z
updated_at: 2026-09-28T10:42:39Z
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
  - OTV2-20260928-use-wire-m2b (Server Seam builds the fence and calls the unchanged
    LocalObjectRuntime::bind; excluded here) — now allocated and in progress
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

M2a of the native entry-room door (#162 comment 5865792400, owner decision A4-a): the native
entry room gets exactly one usable door on the content side. `world_runtime.rs` was **not touched**
in this task.

- `native_entry.rs` / `native_entry_room.json`: a 4th walkable Terrain cell
  (`oteryn:cell/entry-door`, adjacent to `east`/`north`) plus a typed door overlay
  (`native_first_entry.doors`, exactly one element): one LocalObject, closed
  (collision-Present)/open (collision-Absent) states, two transitions (open, close).
  `NativeEntryProject::door()` exposes the door's own genuine Reference-profile content (real
  `REFERENCE_PLAYABLE_CONTENT_PROFILE_ID`, linked by the real `link_reference_playable`; never
  faked, never skipped), sharing the qualified project's own package/Content-Lock identity.
- Validator: exactly 3 room Terrain cells + 1 door cell + 1 door. Fails closed on zero/two doors,
  a door off the World frame, an unknown declared state, a mismatched transition, and now a
  changed door definition key. Cardinality is checked before any placement is indexed.
- `door().placements` is empty (DECISION_REQUIRED, see below) — genuinely, fully linked, no
  fabricated placement.

Excluded from M2a: any change to `world_runtime.rs`. M2b (Server Seam owner) builds the
`ScopeContentGenerationFence` at activation time and calls the unchanged `LocalObjectRuntime::bind`
against a placement it constructs itself from `door()` + `source()`.

## Codex round 2 (PR #1075) — three findings, all addressed

1. **r4120444668** (placement-cardinality panic): `state.placements[0]` was indexed before the
   4-placement cardinality check. Fixed: cardinality (`overlay.cells`, `overlay.doors`,
   `state.placements`) is now checked first, plus `state.placements.first().ok_or(...)` as a
   panic-free lookup regardless. New test `zero_placements_refuses_without_panicking`.
2. **r4120444680** (MATERIAL, provenance): `door()` appended a placement after
   `link_reference_playable` with fabricated `manifest-r0`/`Unknown` evidence, labeling the
   unlinked result canonical. **DECISION_REQUIRED, unresolved as a code fix, not fabricated**:
   `content/reference_playable.rs`'s `REFERENCE_TARGET_CLAIM_CASE_BINDINGS` is an accepted-empty
   array ("the evidence manifest currently has no `CONTENT_WORLD` mechanic case that can authorize
   any of these claims"), excluded/read-only here. `validate_placement` ->
   `require_reference_promotion` therefore refuses *every* placement's evidence unconditionally
   today, honest or not — proven by new test `door_placement_is_refused_by_the_real_linker_path`,
   which reconstructs `door()`'s own content plus a placement and re-runs it through the real
   linker (refused). Fix applied: the fabricated placement is removed entirely; `door()` now stays
   genuinely, fully linked with `placements` empty. A future consumer (M2b) derives the door's
   placement from `source()` + accepted keys and constructs its own synthetic,
   deliberately-unpromoted `PlacementRef`, exactly as `world_runtime.rs`'s existing CW4 test
   fixtures already do for ordinary Reference-sourced LocalObjects. Unblocking this for real
   requires a separately accepted `CONTENT_WORLD` evidence case bound in
   `REFERENCE_TARGET_CLAIM_CASE_BINDINGS` — outside this task's owned/excluded paths.
3. **r4120444694** (unpinned door identity): `accepted::DOOR_DEFINITION` was declared but never
   compared against the resolved door definition key. Fixed: `require_accepted_bindings` now takes
   `door: &CanonicalReferencePlayableContent` and pins
   `door.definitions[0].definition.key() == accepted::DOOR_DEFINITION`. New test
   `door_definition_key_must_match_the_accepted_binding` (record identity and
   `doors[0].definition.key` both changed to the same new key — still refused).

Replied once on each of the three review threads before pushing this fix.

## Codex round 3 (PR #1075) — two more findings, both addressed

1. **r4120672731 (P1)**: a closed door must not be walkable before M2b composes the runtime
   blocker. Fixed: `movement_cells()` now filters the door cell out of the claims it builds by key
   (`accepted::DOOR_CELL.0`), so the active `NativeEntryMovementCells` index is exactly the three
   room Terrain cells; the door cell stays `Walkable` in `source().cells` (needed for the
   accepted-bindings bijection) but is absent from active movement until M2b lands the runtime
   blocker and adds it atomically. Verified `content/activation.rs`/`movement.rs` need no change
   (opaque `movement_cells` carry, generic `EngineeringStaticCellIndex` consumer; no
   `SHARED_LEASE_REQUIRED`). New test `movement_cells_exclude_the_door_cell`, inside
   `native_entry.rs`'s own `#[cfg(test)] mod tests` since `NativeEntryMovementCells::index()`/
   `::scope()` and `static_cell_engine` are `pub(crate)`, unreachable from integration tests.
2. **r4120672740 (P2)**: door-adjacency distance arithmetic could overflow `i32` for a malformed
   project's extreme coordinates. Fixed: widened to `i64` before subtracting/summing. New test
   `door_adjacency_near_i32_extremes_refuses_without_panicking` (door near `i32::MAX`, a room cell
   near `i32::MIN`, World bounds widened to admit both) — confirmed it reproduces the original
   overflow panic against the pre-fix arithmetic, then passes cleanly with the fix.

Replied once on each of these two threads before pushing this fix.

## Architecture and source of truth

- `PROVEN`: #162 comment 5865792400 (owner decision A4-a) is this task's exact allocation/scope.
- `PROVEN`: `world_runtime`'s module declaration in `lib.rs` is `pub(crate) mod world_runtime;` —
  unreachable from integration tests; an earlier bind-split attempt was reverted to `origin/main`
  entirely after a separate control-plane review (tautological fence), before this Codex round.
- `PROVEN`: `content/reference_playable.rs:886`, `REFERENCE_TARGET_CLAIM_CASE_BINDINGS = &[]` —
  the exact, cited, currently-binding reason no placement can be linker-validated (finding 2).
- `PROVEN`: amendment doc §3/§4/§7 describes the door field and the 3-cells-plus-1-door-cell
  bijection; envelope unwidened at `[0,2) x [-1,1)`.
- `PROVEN`: `project/v2.rs` is read-only/excluded and unchanged.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No production mutation, no session/lease/generation/authority-consuming write, no PREPARE or
  COMMIT, no world_runtime.rs change. Content admission only (typed, in-memory, deterministic).
```

## Acceptance criteria

- [x] Room content: 4th walkable cell adjacent to the accepted three, one LocalObject door
      (closed/open, blocking when closed), two transitions (open, close).
- [x] Validator: exactly 3 Terrain cells + 1 door cell + 1 door, cardinality checked before
      indexing; door definition key pinned to `accepted::DOOR_DEFINITION`.
- [x] No faked profile ID; no skipped semantic validation; no fabricated/mislabeled placement.
- [x] Amendment doc updated: "exactly three" -> three room cells plus one door cell.
- [x] `world_runtime.rs` unchanged from `origin/main`.
- [x] Full focused-validation suite green (see Validation).
- [x] Real linker-validated door placement: DECISION_REQUIRED, blocked on an excluded-path
      evidence binding (see Codex round 2, finding 2); `door().placements` stayed empty for this
      task's scope. M2b (successor task) constructs its own synthetic, non-promotable
      `PlacementRef` at Channel activation, exactly as this task's own note anticipated — this
      criterion closes as designed, not as a gap.

## Validation

### Focused

- command/run: `cargo fmt --check -p oteryn-game-server`; `cargo clippy -p oteryn-game-server
  --all-targets -- -D warnings`; `cargo test -p oteryn-game-server`; `python3
  tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`
- result: fmt PASS; clippy PASS (no warnings); full `cargo test -p oteryn-game-server` PASS across
  every test binary (0 failed); `content_native_entry` 9/9 (3 new this round: zero-placements,
  linker-refusal, door-identity-pin); `content_native_entry_room` 2/2 (+1 ignored, unchanged);
  governance validator PASS; repository-policy validator PASS.
- `tools/qualification/native_entry_room/run.sh`: still `BLOCKED
  reason=exact_platform_checkout_missing` in this environment; unrelated to this change.

### Component/integration

- `NOT_APPLICABLE` — no Server Seam composition, no `world_runtime.rs` change, in M2a.

### E2E

- `NOT_APPLICABLE` — no end-to-end door path before M2b.

### Exact-head CI

- final head: `8cf5ba5c4bc8f85802d0fc734b5d92dd9ad62b0f` — green; merged.
- trigger source: push to `claude/native-entry-door-m2a`.

## Self-review

- exact head: `8cf5ba5c4bc8f85802d0fc734b5d92dd9ad62b0f`.
- method/reviewer: implementing agent (that session), addressing Codex's PR #1075 findings across
  rounds 2 and 3.
- material findings: r4120444668 (panic), r4120444680 (MATERIAL, fabricated evidence — decided
  DECISION_REQUIRED rather than fabricating a fix), r4120444694 (unpinned identity), r4120672731
  (P1, closed-door-walkable-before-blocker), r4120672740 (P2, i32 overflow). All either fixed or
  explicitly deferred with cited reason; replied on each thread before push.
- verdict: ready to freeze for the content-only scope; finding 2's real fix needs a separately
  accepted evidence case outside this task, deferred to a future content-world task.

## Independent review

- required: YES — content admission graph change, per root governance norm.
- exact head: `8cf5ba5c4bc8f85802d0fc734b5d92dd9ad62b0f`.
- method/auditor: Codex, automated PR review (not triggered by this worker); rounds 2 and 3
  findings addressed above; no further findings on the frozen head.
- verdict: accepted.

## PR and closeout

- PR #1075 merged into `main` as merge commit `69284a571a3b58249546e17f992dff59883be42f`
  ("feat(content): native entry-room door M2a - one door, content side only (#1075)").
- changed-file review: all changes inside owned_paths.
- independent review: Codex rounds 2 and 3 findings addressed on the frozen head; threads
  resolved.
- exact-head CI: green on `8cf5ba5c4bc8f85802d0fc734b5d92dd9ad62b0f`; merged through the Merge
  Queue; protected `main` readback confirms merge commit `69284a5`.
- ownership release: all owned paths released. M2b (Server Seam composition,
  `OTV2-20260928-use-wire-m2b`) is the successor task, now allocated and in progress on #162.
- related/superseded PRs: none known.

## Context checkpoint

```yaml
last_progress: PR #1075 merged into main as 69284a5; task terminal.
status: completed
branch: claude/native-entry-door-m2a
head_sha: bad4b3f96411e501d8dd7798e8fc7344955f92fd
pr: 1075
final_head_sha: 8cf5ba5c4bc8f85802d0fc734b5d92dd9ad62b0f
final_head_frozen_at: 2026-09-28T10:42:39Z
ci_trigger_source: null
ci_checks_for_current_head: 0
runner_assignment_state: unknown
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none (terminal); successor OTV2-20260928-use-wire-m2b allocated separately on #162
```
