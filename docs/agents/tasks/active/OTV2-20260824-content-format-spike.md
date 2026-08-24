# OTV2-20260824-content-format-spike

```yaml
task_id: OTV2-20260824-content-format-spike
title: Compare native World Bundle representations
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: spike/content-format-representations-20260824
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260824-content-format-spike
created_at: 2026-08-24T19:31:00+02:00
updated_at: 2026-08-24T19:31:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - tools/content-format-spike/**
  - docs/agents/evidence/OTV2-CONTENT-FORMAT-SPIKE.md
  - docs/agents/evidence/OTV2-CONTENT-FORMAT-SPIKE-results.json  - docs/agents/tasks/active/OTV2-20260824-content-format-spike.md
public_contracts:
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/migration/CRYSTAL_WORLD_CONTENT_MIGRATION_DESIGN_CHECKPOINT.md
depends_on:
  - Oteryn-Game#95
  - Oteryn-Game#87
  - Oteryn-Game#89
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Produce a reproducible evidence dossier comparing three bounded physical representations for a native World Bundle without selecting a permanent production format.

## Architecture and source of truth

- `PROVEN`: ADR-0005 requires deterministic, indexed, bounded, corruption-detecting runtime content and source-control-friendly authoring.
- `PROVEN`: Issue #95 authorizes evidence only and requires coordinator allocation before implementation writes.
- `PROVEN`: the migration checkpoint is non-authoritative evidence and must not silently define production semantics.
- `DERIVED`: a standalone standard-library spike can compare physical representations without changing runtime, Cargo/workspace or production CONTENT.
- `UNKNOWN`: final World Project/Bundle representation remains owner-gated.

## Acceptance criteria

- [ ] Compare exactly three concrete representations on one deterministic synthetic fixture.
- [ ] Record serialized size and repeatable full-load/partial-load timings.
- [ ] Exercise deterministic re-emission and corruption detection/recovery behavior.- [ ] Record a trade-off matrix covering all Issue #95 comparison axes.
- [ ] Preserve provenance fields and composite/multi-tile semantic identity in every candidate.
- [ ] Label any recommendation as evidence-only and non-canonical.
- [ ] Do not modify production CONTENT, Cargo/workspace, workflows, registries or architecture contracts.

## Excluded scope

No permanent format selection, production activation, broad content import, Reference-parity claim, renderer-policy encoding, live/protected data, external repository mutation or new dependency.

## Implementation / findings

Waiting for this coordinator allocation to merge. After merge, create `spike/content-format-representations-20260824` from the exact merged `main` and write only the owned paths above.

## Validation

### Focused

- command/run: `python -m unittest discover -s tools/content-format-spike/tests -p "test_*.py"`
- result: pending

### Component/integration

- command/run: `python tools/content-format-spike/run_spike.py --output docs/agents/evidence/OTV2-CONTENT-FORMAT-SPIKE-results.json`
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — evidence-only offline representation spike
- result: `NOT_APPLICABLE`

### Exact-head CI
- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent, whole diff
- material findings: pending
- verdict: pending

## Independent review

- required: NO — standalone evidence tooling only; no production/runtime/contract authority
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: Issue #95
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending
## Context checkpoint

```yaml
last_progress: coordinator allocation candidate prepared for Issue #95
status: waiting
branch: spike/content-format-representations-20260824
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
blocker: allocation not effective until allocation PR merges
next_action: merge allocation, then implement the evidence-only spike on the exact merged main
```
