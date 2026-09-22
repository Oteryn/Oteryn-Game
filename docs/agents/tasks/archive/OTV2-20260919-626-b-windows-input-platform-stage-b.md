> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #701 merged as `e765a314ceb2b81e4260dd1dee1f1e547f6ce920`, and the canonical task branch is deleted. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-626-b-windows-input-platform-stage-b

```yaml
task_id: OTV2-20260919-626-b-windows-input-platform-stage-b
title: Activate Windows input-platform CI evidence
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
issue: 626
base_branch: main
branch: agent/626-b-windows-input-platform-stage-b
pr: null
base_sha: f4f1292544b1fffbb09e9df6ebefd5c8bb1f7879
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl qa
created_at: 2026-09-19T00:00:00Z
updated_at: 2026-09-20T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/merge-group-gate.yml
  - .github/workflows/rust.yml
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260919-626-b-windows-input-platform-stage-b.md
public_contracts: []
depends_on:
  - "#626 Finding B Stage A / PR #678"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The authoritative PR, Merge Queue and protected-main Windows lanes execute the existing `oteryn-input-platform` package tests unconditionally on `x86_64-pc-windows-msvc` without weakening any existing evidence.

## Architecture and source of truth

- `PROVEN`: #162 allocation comment 5743678218 defines the exact Stage B semantic and path custody.
- `PROVEN`: Stage A preapproved Merge Queue workflow blob `ad439cf3b04aaea084521f7be37761d3b1458cc5`.
- `PROVEN`: `docs/agents/BUILD_TEST_MATRIX.md` defines the current executable evidence surfaces.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this change activates CI test evidence and does not perform or authorize a production mutation, authority transition or persisted-state recovery.

## Acceptance criteria

- [x] The exact Windows input-platform test command is unconditional in PR, Merge Queue and protected-main Windows lanes.
- [x] The Merge Queue workflow Git blob is exactly `ad439cf3b04aaea084521f7be37761d3b1458cc5`.
- [x] Focused policy regressions reject deletion, conditioning, rename and material command alteration.
- [x] Focused validators, repository policy, governance and custody checks pass on the final candidate.
- [ ] Normal exact-head hosted CI and independent CONTROL/HIGH review qualify the published candidate.

## Excluded scope

No product/runtime code, input-platform source/tests, protected audit workflow, ruleset, branch protection, unrelated workflow dependency update or external/production system is changed.

## Implementation / findings

The current baseline omitted the package test on all three authoritative Windows surfaces. The candidate inserts one exact test step immediately before deterministic simulation on each surface and refreshes only the deterministic pins, regressions and matrix statements made stale by those insertions.

## Validation

### Focused

- command/run: `python tools/repository/test_validate_pr_gate_pg_sim.py`; `python tools/repository/test_validate_merge_group_pg_sim.py`
- result: PASS, including 21 PR-gate regressions and 16 Merge Queue workflow mutations.

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`; `python tools/agents/validate_governance.py`; `git diff --check`
- result: PASS

### E2E

- scenario: `NOT_APPLICABLE`; workflow-policy activation is qualified by focused policy regressions and hosted Windows execution.
- result: pending hosted exact-head evidence

### Exact-head CI

- final head: pending publication
- trigger source: canonical pull request after worker handoff
- workflow/run/job: pending
- runner assignment: pending
- classification: FULL
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent, whole-diff adversarial review
- material findings: none; the changed paths remain exactly the nine allocated paths and the Merge Queue workflow is only the preapproved insertion.
- verdict: PASS

## Independent review

- required: YES; authoritative workflow/control-plane evidence change
- exact head: pending
- method/auditor: coordinator-routed CONTROL/HIGH review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: Stage A PR #678
- protected auto-merge: forbidden for worker
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Completed focused policy, repository-policy, governance, exact-blob, custody and whole-diff validation.
status: validating
branch: agent/626-b-windows-input-platform-stage-b
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
next_action: Publish the exact candidate for control-plane PR qualification.
```
