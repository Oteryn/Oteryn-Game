# OTV2-20260904-merge-group-audit-pin-rotation

```yaml
task_id: OTV2-20260904-merge-group-audit-pin-rotation
title: Rotate protected merge-group gate blob authority
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/merge-group-audit-pin-284
pr: null
issue: 284
parent_issue: 277
base_sha: b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol implementation worker
created_at: 2026-09-04T16:46:00Z
updated_at: 2026-09-04T16:46:00Z
execution_budget_minutes: 60
large_budget_reason: protected-base merge-authority rotation
owned_paths:
  - .github/workflows/merge-authority-audit.yml
  - docs/agents/tasks/active/OTV2-20260904-merge-group-audit-pin-rotation.md
public_contracts:
  - protected merge-group gate blob authority
depends_on:
  - issue: 277
blocks:
  - issue: 285
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Protected-base merge-authority audit preapproves exactly one strengthened future merge-group gate blob, without activating it or allowing candidate-controlled self-authorization.

## Architecture and source of truth

- `PROVEN` — protected main at allocation is `b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3`.
- `PROVEN` — current protected audit pins merge-group gate blob `1e0e7b70a806fe744d394ca8abf43ee434ead3f2` and intentionally rejects candidates that modify the audit itself.
- `PROVEN` — exact unattached future gate blob `e3291fe8fca8fcf70166d5652b43d5a26fa0d762` has been prepared before this rotation and includes exact-head PG 17.6 plus Windows SIM in the merge-group `game-gate` fan-in.
- `PROVEN` — owner explicitly instructed completion of this approved programme on 2026-09-04; no bypass or direct-main mutation is authorized.
- `DERIVED` — rotating the protected pin first preserves the protected-base trust boundary for subsequent #285 activation.

## Acceptance criteria

- [ ] Audit pin is exactly `e3291fe8fca8fcf70166d5652b43d5a26fa0d762`.
- [ ] Required fragments require the exact new fan-in, PostgreSQL job/image/test, exact merge-group SHA evidence and Windows simulation command.
- [ ] Candidate code is never checked out by the protected-base audit.
- [ ] Same-repository, exact-head and main-base checks remain unchanged.
- [ ] Forbidden permissions/actions/bypass patterns remain unchanged or stricter.
- [ ] No merge-group gate is activated in this task.
- [ ] Repository-policy/agent-governance/canonical `game-gate` pass as applicable.
- [ ] The expected protected-base audit self-modification rejection is explicitly recorded and is not misreported as product validation failure.
- [ ] One independent deep review of the stable rotation candidate has no unresolved actionable finding.
- [ ] Integration uses normal protected controls and protected-main readback.

## Excluded scope

No `merge-group-gate.yml` mutation, ruleset change, required-status change, runtime change, production/secret/external-repository mutation or second attestation/control plane.

## Implementation / findings

This is an exceptional protected-audit rotation. The audit's existing self-modification rejection must remain in place; therefore the protected-base audit is expected to reject this PR specifically because it edits `.github/workflows/merge-authority-audit.yml`. That expected advisory/control-plane signal must not be disabled. Integration requires current owner authorization plus deterministic validation and independent review.

## Validation

### Focused

- precomputed future gate blob: `e3291fe8fca8fcf70166d5652b43d5a26fa0d762`
- result: immutable Git blob created, not attached to any branch

### Component/integration

- repository policy / governance: pending
- exact audit diff review: pending

### E2E

- `NOT_APPLICABLE` — audit pin only

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- classification: protected control-plane rotation
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES — protected merge authority change
- exact head: pending
- method/auditor: one independent deep review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly two allocated paths
- unresolved review threads: pending
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: after protected-main readback

## Context checkpoint

```yaml
last_progress: exact future gate blob computed before protected pin rotation
status: implementing
branch: ci/merge-group-audit-pin-284
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
next_action: rotate only the protected audit pin and exact required future-gate fragments, then open Draft PR and run deterministic validation plus one independent deep review
```
