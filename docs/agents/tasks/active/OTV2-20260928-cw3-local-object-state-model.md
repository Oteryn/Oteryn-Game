# OTV2-20260928-cw3-local-object-state-model

```yaml
task_id: OTV2-20260928-cw3-local-object-state-model
title: LocalObject per-state collision presence, authored initial state, RETAG intent family
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cw3-local-object-state-model
issue: 162
pr: null
base_sha: dd209a1264e98f3d1f0f167ec3320124a071db53
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world build (session_01LphUANMfC2q2WKdfEb39eC)
created_at: 2026-09-27T22:30:02Z
updated_at: 2026-09-27T22:30:02Z
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

- Added `LocalObjectCollisionPresence` (`Present | Absent`) and `LocalObjectStateDefinition`
  (`{key: ProductionKey, collision: LocalObjectCollisionPresence}`) to `reference_playable.rs`;
  `ReferenceDefinitionKind::LocalObjectStates` and `ClientSafeDefinitionKind::LocalObjectStates`
  now hold `Vec<LocalObjectStateDefinition>` (was `Vec<ProductionKey>`).
- `validate_definition_shape` duplicate/empty checks now operate on `.key` (state identity),
  independent of collision presence.
- `validate_transition` resolves state membership through `.key` and adds a RETAG invariant: when
  `normalized_intent_family` equals the new `LOCAL_OBJECT_RETAG_INTENT_FAMILY` constant
  (`oteryn:reference.intent.local-object-retag`), `source_state` and `target_state` must resolve to
  the same `LocalObjectCollisionPresence`. No action-id field was added anywhere (D38 1c).
- `PlacementRef` gains `local_object_initial_state: Option<ProductionKey>`. `validate_placement`
  requires it `Some` and in-vocabulary exactly when the placement's definition is `LocalObject`,
  and requires it absent otherwise (fail-closed both directions).
- `project.rs`: authored `ProjectReferenceRecord::LocalObject.states` changed from `Vec<String>` to
  `Vec<LocalObjectStateDocument>` (`{key: String, collision: LocalObjectCollisionDocument}`);
  `.lower()` maps to the new typed state list.
- `world_runtime.rs` mechanical adaptation (not owned; smallest possible, no semantic change) —
  see PR description "world_runtime.rs mechanical adaptation" section for the exact diff:
  - CW4 `bind()` reads `placement.local_object_initial_state` for the runtime's initial `state`
    instead of deriving it from `open_transition.source_state`; `states.contains(&key)` calls
    adapted to `states.iter().any(|s| s.key == ...)` for the new element type.
  - The CW4 test-fixture builder's `LocalObjectStates(vec![closed, open])` literal became
    `vec![LocalObjectStateDefinition{key: closed, collision: Present}, ...{key: open, collision:
    Absent}]`, and its placement closure now sets `local_object_initial_state: Some(closed)` for
    both fixture placements — preserving the exact prior runtime behavior (both instances start
    Closed) with no observable behavior change.
  - `prepare()`'s Open/Close `next_blocking` derivation (operation-keyed, not state-keyed) was left
    untouched: wiring per-state collision presence into that path is runtime semantics excluded
    from this task.

## Validation

### Focused

- command/run: `cargo fmt --check`; `cargo clippy -p oteryn-game-server --all-targets -- -D warnings` (both under `CARGO_TARGET_DIR=/home/user/.cargo-shared-target flock /home/user/.cargo-build.lock`)
- result: PASS (fmt clean after one `cargo fmt` pass on the mechanical `world_runtime.rs` edit; clippy exit 0, no warnings)

### Component/integration

- command/run: `CARGO_TARGET_DIR=/home/user/.cargo-shared-target flock /home/user/.cargo-build.lock cargo test -p oteryn-game-server --test content_reference_playable`
- result: PASS — 38 passed; 0 failed (7 new tests for 1a/1b/1c; all prior Open/Close and evidence-ordering fixtures unchanged)
- command/run: `CARGO_TARGET_DIR=/home/user/.cargo-shared-target flock /home/user/.cargo-build.lock cargo test -p oteryn-game-server world_runtime`
- result: PASS — 18 passed; 0 failed (all CW4 first-child Open/Close tests, including `open_close_then_replay_open_preserves_current_closed_state` and `occupied_multicell_close_is_atomic_and_open_removes_only_own_contribution`, pass unchanged after the mechanical bind-site adaptation)
- command/run: `python3 tools/agents/validate_governance.py`
- result: PASS
- command/run: `python3 tools/repository/validate_repository_policy.py`
- result: PASS

### E2E

- scenario: NOT_APPLICABLE — content-linking model change only, no runtime/E2E path in scope.

### Exact-head CI

- final head: pending (frozen at commit time below; exact-head CI is the coordinator's/Merge Queue's read after PR open)
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: bound at commit (see PR)
- method/reviewer: implementing/coordinating agent (mandatory; cannot be delegated away)
- material findings: none found. Noted and deliberately scoped out: (1) `prepare()`'s Open/Close
  `next_blocking` derivation stays operation-keyed, not state-collision-keyed — wiring per-state
  collision into CW4 execution is future runtime work, not this content-model task; (2) one
  transitively-broken non-owned test file (`tests/content_world_project.rs`, distinct from the
  PR #1036-owned `tests/content_world_project_repository.rs`) needed the same mechanical
  authored-form update as `project.rs`'s own `LocalObject` variant — applied and reported per the
  same disjointness principle the prompt states for `world_runtime.rs`.
- verdict: no blocking findings; ready for PR

## Independent review

- required: YES — high-risk under `apps/game-server/AGENTS.md`? No (content-model-only, no
  protocol/session/admission/persistence/identifier/fencing/multichannel change); ordinary PR
  review via repository gates/Merge Queue is sufficient.
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: pending
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: task record created, branch cut from origin/main at dd209a1
status: implementing
branch: claude/cw3-local-object-state-model
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: implement 1a/1b/1c in reference_playable.rs, project.rs, and tests
```
