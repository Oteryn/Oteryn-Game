# OTV2-20260806-governance-ci-recovery

```yaml
task_id: OTV2-20260806-governance-ci-recovery
title: Harden exact-head governance CI recovery
mode: GOVERNANCE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: governance/ci-recovery-hardening-20260806
pr: 58
base_sha: 26b5fa275fba19fdee0e26a6f65263489af3e500
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT governance coordinator
created_at: 2026-08-06T20:47:00+02:00
updated_at: 2026-08-06T20:58:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/active/OTV2-20260806-governance-ci-recovery.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Prevent exact-head closeout loops caused by connector-suppressed workflow events, runner starvation and CI-nudge commits while preserving the protected `Agent governance / validate` merge gate.

## Architecture and source of truth

- `PROVEN`: main requires `Agent governance / validate` and allows no routine bypass.
- `PROVEN`: successful connector writes may leave an exact head with no workflow/check run.
- `PROVEN`: a queued job may remain unassigned with `runner_id = 0` and no steps started.
- `PROVEN`: a commit cannot contain its own final SHA; exact frozen-head evidence must therefore live in the PR/check run after the final commit rather than causing another self-referential checkpoint commit.
- `DERIVED`: manual dispatch is a valid recovery path only when it fails closed on PR identity, same-repository ownership and exact expected head.

## Acceptance criteria

- [x] Define a final-head freeze that forbids CI-nudge commits and duplicate replacement PRs used only to regenerate checks.
- [x] Distinguish event suppression, runner starvation and an executed workflow failure.
- [x] Define one bounded recovery path and an exact owner-action blocker when the connector lacks dispatch/cancel operations.
- [x] Harden manual governance dispatch with required PR number and expected 40-character head SHA.
- [x] Verify the dispatched PR is open, targets `main`, comes from `blakinio/Oteryn-v2` and still has the expected head.
- [x] Preserve the required check name `Agent governance / validate` and all branch protections.
- [ ] Run governance and repository-policy validation on the exact final head.
- [ ] Complete independent adversarial review and exact-head protected CI.

## Excluded scope

- no ruleset bypass or weakening;
- no automatic success status creation;
- no `pull_request_target` execution of PR-controlled code;
- no external repository changes;
- no modification of UUIDv7 architecture PR #57.

## Implementation / findings

- Added explicit `EVENT_SUPPRESSED`, `RUNNER_STARVATION`, `WORKFLOW_FAILURE`, `WORKFLOW_CANCELLED` and normal-wait classifications.
- Limited CI recovery to one action per frozen exact head.
- Prohibited no-op/activity commits, branch rewinds, close/reopen loops and replacement PRs whose only purpose is check generation.
- Defined immutable PR/workflow evidence as the place for post-freeze SHA, audit and CI results.
- Hardened `workflow_dispatch` to require an open PR number and full expected head SHA.
- The dispatch workflow verifies selected ref, current PR head, same-repository ownership, `main` target and conventional PR metadata before checking out exact target content.
- Preserved read-only repository/PR permissions and the required job name.
- Added task/recovery fields for trigger source, run/job IDs, runner assignment and required owner action.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: to be recorded against the final PR head without moving it

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`
- result: to be recorded against the final PR head without moving it

### E2E

- scenario: dispatch validation rejects stale SHA/wrong PR and validates an open same-repository PR at the selected exact head
- result: requires retained workflow execution after the final head exists

### Exact-head CI

- final head: record in PR review/check evidence after this implementation commit
- trigger source: `pull_request` or trusted `workflow_dispatch`
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: record in PR review after the final implementation commit
- method/auditor: adversarial governance/workflow review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: seven declared files
- unresolved review threads: pending
- related/superseded PRs: UUIDv7 PR #57 is incident evidence only and remains untouched
- protected auto-merge: pending exact-head PASS
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Implemented bounded exact-head CI recovery policy and fail-closed manual dispatch on PR #58.
status: validating
branch: governance/ci-recovery-hardening-20260806
head_sha: record from live PR after the implementation commit
pr: 58
final_head_sha: record in immutable PR evidence; do not move head merely to fill this field
final_head_frozen_at: after focused validation and full diff review
ci_trigger_source: pending
ci_check_generation: pending
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
next_action: Validate the implementation commit locally, review the complete diff, then freeze and audit the exact PR head without another checkpoint commit.
```
