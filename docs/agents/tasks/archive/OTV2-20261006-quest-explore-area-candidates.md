# OTV2-20261006-quest-explore-area-candidates

```yaml
task_id: OTV2-20261006-quest-explore-area-candidates
title: Qualify Quest explore Area candidates
mode: AUDIT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/quest-explore-area-candidates-20261006
pr: 1899
base_sha: f6894e793c162d9d8a43578332f3a6f77d2936a3
head_sha: f135fe045d033c124bd12ae18631bbcd84fbb2ee
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-quest-completion
created_at: 2026-10-06
updated_at: 2026-10-07
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/quest-authoring/quest_explore_area_candidates.py
  - tools/content-schema/quest-authoring/test_quest_explore_area_candidates.py
  - tools/content-schema/quest-authoring/samples/server-completion/explore-area-candidates.json
  - docs/agents/tasks/archive/OTV2-20261006-quest-explore-area-candidates.md
public_contracts: []
depends_on: []
blocks:
  - QUEST-TRIGGER-1
cross_repository_coordination_id: 1622
external_repositories: []
```

## Outcome

Qualified current Quest `explore` stages against canonical Area identities using only exact Area names and the final canonical key segment.

Current-main refresh after terminal-stage lowering and the Area-tail review repair:

- canonical Area records: **970**
- explore stages: **248**
- `EXACT_SINGLE_AREA_ONLY_TARGET`: **10**
- `EXACT_SINGLE_AREA_WITH_OTHER_TARGETS`: **10**
- `AMBIGUOUS_MULTIPLE_AREAS`: **36**
- `NO_EXACT_AREA`: **192**
- exact Area identity candidates: **20**
- clean one-target candidates: **10**
- native spatial bindings: **0**
- runtime admission: **0**

## Architecture and source of truth

**PROVEN:** aliases are limited to canonical Area `name` and the final dotted segment of the canonical Area key.

**PROVEN:** no fuzzy matching, coordinate inference, containment inference or family preference is performed.

**PROVEN:** Area identity does not prove the spatial occurrence required by a Quest stage. Runtime admission remains owned by the spatial / Quest-trigger runtime.

The Codex review found that the previous key-tail extraction kept `city.liberty_bay` instead of `liberty_bay`. The implementation now uses the actual final dotted key segment. This correctly exposes, for example:

- Hive Outpost: Vandura Hive Outpost + Liberty Bay City -> ambiguity, not a single Area;
- Grave Danger: Darashia City is now retained among the exact Area matches.

## High-risk authority/recovery qualification

```yaml
applicable: false
model: NOT_APPLICABLE
authority_invariants: []
consumer_boundaries: []
mutation_operators:
  applicable: []
  considered_not_applicable:
    - Offline Area identity qualification only; no production mutation or authority-bearing recovery.
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
  p2_fixed_accepted_or_deferred:
    - P2 final-key-segment review finding fixed and regression-tested.
```

## Acceptance criteria

- [x] Every current explore stage is classified deterministically.
- [x] Canonical Area key alias uses only the final dotted key segment.
- [x] Ambiguity is preserved when more than one canonical Area matches.
- [x] Mixed Area + non-Area targets remain held.
- [x] No spatial/runtime binding is promoted.
- [x] Task is archived before final freeze.

## Excluded scope

- Spatial containment inference.
- Coordinate inference.
- Runtime area-enter/leave wiring.
- QuestTransitionRequest cause binding.
- Choosing among ambiguous Area families.
- Treating one exact Area as sufficient when other stage targets are unresolved.

## Validation

### Focused

- canonical Area inventory: **970**
- explore stages: **248**
- exact Area candidates: **20**
- clean one-target candidates: **10**
- dedicated unittest: **8/8 PASS**
- generator `--check`: PASS
- committed packet byte-for-byte drift test: PASS
- `git diff --check`: PASS
- `python -m unittest discover -s tools/agents/tests`: **59/59 PASS**
- `python tools/agents/validate_governance.py`: PASS (re-run after recording this validation evidence)

### Component/integration

- runtime integration: NOT_APPLICABLE; this task only qualifies Area identities.

### E2E

- NOT_APPLICABLE; no executable spatial binding is admitted.

### Exact-head CI

- final exact head is recorded by PR #1899 after closeout; this task file intentionally does not create a self-referential SHA update.

## Self-review

- method/reviewer: implementing agent
- material findings: all exact candidates remain fail-closed; no Area family preference is introduced
- verdict: PASS

## Independent review

- required: YES
- method/auditor: Codex PR review
- material finding: P2 canonical key-tail extraction retained a category prefix instead of the final key segment
- disposition: fixed; Liberty Bay and Darashia regressions covered
- verdict: pending exact-head rereview / thread resolution

## PR and closeout

- changed-file review: generator, focused test, generated candidate packet and archived task record only
- unresolved review threads: expected 0 after exact-head thread resolution
- related/superseded PRs: none
- protected auto-merge: pending exact-head checks
- merge commit/result: pending
- ownership release: yes after merge

## Context checkpoint

```yaml
last_progress: current-main population refreshed to 248 and Area key-tail review defect repaired
status: completed
branch: codex/quest-explore-area-candidates-20261006
head_sha: f135fe045d033c124bd12ae18631bbcd84fbb2ee
pr: 1899
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending exact-head rerun
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: exact-head governance validation and protected merge queue
```
