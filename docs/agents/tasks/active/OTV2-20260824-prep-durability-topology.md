# OTV2-20260824-prep-durability-topology

```yaml
task_id: OTV2-20260824-prep-durability-topology
title: Prepare first Durability implementation topology
mode: CONTRACT
execution_mode: PREPARATION
status: allocation_pending_merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-prep-durability-topology-94
issue: 94
allocation_pr: pending
pr: null
allocation_base_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
worker_base_sha: pending_allocation_merge
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: OTV2-PREP-DURABILITY-TOPOLOGY
created_at: 2026-08-24T21:00:00+02:00
updated_at: 2026-08-24T21:00:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths_after_allocation_merge:
  - docs/architecture/reviews/OTERYN_GAME_DURABILITY_TOPOLOGY_DECISION_PACKET_2026-08-24.md
  - docs/agents/tasks/active/OTV2-20260824-prep-durability-topology.md
public_contracts:
  - docs/agents/prompts/OTV2_PREP_DURABILITY_TOPOLOGY.md
  - docs/superpowers/plans/2026-08-24-oteryn-game-next-wave-master-plan.md
depends_on:
  - Oteryn-Game#94
blocks:
  - OTV2-IMPL-DURABILITY exact implementation allocation
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Produce the exact Issue #94 Durability topology decision packet and implementation-allocation proposal without mutating runtime, DDL, migrations, dependencies, registries, workflows or production infrastructure.

## Architecture and source of truth

The worker must consume live Issue #94, root/nearest governance, the next-wave master plan, ADR-0004, DUR-01/02/03, FND-03/04, GAME-CHAR, GAME-ITEM, ANL-01, current workspace/Cargo topology, `RESOURCE_LIMITS_REGISTRY.json` and the live allocation record. Lower-level convenience never overrides accepted authority or hard-max gates.

## Acceptance criteria

- [ ] Exact runtime/migration/test/shared paths are proposed.
- [ ] Viable Rust PostgreSQL/migration candidates are compared with Rust 1.94 compatibility evidence.
- [ ] One DB client/migration approach is selected with maintenance/security/supply-chain rationale.
- [ ] One immutable game-owned migration history and dedicated migration execution path are frozen.
- [ ] Production startup auto-DDL is forbidden and schema compatibility fails closed.
- [ ] Isolated non-production PostgreSQL test lifecycle is defined.
- [ ] Async `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE` preserves the FND-03 writer-lane boundary.
- [ ] Stable TransactionId/OperationId, ambiguous commit and required audit/outbox atomicity are explicit.
- [ ] Every first-increment DUR-03 amplification/resource dimension is classified and missing accepted hard maxima remain blockers.
- [ ] Final handoff is either `READY_FOR_EXACT_DURABILITY_ALLOCATION` or a precise blocker list.

## Excluded scope

No runtime code, DDL, migrations, dependencies, Cargo/lockfile, resource-registry mutation, workflow change, database provisioning, production configuration/secrets, Platform write or external-repository mutation. The packet cannot self-grant implementation authority.

## Implementation / findings

Pending allocation merge.

## Validation

### Focused

- command/run: source/version compatibility evidence plus packet completeness review
- result: pending

### Component/integration

- command/run: governance validation, `git diff --check`, placeholder scan and whole-diff review
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — preparation packet only; DB-E2E obligations are specified for the later implementation
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
- method/reviewer: preparation agent whole-diff review
- material findings: pending
- verdict: pending

## Independent review

- required: `NO` for the paper-only packet itself unless repository policy or changed semantics make it applicable; later persistence/item/value implementation requires genuinely independent exact-head review
- exact head: pending or `NOT_APPLICABLE`
- method/auditor: pending or `NOT_APPLICABLE`
- material findings: pending or `NOT_APPLICABLE`
- verdict: pending or `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: Issue #94
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: coordinator allocation candidate created; worker remains read-only until that allocation merges
status: allocation_pending_merge
branch: docs/otv2-prep-durability-topology-94
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
blocker: coordinator allocation PR must merge before this task gains write authority
next_action: after allocation merge, branch from exact merged allocation base and write only the decision packet plus this task record
```
