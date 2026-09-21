# OTV2-20260904-merge-group-audit-pin-rotation

```yaml
task_id: OTV2-20260904-merge-group-audit-pin-rotation
title: Rotate protected merge-group gate blob authority
mode: GOVERNANCE
status: WAITING_DEPENDENCY
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/merge-group-audit-pin-284
pr: 291
issue: 284
parent_issue: 277
base_sha: b67f4425e9e9c5bbf9f7bc94c422cd7478edcdd3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex single mutating writer authorized by owner continuation
created_at: 2026-09-04T16:46:00Z
updated_at: 2026-09-04T22:11:17Z
execution_budget_minutes: 60
large_budget_reason: protected-base merge-authority rotation
owned_paths:
  - .github/workflows/merge-authority-audit.yml
  - tools/repository/validate_repository_policy.py
  - docs/agents/tasks/active/OTV2-20260904-merge-group-audit-pin-rotation.md
public_contracts:
  - protected merge-group gate blob authority
depends_on:
  - issue: 277
  - issue: 279
    state: waiting_protected_main_integration
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

- [x] Audit pin is exactly `e3291fe8fca8fcf70166d5652b43d5a26fa0d762`.
- [x] Required fragments require the exact new fan-in, PostgreSQL job/image/test, exact merge-group SHA evidence and Windows simulation command.
- [x] Candidate code is never checked out by the protected-base audit.
- [x] Same-repository, exact-head and main-base checks remain unchanged.
- [x] Forbidden permissions/actions/bypass patterns remain unchanged or stricter.
- [x] No merge-group gate is activated in this task.
- [ ] Repository-policy/agent-governance/canonical `game-gate` pass as applicable.
- [x] The expected protected-base audit self-modification rejection is explicitly recorded and is not misreported as product validation failure.
- [ ] One independent deep review of the stable rotation candidate has no unresolved actionable finding.
- [ ] Integration uses normal protected controls and protected-main readback.

## Excluded scope

No `merge-group-gate.yml` mutation, ruleset change, required-status change, runtime change, production/secret/external-repository mutation or second attestation/control plane.

## Implementation / findings

Issue #284 expanded the initial two-path allocation to the exact three paths above before the accepted expression-interpolation P1 repair. This record corrects the stale allocation and missing PR binding; it does not grant new scope.

P1 `3936185751` / thread `PRRT_kwDOT8SzxM6fYCU0` was accepted. Validator-only RED `f2450ebbcd483b50786ff5d5484e641db6a38a9e` failed the six expression-safety contracts in hosted job `101105184640`. Material GREEN `0b134215d60bf9c63ef8406f97043d7b09755bbf` constructs future-context expressions at Python runtime using `'$' + '{{ ... }}'`, preserving their literal values for matching. The RED/repair is complete; do not restart it from the old Issue next_action.

Read-only prequalification was repeated on that exact material head during the owner-authorized takeover of #287/#291. The entire audit Python script compiles, contains no premature workflow expressions, and all 12 required fragments match the retrieved future gate. Its Git blob hash was independently recomputed as `e3291fe8fca8fcf70166d5652b43d5a26fa0d762`. Local repository policy and agent governance both pass. No additional confirmed production defect was found in the three-file diff.

Protected main is now `68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705`. Final reconciliation is deliberately sequenced after #287 protected-main integration so the shared repository-policy wrapper retains both #287 PG/SIM validation and this audit-expression validation. The owner authorized repair and qualification only, with no merge. This task therefore remains `WAITING_DEPENDENCY`; prequalification is not the final independent deep review.

This is an exceptional protected-audit rotation. The audit's existing self-modification rejection must remain in place; therefore the protected-base audit is expected to reject this PR specifically because it edits `.github/workflows/merge-authority-audit.yml`. That expected advisory/control-plane signal must not be disabled. Integration requires current owner authorization plus deterministic validation and independent review.

## Validation

### Focused

- precomputed future gate blob: `e3291fe8fca8fcf70166d5652b43d5a26fa0d762`
- result: immutable Git blob created, not attached to any branch

### Component/integration

- repository policy / governance: PASS locally on material head `0b134215d60bf9c63ef8406f97043d7b09755bbf`
- exact audit diff review: prequalification PASS; final post-dependency review pending

### E2E

- `NOT_APPLICABLE` — audit pin only

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- classification: protected control-plane rotation
- material-head canonical game-gate: SUCCESS in run `33900317317`
- protected-base audit: expected self-modification rejection in run `33900318122`; not disabled or waived
- result: preintegration evidence only; final post-#287 exact-head requalification pending

## Self-review

- exact material head: `0b134215d60bf9c63ef8406f97043d7b09755bbf`
- method/reviewer: writer inspection plus bounded read-only analyst prequalification
- material findings: accepted P1 is repaired; no additional confirmed production defect found
- verdict: WAITING_DEPENDENCY, not final qualification

## Independent review

- required: YES — protected merge authority change
- exact head: pending
- method/auditor: one independent deep review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly three allocated paths per Issue #284
- unresolved review threads: `PRRT_kwDOT8SzxM6fYCU0`, retained until final GREEN and independent review
- protected auto-merge: not enabled by this writer
- merge commit/result: pending
- ownership release: after protected-main readback

## Context checkpoint

```yaml
last_progress: expression P1 repair and exact future-blob compatibility reverified; stale allocation and PR metadata reconciled
status: WAITING_DEPENDENCY
branch: ci/merge-group-audit-pin-284
head_sha: null
pr: 291
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: preintegration_expression_green
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: PR #287 must integrate to protected main before final shared-validator reconciliation; merge is excluded from this session
next_action: after #287 protected-main integration/readback, perform a normal non-force merge-up preserving both validator families, rerun exact-head qualification and one independent deep review, dispose the P1 thread, and return the qualified candidate without merge
```
