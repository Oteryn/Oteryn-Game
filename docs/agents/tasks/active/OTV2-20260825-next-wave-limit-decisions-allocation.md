# OTV2-20260825-next-wave-limit-decisions-allocation

```yaml
task_id: OTV2-20260825-next-wave-limit-decisions-allocation
title: Allocate Issue #133 next-wave limit decision worker
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/allocate-next-wave-limit-decisions-133
issue: 133
pr: 137
base_sha: 91b73a7566a59991ebf7d471eacb3a858b755c9c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT coordinator for Issue #131
created_at: 2026-08-25T07:18:00+02:00
updated_at: 2026-08-25T07:25:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260825-next-wave-limit-decisions-allocation.md
  - docs/agents/tasks/active/OTV2-20260825-next-wave-limit-decisions.md
public_contracts:
  - docs/agents/prompts/OTV2_CLOSE_NEXT_WAVE_BLOCKERS.md
  - docs/superpowers/plans/2026-08-25-oteryn-close-next-wave-blockers-implementation-plan.md
depends_on:
  - issue:131
  - issue:133
blocks:
  - task:OTV2-20260825-next-wave-limit-decisions
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Merge one docs-only coordinator allocation that authorizes the Issue #133 worker to produce only the reproducible resource-limit harness, evidence JSON, decision baseline and its own task record. Parent coordinator #131 remains active.

## Architecture and source of truth

- `PROVEN`: Issue #131 is open and remains the parent coordinator until all blocker lifecycles close.
- `PROVEN`: Issue #133 is the bounded child decision/evidence task authorized by #131/#128.
- `PROVEN`: current `main@91b73a7566a59991ebf7d471eacb3a858b755c9c` contains merged coordinator allocation PR #132.
- `PROVEN`: this allocation grants no registry, Cargo/workspace, Foundation/product code, listener, gameplay, Durability, production, Platform or external-repository write authority.
- `DERIVED`: the worker branch must be created only after this allocation merges, from that exact merge SHA.

## Acceptance criteria

- [x] Parent task `OTV2-20260825-close-next-wave-blockers` remains active and unarchived.
- [x] Worker authority after merge is limited to the four Issue #133 evidence/decision paths.
- [x] Worker branch is fixed as `arch/next-wave-limit-decisions-133` and will be created from the exact allocation merge SHA.
- [x] Canonical `RESOURCE_LIMITS_REGISTRY.json` remains unallocated to this worker.
- [x] No product behavior, production tuning, gameplay balance, stable ID or Reference-parity decision is introduced.
- [x] Governance and `git diff --check` pass.
- [ ] Whole-diff self-review and exact-head repository checks including `game-gate` pass before merge.

## Excluded scope

No `RESOURCE_LIMITS_REGISTRY.json`, runtime code, Cargo/lockfile/workspace, listener, Durability, Ability, Interaction, AI, Movement, Combat, Client, production configuration, secrets, deployment, Platform or external-repository mutation.

## Implementation / findings

The originally staged branch incorrectly archived the still-active parent #131 task and did not persist a live allocation candidate. Root cause was lifecycle-role confusion: the child allocation was treated as parent closeout. The corrected allocation keeps #131 active, transfers the coordinator-only live-allocation one-writer lease to this allocation task, and follows the established allocation pattern used by PR #118. The parent task edit in this PR is an explicit ownership/checkpoint correction, not a second owner of this allocation task's paths.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: `PASS` - 25 required policy documents and 9 project lanes validated.

### Component/integration

- command/run: `git diff --check`
- result: `PASS` - no whitespace/error-marker defects.

### E2E

- scenario: `NOT_APPLICABLE` - docs-only allocation has no runtime outcome
- result: `NOT_APPLICABLE`

### Exact-head CI
- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: docs-only bounded allocation
- result: pending

## Self-review

- exact head: pending
- method/reviewer: coordinating agent, complete `origin/main...HEAD` diff
- material findings: The pre-existing staged commit prematurely archived #131 and omitted a live allocation candidate. Root cause was lifecycle-role confusion; corrected by retaining #131, using a dedicated #133 allocation task, and transferring the one-writer live-allocation lease without overlap.
- verdict: `PASS`

## Independent review

- required: `NO` - ordinary bounded allocation under already owner-approved #128/#131 coordinator authority; no new security/runtime/persistence or cross-repository authority
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: `PASS` - effective diff versus current main contains only coordinator/live-allocation/task documentation.
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: after allocation merge; worker paths transfer to Issue #133 child task

## Context checkpoint

```yaml
last_progress: PR #137 is open on the corrected allocation tree; final metadata is recorded before freezing the exact head.
status: validating
branch: coord/allocate-next-wave-limit-decisions-133
head_sha: null
pr: 137
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
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze the exact PR #137 head, record self-review evidence, and require exact-head CI including game-gate before merge.
```
