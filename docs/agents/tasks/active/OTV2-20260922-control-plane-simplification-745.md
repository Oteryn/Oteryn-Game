# OTV2-20260922-control-plane-simplification-745

```yaml
task_id: OTV2-20260922-control-plane-simplification-745
title: Simplify candidate qualification and coordinator authority
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/control-plane-simplification-745
issue: 745
pr: null
base_sha: 8ad99922d7140016df2de38bc3b6ae3a8d4edc1e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: work coordinator
created_at: 2026-09-22T09:43:00+02:00
updated_at: 2026-09-22T09:43:00+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/repository/validate_pr_routing_contract.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/agents/validate_governance.py
  - tools/agents/validate_inherited_prompt_policy.py
  - tools/agents/tests/test_validate_governance_lifecycle.py
  - tools/agents/tests/test_meta_agent_policy_adoption.py
  - docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/tasks/active/OTV2-20260922-agent-hygiene-cleanup-740.md
  - docs/agents/tasks/archive/OTV2-20260922-agent-hygiene-cleanup-740.md
  - docs/agents/tasks/active/OTV2-20260922-control-plane-simplification-745.md
public_contracts:
  - Game candidate qualification and coordinator lifecycle
depends_on: []
blocks:
  - 737
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Keep one authoritative mutating Game coordinator and make PR qualification candidate-scoped: unrelated protected-main lifecycle/routing drift must not invalidate an unchanged exact PR head, while candidate-caused defects, protected-main health, required `game-gate`, Merge Queue and real `merge_group` qualification remain fail-closed.

## Architecture and source of truth

- PROVEN: META policy 3.1 says ordinary protected-main movement does not invalidate a qualified head by itself and Merge Queue owns composition with current `main`.
- PROVEN: repository ruleset 20991995 requires `game-gate`, has Merge Queue enabled and does not require strict up-to-date PR branches.
- PROVEN: PR #737 currently fails from inherited routing/task lifecycle state already repaired on protected `main`.
- PROVEN: PR #739 owns merge-gate/MQ/rust workflow and PG-policy paths; this task is path-disjoint and must not edit them.
- PROVEN: #740 remains an active ownership packet after its canonical PR #741 merged; this task explicitly supersedes that stale ownership record.

## High-risk authority/recovery qualification

NOT_APPLICABLE — repository governance/qualification only; no production, gameplay, protocol, persistence, account/session authority or external-repository mutation.

## Acceptance criteria

- [ ] inherited routing snapshot drift remains visible but no longer fails an otherwise disjoint exact-head PR;
- [ ] candidate-caused routing drift still selects conservative validation and protected-main routing drift still fails;
- [ ] active-task live-state validation on PRs is restricted to task packets changed by that candidate; protected-main/full-health validation may still scan all active packets;
- [ ] redundant `OTV2_IMPLEMENTATION_COORDINATOR` is retired in favor of the sole reusable `OTV2_WORK_DELIVERY_COORDINATOR`;
- [ ] #740 stale active ownership packet is archived/superseded;
- [ ] no #739-owned path changes;
- [ ] focused lifecycle/routing/META regressions and exact-head repository CI pass.

## Excluded scope

No ruleset/protection change, no direct merge/auto-merge substitute, no Merge Queue weakening, no product/runtime code, no PostgreSQL routing changes from #739, no force/rebase/reset, no production or external-repository mutation.

## Context checkpoint

```yaml
last_progress: issue #745 and dedicated branch created from protected main
status: implementing
branch: ci/control-plane-simplification-745
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
next_action: implement candidate-scoped routing/live-state regressions and retire duplicate coordinator authority
```
