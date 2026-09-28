# OTV2-20260928-cw3-local-object-state-model

```yaml
task_id: OTV2-20260928-cw3-local-object-state-model
title: LocalObject per-state collision presence, authored initial state, RETAG intent family
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cw3-local-object-state-model
issue: 162
pr: 1046
base_sha: dd209a1264e98f3d1f0f167ec3320124a071db53
head_sha: 975f077eec3ec8ad5f9d2f5333709f6cbe7c7a03
final_head_sha: 975f077eec3ec8ad5f9d2f5333709f6cbe7c7a03
final_head_frozen_at: 2026-09-27T23:22:24Z
owner: Oteryn: content world build (session_01LphUANMfC2q2WKdfEb39eC)
created_at: 2026-09-27T22:30:02Z
updated_at: 2026-09-28T00:39Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/project.rs
  - apps/game-server/tests/content_reference_playable.rs
  - docs/agents/tasks/active/OTV2-20260928-cw3-local-object-state-model.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop
window.

## Outcome

`ReferenceDefinitionKind::LocalObjectStates` becomes a typed per-state vocabulary
(`{key, collision: Present|Absent}`) instead of a bare `Vec<ProductionKey>`, so a CW4-family
runtime child can read a state's collision presence directly instead of deriving it from an
Open/Close intent pair. A `PlacementRef` targeting a `LocalObject` definition now carries an
authored initial state, validated fail-closed at link time against the definition's declared
state vocabulary. The RETAG coordinator decision (D38 follow-up, allocation 1c) is recorded as a
named intent-family constant plus a linker invariant: a RETAG transition may only connect two
states of the same collision class; no action-id field is added anywhere in the model.

## Architecture and source of truth

- `docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §4/§6
  (D38, accepted 2026-09-27): world-object overlay operations are `TRANSFORM`, `CREATE`, `REMOVE`,
  `RETAG` over pre-authored anchors; state itself is scope-ephemeral runtime overlay, not part of
  this task (`PROVEN`, read directly).
- Issue #162 comment 5860394705 (allocation given verbatim in the worker prompt): task
  `OTV2-20260928-cw3-local-object-state-model`, objective 1a/1b/1c (`PROVEN`, allocation text
  supplied by the coordinator; not independently re-fetched per the context-economy bootstrap
  instruction).
- `apps/game-server/src/content/reference_playable.rs` (~L809 `LocalObjectStates`, ~L1268
  `PlacementRef`, ~L1905 linker validation, ~L1344/~L1392 client-safe projection) and
  `apps/game-server/src/world_runtime.rs` (~L630-720 first CW4 child bind site): `PROVEN`, read
  directly this task.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: >
  No production mutation gated by session/lease/generation/authority fence evidence; no
  PREPARE/COMMIT authorization; no controller install/restore; no authority-bearing session
  replacement; no persisted recovery evidence interpretation. This task only extends an immutable
  Content-linking model (definitions/placements/transitions) validated at link/compile time.
```

## Acceptance criteria

- [ ] `ReferenceDefinitionKind::LocalObjectStates` entries carry `{key, collision}` and round-trip
      through `link_reference_playable` and `client_safe_definitions()`.
- [ ] Duplicate or empty state-key vocabularies are rejected fail-closed (unchanged from prior
      behavior, re-verified against the new shape).
- [ ] A `LocalObject` placement without an authored initial state, or with one outside the
      definition's declared vocabulary, is rejected fail-closed.
- [ ] Existing Open/Close fixtures (`apps/game-server/tests/content_reference_playable.rs`,
      `apps/game-server/src/world_runtime.rs` CW4 tests) still pass after the mechanical adaptation.
- [ ] A RETAG-family transition connecting two states of differing collision presence is rejected;
      no `action_id`-shaped field exists anywhere in the transition or state model.

## Excluded scope

- `content/**`, `imports/**`, `tools/**` (no committed LocalObject content exists on `main`).
- protocol, persistence, Foundation, `revert_after`, and `world_runtime.rs` runtime *semantics*
  (only a mechanical, non-semantic adaptation at the existing CW4 bind site is in scope; any
  semantic runtime change is out of scope and returns `SHARED_LEASE_REQUIRED`).
- `apps/game-server/tests/content_world_project_repository.rs` and
  `apps/game-server/examples/materialize_content_world_project_v2.rs` (PR #1036 owns them).
- Wiring per-state collision presence into CW4 runtime *execution* (`prepare()`'s Open/Close
  `next_blocking` derivation) — that is future CW4 runtime work, not this content-model task.

## Implementation / findings

- `reference_playable.rs`: added `LocalObjectCollisionPresence` (`Present|Absent`) and
  `LocalObjectStateDefinition {key, collision}`; `LocalObjectStates` variants hold
  `Vec<LocalObjectStateDefinition>` (was `Vec<ProductionKey>`). `validate_transition` adds the
  RETAG invariant (`LOCAL_OBJECT_RETAG_INTENT_FAMILY`: same collision class required, no
  action-id field). `PlacementRef` gains `local_object_initial_state: Option<ProductionKey>`,
  required+in-vocabulary iff `LocalObject`, forbidden otherwise (fail-closed both directions).
- `project.rs`: authored `LocalObject.states` mirrors the new per-state shape.
- `world_runtime.rs` (not owned; smallest mechanical adaptation, reported per prompt): CW4
  `bind()` reads the authored initial state instead of deriving it from OPEN; fixture updated to
  match, preserving prior observable behavior. `prepare()` untouched.

### Repair round 1 (Codex, head 969e9867): both P2 fixed

- world_runtime.rs: `bind()` set `blocking_cells` from the full footprint unconditionally even
  for an Absent/open initial state. Fixed: derive it from that state's own
  `LocalObjectCollisionPresence` (`Present` → footprint, `Absent` → empty), defensive
  `InvalidBinding` if absent from vocabulary. `prepare()` untouched. New test:
  `authored_open_initial_state_starts_unblocked_then_close_commits_and_blocks`.
- project.rs: the `Vec<String>`→`Vec<LocalObjectStateDocument>` change broke decoding any
  existing `v1` `LocalObject` record. Per coordinator decision (no v2 bump): new
  `LocalObjectStateEntryDocument` (`#[serde(untagged)]`, `Legacy(String)|Typed(...)`); `Legacy`
  decodes but `.lower()` fails closed (never defaults collision); writer only emits `Typed`.
  4 new tests (legacy decode, legacy-lower rejection, typed round-trip, full legacy record).
  `tests/content_world_project.rs` (not owned) updated its one fixture to match.

### Repair round 2 (Codex, head 3934894c, FINAL round): P2 fixed

- world_runtime.rs ~720: `bind()` now derives `blocking_cells` from the initial state, but
  `prepare()` still hard-codes Open→empty/Close→full, and nothing constrained a definition whose
  OPEN target is Present or CLOSE target is Absent — accepted but behaviorally inconsistent.
  Coordinator decision (kept mechanical; `prepare()` untouched): `bind()` now rejects such a pair
  with `InvalidBinding("OPEN/CLOSE transition collision classes do not match runtime
  operations")` unless OPEN source=Present/target=Absent (CLOSE is the same pair reversed, per
  the existing two-state-inverse check). Placed last among the OPEN/CLOSE structural checks so
  an already-invalid pair (wrong definition/intent family/swapped keys) still surfaces its own
  specific error first — this reordering was needed to keep
  `swapped_open_close_intents_fail_closed_before_runtime_creation` passing. New test:
  `mismatched_open_close_collision_classes_reject_before_runtime_creation` (swaps the fixture
  states' collision presence, asserts this exact error). Added a small shared helper,
  `local_object_state_collision`, reused by both this check and the round-1 initial-collision
  lookup.

## Validation

Commands (all cargo under
`CARGO_TARGET_DIR=/home/user/.cargo-shared-target flock /home/user/.cargo-build.lock`):
`cargo fmt --check`; `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`;
`cargo test -p oteryn-game-server --test content_reference_playable --test content_world_project`;
`cargo test -p oteryn-game-server --lib content::project`;
`cargo test -p oteryn-game-server world_runtime`; `python3 tools/agents/validate_governance.py`;
`python3 tools/repository/validate_repository_policy.py`.

Latest (repair round 2) result: all PASS. `content_reference_playable` 38/38,
`content_world_project` 22/22 (both cross-checked against `grep -c '^#\[test\]'`);
`content::project` lib unittests 17/17 (includes the 4 round-1 legacy-compat tests);
`world_runtime` unittests 20/20 (19 prior + 1 new this round). fmt/clippy clean, zero warnings.
Both governance/policy validators clean.

### E2E

- scenario: NOT_APPLICABLE — content-linking model change only, no runtime/E2E path in scope.

### Exact-head CI

- final head: 975f077eec3ec8ad5f9d2f5333709f6cbe7c7a03 (see Terminal integration)
- trigger source: Merge Queue (`merge_group`)
- workflow/run/job: Merge gate / `game-gate` aggregate
- runner assignment: complete
- classification: terminal
- result: PASS (Merge Queue admitted and merged the candidate)

## Self-review

- exact head: 975f077eec3ec8ad5f9d2f5333709f6cbe7c7a03
- method/reviewer: implementing/coordinating agent (mandatory; cannot be delegated away)
- material findings: none found after repair round 2. Both rounds' Codex P2 findings were
  accepted and repaired (see Implementation/findings above); `prepare()`'s Open/Close semantics
  remain intentionally untouched per the coordinator's own decision each round. One reordering
  side-effect was caught and fixed by this agent before pushing: placing the new round-2 check
  before the intent-family check would have changed the error message an existing test
  (`swapped_open_close_intents_fail_closed_before_runtime_creation`) asserts — moved the new
  check to last among the OPEN/CLOSE structural invariants instead.
- verdict: PASS; round 2 of 2 was FINAL per coordinator, merged clean

## Independent review

- required: YES — high-risk under `apps/game-server/AGENTS.md`? No (content-model-only, no
  protocol/session/admission/persistence/identifier/fencing/multichannel change); ordinary PR
  review via repository gates/Merge Queue is sufficient.
- exact head: 975f077eec3ec8ad5f9d2f5333709f6cbe7c7a03 (via Codex review, 2 repair rounds; see
  Terminal integration)
- method/auditor: Codex review, 2 repair rounds (see Implementation/findings above)
- material findings: both rounds' P2s accepted and repaired; the final remaining P2 (legacy
  bare-string v1 `LocalObject` rejected at parse) was dispositioned by owner decision as a
  deliberate explicit rejection, not a defect (no `LocalObject` content records exist on `main`)
- verdict: PASS; Merge Queue admitted the frozen final head

## PR and closeout

- changed-file review: complete (see Terminal integration)
- unresolved review threads: none
- related/superseded PRs: none known
- protected auto-merge: Merge Queue
- merge commit/result: `832320a` on protected `main`
- ownership release: complete; owned paths released at archive

## Context checkpoint

```yaml
last_progress: terminal integration recorded; PR #1046 merged via Merge Queue as 832320a; record archived
status: completed
branch: claude/cw3-local-object-state-model
head_sha: 975f077eec3ec8ad5f9d2f5333709f6cbe7c7a03
pr: 1046
final_head_sha: 975f077eec3ec8ad5f9d2f5333709f6cbe7c7a03
final_head_frozen_at: 2026-09-27T23:22:24Z
ci_trigger_source: merge_group
ci_check_generation: final
ci_checks_for_current_head: 1
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: complete
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none; task closed
```

## Terminal integration

This section supersedes the historical `implementing`/pending metadata and
checkpoint above with the frozen terminal outcome; the complete implementation
record above (including both Codex repair rounds) remains verbatim as historical
evidence. This closeout performs no code, schema or content mutation of its own; it
only moves this record from `docs/agents/tasks/active/` to
`docs/agents/tasks/archive/` and binds terminal lifecycle fields. Coordination:
issue #162 comment "terminal integration" (control plane).

Candidate head `975f077eec3ec8ad5f9d2f5333709f6cbe7c7a03` was frozen at
2026-09-27T23:22:24Z (issue #162 FREEZE_SHA comment 5860784616). PR #1046 merged
via Merge Queue as commit `832320a` on protected `main` at 2026-09-28T00:39Z. Codex
review ran 2 repair rounds (see Implementation/findings above for both); the final
remaining P2 (legacy bare-string v1 `LocalObject` rejected at parse) was
dispositioned by owner decision as a deliberate explicit rejection, since no
`LocalObject` content records exist on `main` to be broken by it. Protected-main
readback: `reference_playable.rs`, `world_runtime.rs`, `project.rs`, and this task
record's blobs on `main` are byte-identical to the frozen head
`975f077eec3ec8ad5f9d2f5333709f6cbe7c7a03`.

Task status: `completed`. Aggregate issue #162 remains open for later Content/World
work.
