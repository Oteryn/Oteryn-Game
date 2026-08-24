# OTV2-20260825-close-next-wave-blockers

```yaml
task_id: OTV2-20260825-close-next-wave-blockers
title: Close next-wave blockers coordinator allocation
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/close-next-wave-blockers-131
issue: 131
pr: 132
base_sha: 9cc23cdbfe68d0a0f13df054874929b5e5dbe418
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT coordinator session for Issue #131
created_at: 2026-08-25T00:55:13+02:00
updated_at: 2026-08-25T00:58:42+02:00
execution_budget_minutes: 120
large_budget_reason: Four independent blocker lifecycles require evidence, a serialized registry mutation, one TDD security implementation, exact-head review/CI, merges and terminal ownership reconciliation.
owned_paths:
  - docs/agents/tasks/active/OTV2-20260825-close-next-wave-blockers.md
  - docs/superpowers/plans/2026-08-25-oteryn-close-next-wave-blockers-implementation-plan.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
public_contracts: []
depends_on:
  - issue:128
  - main:9cc23cdbfe68d0a0f13df054874929b5e5dbe418
blocks:
  - issue:93
  - issue:115
  - issue:116
  - issue:123
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Issue #131 owns a reviewed, executable decomposition that terminally closes blockers #93, #115, #116 and #123 while keeping every gameplay/listener/durability/client implementation outside this coordinator allocation.

## Architecture and source of truth

- PROVEN: Issue #128 and OTV2_CLOSE_NEXT_WAVE_BLOCKERS.md authorize this exact closure programme and the bounded #115 implementation exception.
- PROVEN: baseline main@9cc23cdbfe68d0a0f13df054874929b5e5dbe418 passes governance, diff check and cargo test --workspace --all-targets.
- PROVEN: all four target blocker issues remain open and no active non-Dependabot PR owns their paths.
- PROVEN: shared registry, Cargo/workspace and Foundation composition paths are currently released; this docs-only allocation does not acquire them.
- DERIVED: decision/evidence, registry, verifier implementation and terminal closeout must use separate independently mergeable branches/PRs.

## Acceptance criteria

- [x] The implementation plan defines exact tasks, paths, RED/GREEN gates, review and closeout order.
- [x] Live allocations names Issue #131 and this branch without granting child write authority early.
- [x] Governance and diff checks pass.
- [ ] Exact-head protected CI including game-gate passes and the allocation PR merges.

## Excluded scope

No registry mutation, Cargo/lockfile mutation, Foundation code, Server Seam listener, Durability implementation, Ability/Interaction/AI/Movement/Combat/Client implementation, production configuration, secrets, deployment, Platform or external-repository write.

## Implementation / findings

Coordinator preflight is complete. The only current open PRs are unrelated Dependabot updates. The accepted prompt and blocker packets are present on the exact base. Child paths remain unallocated until this plan merges.

## Validation

### Focused

- command/run: python tools/agents/validate_governance.py
- result: PASS — 25 required policy documents and 9 project lanes validated.

### Component/integration

- command/run: git diff --check
- result: PASS — no whitespace/error-marker defects.

### E2E

- scenario: NOT_APPLICABLE — docs-only coordinator allocation has no runtime behavior
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: docs-only governance/allocation
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: Initial task packet omitted the validator-required positive Issue field; repaired with `issue: 131`. Whole allocation diff otherwise matches Issue #128 scope and grants no child/shared path early.
- verdict: PASS

## Independent review

- required: NO — this PR allocates docs-only work and creates no security/durable/runtime authority; protected semantic/governance audits remain required
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: after allocation merge; Issue #131 remains coordinator authority for child tasks

## Context checkpoint

```yaml
last_progress: PR #132 is open; all final task/PR metadata except immutable exact-head evidence is prepared.
status: validating
branch: coord/close-next-wave-blockers-131
head_sha: null
pr: 132
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
next_action: Freeze the exact head, run protected CI and merge PR #132.
```
