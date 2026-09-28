# OTV2-20260927-cw4-world-object-overlay

```yaml
task_id: OTV2-20260927-cw4-world-object-overlay
title: Generalize CW4 LocalObjectRuntime from Open/Close to typed D38 world-object operations
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cw4-world-object-overlay
issue: 162
pr: 1055
base_sha: 3426839ddc00e18f14968c60f7eeb6a523e2884f
head_sha: bd68f493252d2b2888382c698d6b06d5172259ce
final_head_sha: bd68f493252d2b2888382c698d6b06d5172259ce
final_head_frozen_at: 2026-09-28T01:12:00Z
owner: Oteryn: content world runtime (session_01PwTJFS62J35S88Srpqnrgx)
created_at: 2026-09-28T01:05:47Z
updated_at: 2026-09-28T01:45:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_runtime.rs
  - docs/agents/tasks/archive/OTV2-20260927-cw4-world-object-overlay.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop
window.

## Outcome

`LocalObjectRuntime` in `apps/game-server/src/world_runtime.rs` no longer hard-codes the two-state
Open/Close pair. `bind()` accepts an arbitrary, non-empty set of pre-authored `TransitionKey`s over
a `LocalObject` definition's declared state vocabulary (any size); `prepare()` derives the next
blocking footprint, and the OCCUPIED check, from the *target* state's own authored
`LocalObjectCollisionPresence` (#1046's per-state model) instead of which named operation was
invoked. `LocalObjectOperation` is a thin wrapper around a bound `TransitionKey`: TRANSFORM
(arbitrary from/to), CREATE/REMOVE (fixed-footprint anchor toggling collision-Present/Absent) and
RETAG (same-collision-class rearm, `LOCAL_OBJECT_RETAG_INTENT_FAMILY`) are all "invoke one of this
runtime's bound transitions" — no separate Rust variant per D38 operation name. Open/Close become
the two-state special case of this mechanism, not a distinct code path.

## Architecture and source of truth

- `docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §4/§6
  (D38, accepted 2026-09-27): overlay operations are `TRANSFORM(from,to)`, `CREATE(def)`,
  `REMOVE(def)`, `RETAG(action)`; state is scope-ephemeral; `revert_after`/timers out of scope
  (`PROVEN`, read directly). §6 W1: scope runtime owns extending the local-transition candidate to
  these four operations.
- PR #1046 (merged, `832320a`): `LocalObjectStates` is per-state `{key, collision}`;
  `PlacementRef.local_object_initial_state` is the authored start; `LOCAL_OBJECT_RETAG_INTENT_FAMILY`
  plus the linker's `validate_transition` (RETAG must connect same-collision-class states) already
  fully enforce C1 fail-closed at the content layer for every transition in `content.transitions`,
  not only bound ones, since `bind()`'s `validate_reference_semantic_core` re-links the whole
  content every call (`PROVEN`, read directly).
- Issue #162 allocation comments 5860129276 / 5861341063 (text supplied verbatim in the worker
  prompt; not re-fetched, per context-economy bootstrap).

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: >
  No production mutation gated by session/lease/generation/authority fence evidence; no
  PREPARE/COMMIT authorization; no controller install/restore; no authority-bearing session
  replacement; no persisted recovery evidence interpretation. This task generalizes an in-process,
  non-persisted (D38 W2: scope-ephemeral) CW4 overlay runtime's state-machine execution; it does
  not touch protocol, session, admission, persistence or Foundation fencing.
```

## Acceptance criteria

- [x] `bind()` accepts any non-empty set of `TransitionKey`s over any number of declared states
      (not just two); each must target the placement's own definition, reference only declared
      states, carry the supported capability and no policy guard, else `InvalidBinding` (C1, C3).
      Evidence: `binding_with_no_transitions_is_rejected_before_runtime_creation`,
      `binding_a_foreign_definition_transition_is_rejected_before_runtime_creation`.
- [x] `prepare()`'s next blocking footprint and OCCUPIED check come from the transition's *target*
      state's own authored collision presence, not a hard-coded Open=empty/Close=full mapping.
      Evidence: `create_retag_and_transform_drive_blocking_from_the_target_states_own_collision`.
- [x] TRANSFORM, CREATE, REMOVE, RETAG each have a positive and a fail-closed negative case.
      Evidence: `create_retag_and_transform_drive_blocking_from_the_target_states_own_collision`
      (TRANSFORM/CREATE/RETAG positive), `create_then_remove_round_trips_and_replay_of_create_does_not_reexecute`
      (REMOVE + replay), `create_onto_an_occupied_footprint_is_rejected_and_leaves_no_partial_state`
      (CREATE/OCCUPIED negative), `retag_across_collision_classes_is_rejected_by_the_content_layer_before_runtime_creation`
      (RETAG negative).
- [x] Every existing semantic preserved: command identity, `validate_current_authority` fences,
      per-placement revision, STALE_STATE, NO_CHANGE, OCCUPIED, replay, FIFO ordering, atomic
      footprint. Evidence: all 19 pre-existing `world_runtime` tests pass with unchanged behavior
      (only mechanically adapted signatures — see Implementation/findings).
- [x] Scope restart / fresh `bind()` restores the authored initial state.
      Evidence: `fresh_bind_after_scope_restart_restores_the_authored_initial_state`.
- [x] Open/Close callers and tests keep passing, expressed as the special case.
      Evidence: `open_operation()`/`close_operation()` helpers build `LocalObjectOperation` from
      the same OPEN/CLOSE `TransitionKey`s; all prior Open/Close tests pass.

## Excluded scope

- `revert_after` / timers / scope progression: not touched; no clock or timer added.
- `content/**` (`reference_playable.rs`, `project.rs`): not touched. The content model already
  fully supports this generalization (PR #1046); no content-model change was needed, so no
  `SHARED_LEASE_REQUIRED`.
- protocol registry, wire/egress, `gameplay_transport`, Foundation, `lib.rs`, Cargo files,
  persistence/migrations, DUR-03, quest state, D37 relocation, `tools/**`, `imports/**`: not
  touched. `LocalObjectRuntime`/`Operation`/`Command` stay `pub(crate)` with no external caller in
  this repository (verified), so no wiring surface exists yet.
- creature removals: out of scope (C4); this runtime only toggles a placement's own pre-authored
  anchor, never a creature.

## Implementation / findings

- `bind()`: fixed `open_transition_key`/`close_transition_key` pair -> `transition_keys:
  &[TransitionKey]` (non-empty). `states.len() == 2` -> `!states.is_empty()`. Removed OPEN/CLOSE-
  only structural checks no longer meaningful once operation identity is the bound `TransitionKey`
  itself (two-state inverse pair, distinct intent families, intent-family-to-operation matching,
  bind-time Open=Present/Close=Absent collision assumption the objective calls out for removal).
  Kept, generalized over the whole bound set: transition targets this placement's definition;
  source/target states declared; capability is `LOCAL_OBJECT_TRANSITION_CAPABILITY`; no policy
  guards; no duplicate key. Unknown/undeclared states and cross-class RETAG stay fail-closed via
  the CW3 linker over the whole content (see Architecture), not re-implemented here.
- Struct: `open_transition`/`close_transition` -> `states: Vec<LocalObjectStateDefinition>` +
  `transitions: BTreeMap<TransitionKey, TransitionBinding>`.
- `LocalObjectOperation`: was `enum {Open,Close}` (`Copy`); now a tuple struct wrapping
  `TransitionKey`. `transition_for()` is a fallible lookup (`InvalidBinding("command names a
  transition this local-object runtime does not bind")`) instead of an infallible two-arm match.
- `prepare()`: `match operation {Open=>empty,Close=>full}` and the `operation==Close` OCCUPIED
  gate -> one `target_collision = local_object_state_collision(&self.states,
  &transition.target_state)` lookup used for both. The old "state escaped the bound two-state
  transition" hard `Err` (unreachable under the old exclusive two-state model) is now a soft
  `DISPOSITION_STALE_STATE`: with >2 states/>1 outgoing transition, "source state != current" is
  a normal, replay-safe mismatch, same kind as `expected_revision` — not an invariant bug.
- Tests: added `world_object_overlay_content` (absent/present/armed/dormant + a second, unrelated
  `LocalObject` definition for the foreign-transition negative test), exercising
  CREATE/REMOVE/RETAG/TRANSFORM and the new fail-closed cases. Replaced the two Open/Close-only
  negative tests whose asserted text encoded removed checks with
  `binding_with_no_transitions_is_rejected_before_runtime_creation` and
  `binding_a_foreign_definition_transition_is_rejected_before_runtime_creation`. All other
  pre-existing tests kept, mechanically adapted: `Operation::Open`/`::Close` call sites now use
  `open_operation()?`/`close_operation()?` helpers; two direct `bind(...)` sites pass a 2-slice.

## Validation

Commands (`CARGO_TARGET_DIR=/home/user/.cargo-target-cw4`, one build at a time):
`cargo fmt --check -p oteryn-game-server`;
`cargo clippy -p oteryn-game-server --all-targets -- -D warnings`;
`cargo test -p oteryn-game-server world_runtime`;
`python3 tools/agents/validate_governance.py`;
`python3 tools/repository/validate_repository_policy.py`.

Result: fmt clean; clippy clean (0 warnings under `-D warnings`, exit 0); `world_runtime` 26/26
passed (19 pre-existing + 7 new: 2 bind-time negative, 1 TRANSFORM/CREATE/RETAG positive, 1
CREATE/REMOVE round-trip + replay, 1 CREATE/OCCUPIED negative, 1 RETAG cross-class negative via
content layer, 1 unbound-operation negative, 1 scope-restart restore). Both validators: PASS.

### E2E

- scenario: NOT_APPLICABLE — `LocalObjectRuntime` has no external caller yet (verified: no
  reference outside `world_runtime.rs`); no gameplay E2E path exists to exercise.

### Exact-head CI

- final head/trigger/workflow/run/job/runner/classification/result: pending (frozen at commit
  time; exact-head CI is the coordinator's/Merge Queue's read after PR open)

## Self-review

- exact head: bound at commit (see PR)
- method/reviewer: implementing agent (mandatory; cannot be delegated away)
- material findings: none found. Two points worth an independent reviewer's attention: (a)
  removing the bind-time "OPEN source must be Present, CLOSE target must be Absent" check
  (intentional per the objective; the content layer's RETAG-only collision check is unaffected);
  (b) reusing `DISPOSITION_STALE_STATE`, not a new disposition, for the newly-reachable
  "operation's source state != current state" case that >2 states / >1 outgoing transition makes
  possible.
- verdict: no blocking findings; ready for review

## Independent review

- Required by the #162 allocation (comments 5860129276 and 5861341063). One `@codex review` was requested by the control plane on the exact frozen head `bd68f49` (PR comment 5861594751).
- Verdict: "Didn't find any major issues" (PR comment 5861618806). No review thread was opened.

## PR and closeout

- PR #1055, one commit, exactly the owned paths. The freeze is recorded in #162 comment 5861595726.
- Exact-head PR CI: aggregate `game-gate` SUCCESS (job 108751938928), including Rust Linux workspace, Server Seam over TCP+TLS and Node boot against the real Platform.
- Merge Queue: `merge_group` run 36365974694 SUCCESS, base `ac6d820b`, head `070d119dda60c25159bee8670e069867993ebad0`.
- Protected `main@070d119` read back: `world_runtime.rs` and this record are byte-identical to the frozen head.
- Ownership of `apps/game-server/src/world_runtime.rs` is released with this archive.
- Not done by this task: `revert_after` (#1045 parked); wire/egress for world-object commands (A2 decision packet, #162 comment 5860722313).

## Context checkpoint

```yaml
last_progress: merged through the Merge Queue as 070d119; protected-main readback identical
status: completed
branch: claude/cw4-world-object-overlay
head_sha: bd68f493252d2b2888382c698d6b06d5172259ce
pr: 1055
final_head_sha: bd68f493252d2b2888382c698d6b06d5172259ce
final_head_frozen_at: 2026-09-28T01:12:00Z
owner_action_required: null
blocker: null
next_action: none; the successor wire/egress work waits on the A2 owner decision
```
