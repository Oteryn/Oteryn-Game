# OTV2-20261006-quest-completion-all373

```yaml
task_id: OTV2-20261006-quest-completion-all373
title: Build and close the all-373 quest completion backlog
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/quest-completion-all373-20261006
pr: null
base_sha: e953f1ef67ed5a9c66cf8ce6cac37da66f999dcb
head_sha: cce9ac5a48d26677b50f01cbdc73b7d627ee8140
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-quest-completion
created_at: 2026-10-06T08:55:00+02:00
updated_at: 2026-10-06T12:20:00+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/quest-authoring/quest_completion_matrix.py
  - tools/content-schema/quest-authoring/test_quest_completion_matrix.py
  - tools/content-schema/quest-authoring/run_checks.py
  - tools/content-schema/quest-authoring/quest_completion_import.py
  - tools/content-schema/quest-authoring/ots_questlog.py
  - tools/content-schema/quest-authoring/samples/completion-matrix/
  - tools/content-schema/quest-authoring/samples/server-completion/chosen-source-progress/
  - tools/content-schema/quest-authoring/test_chosen_source_progress.py
  - tools/content-schema/quest-authoring/test_source_path_normalization.py
  - content/quests/missions/quest-state-completion-candidate.json
  - content/quests/missions/completion-candidate.json
  - docs/agents/tasks/OTV2-20261006-quest-completion-all373.md
public_contracts: []
depends_on:
  - docs/agents/evidence/OTV2-20261006-crystal-summer-quest-audit.md
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - opentibiabr/canary
  - zimbadev/crystalserver
```

## Outcome

Create a deterministic work queue for every one of the 373 pinned wiki quest titles, mapping each title to the existing canonical Oteryn Quest definition(s), donor coverage, current source/data readiness and native/runtime admission state. Use that queue to close quest gaps in bounded follow-up batches without duplicating canonical identities.

## Architecture and source of truth

- **PROVEN**: `tools/content-schema/quest-authoring/samples/quest-coverage-2026-09-27.json` contains the pinned 373-title wiki inventory and Canary/Crystal coverage classifications.
- **PROVEN**: `content/quests/definitions/index.json` currently indexes 352 canonical Quest definitions.
- **PROVEN**: `tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json` is evidence-only and does not establish executable/runtime completeness.
- **DERIVED**: 373 wiki titles can be mapped to current canonical definitions; multiple wiki titles may intentionally share a canonical family definition.
- **UNKNOWN**: end-to-end playability for the catalogue; no quest is promoted to playable by this task's matrix alone.
- Existing quest authoring architecture and source-fidelity holds remain authoritative.

## High-risk authority/recovery qualification

```yaml
applicable: false
model: NOT_APPLICABLE
authority_invariants: []
consumer_boundaries: []
mutation_operators:
  applicable: []
  considered_not_applicable:
    - This task creates offline quest audit/work-queue artifacts and does not perform production mutation or authority-bearing recovery.
one_invariant_per_negative_case: not_applicable
independent_current_fact_sources: []
record_derived_matching_helper:
  allowed_for_positive_happy_path: not_applicable
  forbidden_for_negative_authority_or_provenance_cases: not_applicable
finding_family_sweep:
  sibling_apis: not_applicable
  protocol_versions: not_applicable
  direct_and_reconciled_paths: not_applicable
  fenced_durable_writes: not_applicable
  restart_retry_replay_concurrency_pg_reload: not_applicable
  evidence: []
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] Every pinned wiki title is represented in the generated matrix.
- [x] Canonical mapping is derived from committed Oteryn definitions/catalogue evidence; no new identity is invented.
- [x] Donor coverage distinguishes implemented, partial and absent-in-both reference cases.
- [x] Work state distinguishes source fidelity, typed-progress/native work and runtime verification.
- [x] The matrix never claims playability without dedicated runtime evidence.
- [x] Drift checking is part of the existing offline quest authoring checks.
- [ ] Focused tests and the applicable quest/governance checks pass on the candidate.

## Excluded scope

- No claim that donor implementation equals Global Tibia parity.
- No automatic runtime activation.
- No bulk replacement of canonical Quest identities.
- No direct Lua-to-runtime copy.
- No quest is marked playable solely because its definition or source crosswalk exists.
- Runtime repairs are split into bounded follow-up batches under their own owned paths.

## Implementation / findings

Initial read-only audit retained at `docs/agents/evidence/OTV2-20261006-crystal-summer-quest-audit.md`.

The first implementation slice builds the machine-readable 373-title matrix and wires byte-for-byte drift checking into `run_checks.py`.

The second slice separates source-fidelity holds from implementation state and expands the existing unactivated completion candidate. Of 146 chosen source-derived recipes absent from production Source QuestState lowering, 139 satisfy the accepted closed linear stage-counter shape. They are added as `CHOSEN_SOURCE_TYPED_PROGRESS_ONLY`; seven remain explicit holds because their terminal completion stage has count greater than one.

Candidate result before source-state completion overlay: **303 quest owners, 2445 tracks, 4378 transitions**. The added chosen-source slice is **139 owners / 699 tracks / 699 transitions**. `runtime_activated=false`; native event, NPC and reward bindings remain zero.

A third committed slice adds `chosen-source-events-rewards` for all 139 chosen-source owners using the same pinned resolver epoch and qualified-world snapshot as authored68. Result: **699 stages, 283 exact stage target refs, 300 reward intents, 232 exact reward refs, 8 existing encounter outcome seams**; runtime admission remains false. `completion-binding-plan.json` is expanded from authored68 to **207 quest records / 1115 stage records**. Source139 talk stages intentionally retain `NPC_dialogue_candidates=null`, and all native dispatch/reward bindings remain null.

The current committed matrix derives implementation state directly from the typed-progress candidate. It reports **214 `NATIVE_BINDINGS_PENDING`**, **117 `NATIVE_LOWERING_PENDING`**, **41 `DEFINITION_READY_RUNTIME_UNKNOWN`**, and **1 `MAPPING_REVIEW`**. Source fidelity remains an independent axis: 222 titles retain source holds and 151 are clear.

A further local-only completion-overlay qualification has been proven but is not yet committed: among the 90 source QuestState owners whose completion is `NOT_LOWERED_MULTI_TRACK` or `NOT_LOWERED_NO_MISSIONS`, **88** satisfy the same closed chosen-stage counter contract (**707 tracks / 707 transitions**). Only `Barbarian Arena Quest` and `The Ancient Tombs Quest` remain held because their terminal completion stage has count greater than one. Applying that overlay locally preserves all donor tracks and yields 96 source owners with completion states: 88 `SOURCE_PLUS_CHOSEN_TYPED_COMPLETION`, 6 `LOWERED`, 1 multi-track hold and 1 no-missions hold.

## Validation

### Focused

- committed slice: `test_quest_completion_matrix.py`, `test_chosen_source_progress.py`, `test_source_path_normalization.py`, and `test_chosen_source_binding_plan.py`: **14 focused tests PASS**
- source139 association packet: **5 packet tests PASS**
- `python quest_completion_matrix.py --check`: **PASS**
- `python quest_completion_import.py --check`: **PASS**
- committed candidate integrity: **303/303 unique quest owners, 2445 tracks, 4378 transitions**
- committed binding plan: **207 quests / 1115 stages**, runtime disabled, native dispatch bindings 0, native reward delivery bindings 0
- local overlay candidate: **18 focused tests PASS** before final publication; candidate becomes **303 owners / 3152 tracks / 5085 transitions**, with 88 source-state completion overlays and 2 explicit overlay holds
- production `quest-state.json`: unchanged

### Component/integration

- `python tools/content-schema/quest-authoring/run_checks.py`: quest-specific checks pass with `PYTHONUTF8=1`; the full local suite is environment-blocked later by an existing map test attempting to invoke missing `g++` (`WinError 2`). No quest regression was observed before that external-tool failure.

### E2E

- scenario: NOT_APPLICABLE for the matrix slice; quest runtime E2E belongs to subsequent completion batches.
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none identified
- protected auto-merge: not requested
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: 303-owner candidate committed; source139 event/reward associations committed; binding plan expanded to 207 quests/1115 stages; 88-owner source completion overlay qualified locally
status: implementing
branch: codex/quest-completion-all373-20261006
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
next_action: publish the validated 88-owner source completion overlay, refresh the all-373 matrix, then continue native binding batches
```
