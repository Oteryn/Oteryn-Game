# OTV2-20260824-next-wave-parallel-worker-prompts

```yaml
task_id: OTV2-20260824-next-wave-parallel-worker-prompts
title: Prepare next-wave parallel worker prompts
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/next-wave-parallel-workers-20260824
pr: 107
base_sha: cfb24c95f24ff5067d446a1f9d6ff92db53eeedb
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT coordination session
created_at: 2026-08-24T17:18:43Z
updated_at: 2026-08-24T17:22:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/prompts/OTV2_NEXT_WAVE_PARALLEL_PREPARATION.md
  - docs/agents/prompts/OTV2_PREP_WAVE2_RESOURCE_LIMITS.md
  - docs/agents/prompts/OTV2_PREP_DURABILITY_TOPOLOGY.md
  - docs/agents/prompts/OTV2_PREP_SERVER_SEAM.md
  - docs/agents/prompts/OTV2_PREP_PROGRAMME_STATUS.md
  - docs/agents/prompts/README.md
  - docs/agents/tasks/active/OTV2-20260824-next-wave-parallel-worker-prompts.md
public_contracts: []
depends_on:
  - PR #103 merged as a431ec9390759e28c6cb543b8228e4882ee07652
  - PR #105 merged as cfb24c95f24ff5067d446a1f9d6ff92db53eeedb
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Add reusable preparation-worker prompts and one launch matrix so Issues #93, #94, #95, #96 and #97 can be assigned to isolated agents in parallel while preserving the master-plan readiness and authority gates.

## Architecture and source of truth

- `PROVEN`: current base is `main@cfb24c95f24ff5067d446a1f9d6ff92db53eeedb`.
- `PROVEN`: PR #103 already hardened Server Seam handoff and staged Movement hard-max closure.
- `PROVEN`: preparation Issues #93/#94/#95/#96/#97 are separate bounded decision/evidence/status domains.
- `DERIVED`: four reusable prep prompts were missing; #95 already has `OTV2_CONTENT_FORMAT_SPIKE.md`.
- `PROVEN`: prompts do not grant write authority; live Issue/coordinator allocation and GitHub state remain authoritative.

## Acceptance criteria

- [x] Four preparation prompts exist for #93, #94, #96 and #97.
- [x] One launch prompt/matrix defines all five preparation lanes and safe parallelism.
- [x] Existing #95 prompt is reused rather than duplicated.
- [x] README indexes all new aliases.
- [x] No runtime, contract, registry, Cargo/workspace, workflow or production authority changes.
- [ ] Governance, diff check, placeholder scan and exact-head repository gates pass.

## Excluded scope

No implementation allocation, numeric limit decision, database topology decision, server-seam runtime code, programme-status mutation, permanent Content format decision, production access, Platform mutation or external-repository write is performed by this task.

## Implementation / findings

PR #103 already closed the previously identified execution-clarity gaps. This task only prepares reusable isolated worker contracts for the lawful preparation wave. PR #107 contains the launcher, four missing prep prompts, README alias index update and this task record.

## Validation

### Focused

- command/run: pending exact-head diff/governance/placeholder review
- result: pending

### Component/integration

- command/run: `NOT_APPLICABLE` — docs/prompt-only delivery
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no product/runtime mutation
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request #107
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

- required: NO — bounded docs/prompt packaging does not widen runtime/product/write authority
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: PR #103
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: PR #107 opened with seven intended docs/prompt paths
status: validating
branch: docs/next-wave-parallel-workers-20260824
head_sha: null
pr: 107
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
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
next_action: perform exact-head whole-diff review and repository gate validation
```
