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
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-quest-completion
created_at: 2026-10-06T08:55:00+02:00
updated_at: 2026-10-06T08:55:00+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/quest-authoring/quest_completion_matrix.py
  - tools/content-schema/quest-authoring/test_quest_completion_matrix.py
  - tools/content-schema/quest-authoring/run_checks.py
  - tools/content-schema/quest-authoring/samples/completion-matrix/
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

- [ ] Every pinned wiki title is represented in the generated matrix.
- [ ] Canonical mapping is derived from committed Oteryn definitions/catalogue evidence; no new identity is invented.
- [ ] Donor coverage distinguishes implemented, partial and absent-in-both reference cases.
- [ ] Work state distinguishes source/data work from native-binding work and runtime verification.
- [ ] The matrix never claims playability without dedicated runtime evidence.
- [ ] Drift checking is part of the existing offline quest authoring checks.
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

The first implementation slice builds the machine-readable 373-title matrix and wires byte-for-byte drift checking into `run_checks.py`. Runtime/source repairs follow from this queue in reviewable batches.

## Validation

### Focused

- `python -m unittest test_quest_completion_matrix.py`: pending publication readback
- `python quest_completion_matrix.py --check`: pending publication readback

### Component/integration

- `python tools/content-schema/quest-authoring/run_checks.py`: running locally; Windows default-codepage false failure is separated by rerun with `PYTHONUTF8=1`.

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
last_progress: all-373 matrix generator prototyped locally; 4 focused tests pass
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
next_action: publish and validate the deterministic all-373 completion matrix
```
