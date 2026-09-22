# OTV2-20260922-control-plane-simplification-745

```yaml
task_id: OTV2-20260922-control-plane-simplification-745
title: Simplify candidate qualification and coordinator authority
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/control-plane-simplification-745-v2
issue: 745
pr: null
base_sha: bbda147eedc7b01c39827ab4784b5760032b1237
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: work coordinator
created_at: 2026-09-22T09:43:00+02:00
updated_at: 2026-09-22T10:52:00+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/agents/validate_governance.py
  - tools/agents/validate_inherited_prompt_policy.py
  - tools/agents/tests/test_validate_governance_lifecycle.py
  - tools/agents/tests/test_meta_agent_policy_adoption.py
  - docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
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
- PROVEN: PR #739 owns merge-gate/MQ/rust workflow and PG-policy paths; this task is path-disjoint and must not edit them.
- PROVEN: #740 lifecycle closeout was independently integrated by #743 before this successor branch; this task does not re-own that completed cleanup.

## High-risk authority/recovery qualification

NOT_APPLICABLE — repository governance/qualification only; no production, gameplay, protocol, persistence, account/session authority or external-repository mutation.

## Acceptance criteria

- [ ] inherited routing snapshot drift simplification is serialized to phase 2 after #739 protected integration because #739 currently owns the required workflow/policy surfaces;
- [ ] candidate-caused routing drift still selects conservative validation and protected-main routing drift still fails;
- [x] active-task live-state validation on PRs is restricted to task packets changed by that exact head against one stable live base snapshot; protected-main/full-health validation still scans all active packets;
- [x] redundant `OTV2_IMPLEMENTATION_COORDINATOR` is retired in favor of the sole reusable `OTV2_WORK_DELIVERY_COORDINATOR`;
- [x] #740 stale active ownership was independently closed on protected main by #743 before this successor branch;
- [x] no #739-owned path changes in PR #748;
- [ ] focused lifecycle/routing/META regressions and exact-head repository CI pass.

## Excluded scope

No ruleset/protection change, no direct merge/auto-merge substitute, no Merge Queue weakening, no product/runtime code, no PostgreSQL routing changes from #739, no force/rebase/reset, no production or external-repository mutation.

## Context checkpoint

```yaml
last_progress: PR #748 phase-one candidate is path-disjoint from #739, tolerates protected-main advancement without freshness merge-up, and collapses retired coordinator successors directly onto Work; Issue #745 remains the canonical live authority across phase 2
status: validating
branch: ci/control-plane-simplification-745-v2
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
next_action: qualify the repaired exact head, complete one independent deep review, then prepare protected integration of phase 1 while phase 2 stays serialized behind #739
```
