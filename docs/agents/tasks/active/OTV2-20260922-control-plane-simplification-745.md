# OTV2-20260922-control-plane-simplification-745

```yaml
task_id: OTV2-20260922-control-plane-simplification-745
title: Simplify candidate qualification and coordinator authority
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/control-plane-simplification-745-phase2
issue: 745
pr: null
base_sha: 86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: work coordinator
created_at: 2026-09-22T09:43:00+02:00
updated_at: 2026-09-22T14:21:00+02:00
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_pr_routing_contract.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/tasks/active/OTV2-20260922-control-plane-simplification-745.md
public_contracts:
  - Game candidate qualification and coordinator lifecycle
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Keep one authoritative mutating Game coordinator and make PR qualification candidate-scoped: unrelated protected-main lifecycle/routing drift must not invalidate an unchanged exact PR head, while candidate-caused defects, protected-main health, required `game-gate`, Merge Queue and real `merge_group` qualification remain fail-closed.

## Architecture and source of truth

- PROVEN: META policy 3.1 says ordinary protected-main movement does not invalidate a qualified head by itself and Merge Queue owns composition with current `main`.
- PROVEN: repository ruleset 20991995 requires `game-gate`, has Merge Queue enabled and does not require strict up-to-date PR branches.
- PROVEN: PR #737 exposed the inherited-routing false-blocker class and is now protected as `main@9daf3522efbf799c5d9ffe9817215895d4fa8af0`; #745 is therefore systemic cleanup, not a prerequisite for that completed Item integration.
- PROVEN: PR #739 is protected-integrated; its former workflow/policy ownership is released. Phase 2 now owns only the minimal PR-gate routing surfaces listed above.
- PROVEN: #740 lifecycle closeout was independently integrated by #743 before this successor branch; this task does not re-own that completed cleanup.

## High-risk authority/recovery qualification

NOT_APPLICABLE — repository governance/qualification only; no production, gameplay, protocol, persistence, account/session authority or external-repository mutation.

## Acceptance criteria

- [ ] inherited routing snapshot drift is non-fatal only for a disjoint PR candidate when the trusted protected base is healthy;
- [ ] candidate-caused routing drift still selects conservative FULL validation; protected-base and protected-main routing drift still fail closed;
- [x] active-task live-state validation on PRs is restricted to task packets changed by that exact head against one stable live base snapshot; protected-main/full-health validation still scans all active packets;
- [x] redundant `OTV2_IMPLEMENTATION_COORDINATOR` is retired in favor of the sole reusable `OTV2_WORK_DELIVERY_COORDINATOR`;
- [x] #740 stale active ownership was independently closed on protected main by #743 before this successor branch;
- [x] no #739-owned path changes in PR #748;
- [ ] exact-head PR gate catches whitespace before Merge Queue;
- [ ] focused routing/repository-policy regressions and exact-head repository CI pass.

## Excluded scope

No ruleset/protection change, no direct merge/auto-merge substitute, no Merge Queue weakening, no product/runtime code, no PostgreSQL routing changes from #739, no force/rebase/reset, no production or external-repository mutation.

## Context checkpoint

```yaml
last_progress: phase 2 authored on the released minimal routing surfaces: stale-inherited candidate drift is advisory, trusted protected-base snapshot health is fatal, and PR routing now runs git diff --check before Merge Queue
status: validating
branch: ci/control-plane-simplification-745-phase2
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
next_action: freeze exact branch head, read back bounded diff/ownership, open one phase-2 PR, and require focused regressions plus exact-head game-gate
```
