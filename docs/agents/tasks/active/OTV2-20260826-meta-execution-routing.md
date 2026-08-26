# OTV2-20260826-meta-execution-routing

```yaml
task_id: OTV2-20260826-meta-execution-routing
title: Adopt merged META execution-routing policy
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/meta-execution-routing-201
pr: null
base_sha: f31453f65477ae9966d724d67bdd2c1857318be1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: oteryn-governance-controller
created_at: 2026-08-26T16:55:00Z
updated_at: 2026-08-26T16:55:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - AGENTS.md
  - docs/agents/tasks/active/OTV2-20260826-meta-execution-routing.md
public_contracts:
  - Oteryn META execution-routing policy at 8fac1d55805fc3372351ea0a55ad7728b3570ebc
depends_on:
  - Oteryn/Oteryn PR #90 merged
blocks: []
cross_repository_coordination_id: Oteryn/Oteryn#90
external_repositories:
  - Oteryn/Oteryn
```

## Outcome

Game adopts the merged META execution-routing policy by reference without runtime, runner-host or deployment changes.

## Acceptance criteria

- [ ] Root instructions require CI/isolated-workspace first, RDC default-deny and fresh GitHub resume state.
- [ ] Substantial tasks require parallel-first lanes with exclusive worktree ownership.
- [ ] Governance validation and exact-head CI pass.

## Excluded scope

No Game runtime, Cargo, workflows, runner configuration, Desktop Commander session or production system change.

## Context checkpoint

```yaml
last_progress: Issue #201 and isolated governance branch created from current main
status: implementing
branch: governance/meta-execution-routing-201
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
next_action: validate the scoped governance change and open the Game PR
```
