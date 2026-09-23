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
base_sha: fcf4be6384e6d590094060ea73078cbbdfe70e5c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: work coordinator
created_at: 2026-09-22T09:43:00+02:00
updated_at: 2026-09-23T09:25:00+02:00
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

Keep one authoritative mutating Game coordinator and simplify candidate qualification without weakening exact-head checks, protected-main health, required `game-gate`, Merge Queue or real `merge_group` qualification.

## Architecture and source of truth

- PROVEN: Phase 1 PR #748 is protected-integrated as `86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1` after real Merge Queue `game-gate=SUCCESS`.
- PROVEN: WP5 routing owner PR #739 is protected-integrated as `cf5c5f35476559450b6bbaf87dce519f7eead9d0`; its former workflow/policy ownership is released.
- PROVEN: Phase 1 makes PR live-task validation candidate-scoped while protected-main/non-PR health still scans the complete active-task set.
- PROVEN: `OTV2_IMPLEMENTATION_COORDINATOR` is retired; `Oteryn: work coordinator` remains the sole reusable mutating Game control plane.
- PROVEN: Phase 2 is allocated on `ci/control-plane-simplification-745-phase2` / PR #759 and owns only the five paths listed above.
- PROVEN: before this reconciliation, exact head `f6f2d9a0ce4ee732e4f6f4f8286bdb50b9e50e02` passed Agent Governance, Architecture Semantic Audit, Merge Gate/`game-gate` and independent Codex review.
- PROVEN: governed Merge Queue rejected that head only because protected main later changed the active #745 task packet; the four material control-plane paths remained disjoint.
- CURRENT: this normal non-force merge-up reconciles current protected `main` into the existing Phase-2 branch; no rebase/reset/force/direct merge/auto-merge substitute is authorized.
## High-risk authority/recovery qualification

NOT_APPLICABLE — repository governance/qualification only; no production, gameplay, protocol, persistence, account/session authority or external-repository mutation.

## Acceptance criteria

- [x] PR candidate live-task validation is scoped to candidate-touched active packets.
- [x] protected-main/non-PR health still validates the complete active-task set.
- [x] duplicate implementation coordinator is retired in favor of Work.
- [x] Phase 1 passed exact-head CI, governed Merge Queue, real `merge_group` `game-gate` and protected-main readback.
- [x] inherited routing snapshot drift is advisory only for a disjoint candidate when the trusted protected base is healthy.
- [x] candidate-caused routing drift remains conservative/FULL; protected-base and protected-main routing drift remain fail-closed.
- [x] exact-head PR routing runs `git diff --check` before Merge Queue.
- [x] repository-policy pins and mutation regressions protect the trusted-base and pre-queue whitespace checks.
- [ ] successor head after this protected-main reconciliation passes exact-head CI and independent review.
- [ ] governed Merge Queue produces real `merge_group game-gate=SUCCESS` and protected-main readback for Phase 2.
- [ ] terminal task packet is archived and Issue #745 is closed after protected Phase-2 integration.

## Excluded scope

No ruleset/protection weakening, direct merge/auto-merge substitute, product/runtime code, production mutation, force/reset/rebase, or implicit seizure of unrelated paths.

## Context checkpoint

```yaml
last_progress: >-
  Phase 2 implementation and first exact-head qualification/review are green; governed MQ exposed one real task-packet merge conflict after protected-main movement, and this approved normal merge-up reconciles only that conflict while preserving the four disjoint material control-plane paths
status: validating
branch: ci/control-plane-simplification-745-phase2
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: successor_after_main_merge_up
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: commit and push the non-force main merge-up, freeze the successor exact head, rerun exact-head CI and independent review, submit through governed Merge Queue, require real merge_group game-gate SUCCESS and protected-main readback, then archive this task and close Issue #745
```
