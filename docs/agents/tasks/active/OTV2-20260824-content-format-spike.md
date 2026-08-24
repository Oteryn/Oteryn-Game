# OTV2-20260824-content-format-spike

```yaml
task_id: OTV2-20260824-content-format-spike
title: Produce native World Project / World Bundle format evidence
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 95
allocation_pr: 112
base_branch: main
branch: spike/content-format-evidence-20260824
pr: null
base_sha: 9f769a77c3c6067f389906f0b62372f2d30684c2
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260824-content-format-spike
created_at: 2026-08-24T19:40:43+02:00
updated_at: 2026-08-24T21:05:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - tools/content-format-spike/**
  - docs/agents/evidence/OTV2-20260824-content-format-spike.md
  - docs/agents/evidence/OTV2-20260824-content-format-spike-results.json
  - docs/agents/tasks/active/OTV2-20260824-content-format-spike.md
public_contracts: []
depends_on:
  - Oteryn-Game#95
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Produce a reproducible evidence package comparing bounded physical representations for editable World Project and compiled World Bundle concerns. The result is advisory evidence only and must preserve `SPIKE_RESULT != OWNER_FORMAT_DECISION`.

## Architecture and source of truth

- `PROVEN`: `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md` requires separate editable source and compiled runtime representations; OTBM remains an import input, not canonical Oteryn format.
- `PROVEN`: `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` and accepted `VSL-CONTENT-01` define bounded, deterministic content/runtime requirements.
- `PROVEN`: merged CONTENT evidence seam provides a deterministic non-production artifact parser/compiler and separate server-authoritative/client-safe projections.
- `PROVEN`: `docs/migration/CRYSTAL_WORLD_CONTENT_MIGRATION_DESIGN_CHECKPOINT.md` is non-authoritative migration evidence and may inform representative scale/shape only.
- `UNKNOWN`: permanent World Project / World Bundle physical encodings remain owner-gated.

## Acceptance criteria

- [ ] Candidate set is small, explicit, and independently isolated from production runtime interfaces.
- [ ] Deterministic synthetic fixtures cover multiple bounded scales and have recorded hashes.
- [ ] Reproducible measurements cover serialization/build, load/access, size/compression, diff locality, patch granularity, and bounded failure behavior.
- [ ] Malformed/truncated/oversize/ratio-limit negative tests execute fail-closed.
- [ ] Exact Python/SQLite/zlib tool versions and benchmark configuration are recorded.
- [ ] Decision dossier distinguishes editable-project versus compiled-runtime concerns and records tradeoffs without selecting the permanent format.
- [ ] Governance, focused tests, `git diff --check`, whole-diff self-review, and exact-head repository CI are clean before merge.

## Excluded scope

No production loader adoption, permanent `.omap`/`.owb` decision, ADR/contract/registry mutation, Cargo/workspace changes, protected deployment, proprietary asset inclusion, Platform/external-repository writes, or production activation authority.

## Implementation / findings

Allocation PR #112 is pending on current main. Worker writes become authorized only after PR #112 merges; the archived historical coordinator task is not revived. The intended prototype implementation uses standard-library Python/SQLite/zlib so the spike does not add repository dependencies.

## Validation

### Focused

- command/run: pending allocation merge
- result: pending

### Component/integration

- command/run: `NOT_APPLICABLE` until spike tooling exists; no production runtime integration is authorized
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — evidence tooling only
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
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: `NO` unless the spike later introduces a material parser/download/signing trust boundary; current allocation is stdlib evidence tooling only
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: coordinator prepared a bounded evidence-only allocation candidate from main@9f769a77c3c6067f389906f0b62372f2d30684c2
status: waiting
branch: spike/content-format-evidence-20260824
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
blocker: write authority activates only after the coordinator allocation PR merges
next_action: merge PR #112, then create the worker branch from that exact merge and execute the bounded spike
```
