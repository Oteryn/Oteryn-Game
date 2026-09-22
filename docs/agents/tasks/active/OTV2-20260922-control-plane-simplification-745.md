# OTV2-20260922-control-plane-simplification-745

```yaml
task_id: OTV2-20260922-control-plane-simplification-745
title: Simplify candidate qualification and coordinator authority
mode: GOVERNANCE
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 745
pr: null
base_sha: 86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: work coordinator
created_at: 2026-09-22T09:43:00+02:00
updated_at: 2026-09-22T14:10:00+02:00
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

Keep one authoritative mutating Game coordinator and simplify candidate qualification without weakening exact-head checks, protected-main health, required `game-gate`, Merge Queue or real `merge_group` qualification.

## Architecture and source of truth

- PROVEN: Phase 1 PR #748 is protected-integrated as `86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1` after real Merge Queue `game-gate=SUCCESS`.
- PROVEN: WP5 routing owner PR #739 is protected-integrated as `cf5c5f35476559450b6bbaf87dce519f7eead9d0`; the Phase-2 serialization dependency is closed.
- PROVEN: Phase 1 makes PR live-task validation candidate-scoped while protected-main/non-PR health still scans the complete active-task set.
- PROVEN: `OTV2_IMPLEMENTATION_COORDINATOR` is retired; `Oteryn: work coordinator` remains the sole reusable mutating Game control plane.
- CURRENT: no Phase-2 writer/branch/PR is allocated by this checkpoint. Live GitHub and fresh Work allocation outrank this record.

## High-risk authority/recovery qualification

NOT_APPLICABLE — repository governance/qualification only; no production, gameplay, protocol, persistence, account/session authority or external-repository mutation.

## Acceptance criteria

- [x] PR candidate live-task validation is scoped to candidate-touched active packets.
- [x] protected-main/non-PR health still validates the complete active-task set.
- [x] duplicate implementation coordinator is retired in favor of Work.
- [x] Phase 1 passed exact-head CI, governed Merge Queue, real `merge_group` `game-gate` and protected-main readback.
- [ ] Phase 2 classifies inherited routing drift without weakening candidate-caused/protected-main failures.
- [ ] Phase 2 adds pre-queue `git diff --check` on the coherent workflow/policy surface.
- [ ] Phase 2 is freshly allocated after path/ownership reconciliation and protected normally.

## Excluded scope

No ruleset/protection weakening, direct merge/auto-merge substitute, product/runtime code, production mutation, force/reset/rebase, or implicit seizure of #739/#757/#749 paths.

## Context checkpoint

```yaml
last_progress: Phase 1 PR #748 protected-integrated through real Merge Queue; Phase 2 dependency #739 is also protected
status: waiting
branch: null
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
blocker: PHASE2_FRESH_ALLOCATION_NOT_ISSUED
next_action: Work performs a fresh overlap/custody readback and allocates one bounded Phase-2 writer on a new canonical branch
```
