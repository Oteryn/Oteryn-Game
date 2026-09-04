# OTV2-20260904-merge-group-gate-pin-rotation

```yaml
task_id: OTV2-20260904-merge-group-gate-pin-rotation
title: Rotate protected merge-group gate blob authority
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/merge-group-gate-pin-284
pr: null
issue: 284
parent_issue: 277
prepares_issue: 285
base_sha: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
head_sha: null
final_head_sha: null
final_head_frozen_at: 2026-09-04T14:14:00Z
owner: ChatGPT GPT-5.6 Pro implementation worker
created_at: 2026-09-04T14:14:00Z
updated_at: 2026-09-04T14:14:00Z
execution_budget_minutes: 60
large_budget_reason: exceptional protected-base merge-authority pin rotation
owned_paths:
  - .github/workflows/merge-authority-audit.yml
  - docs/agents/evidence/OTV2-20260904-approved-merge-group-gate.yml
  - docs/agents/tasks/active/OTV2-20260904-merge-group-gate-pin-rotation.md
public_contracts:
  - protected-base approved merge-group gate blob
depends_on:
  - issue: 277
blocks:
  - issue: 285
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The protected-base merge-authority audit approves one exact future Merge Queue workflow blob that adds real PostgreSQL durability and deterministic simulation qualification, without activating the future gate or weakening the audit's self-modification refusal.

## Architecture and source of truth

- `PROVEN` — protected Game `main` at admission is `d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d`.
- `PROVEN` — current protected gate blob is `1e0e7b70a806fe744d394ca8abf43ee434ead3f2`.
- `PROVEN` — exact future gate bytes were created as inert Git blob `130f7fa876383ec41457ed81e0f54be6f6a79c34`.
- `PROVEN` — Issue #284 explicitly authorizes an owner-controlled audit rotation and requires the audit's self-modification failure to remain visible.
- `DERIVED` — committing the exact future bytes as a documentation evidence path gives reviewers an inspectable artifact whose Git blob identity must equal the new pin regardless of path.
- `PROVEN` — this task does not modify `.github/workflows/merge-group-gate.yml`, so it does not activate the future policy.

## High-risk authority/recovery qualification

```yaml
applicable: CONTROL_PLANE_AUTHORITY_ROTATION
authority_invariants:
  - protected_base_audit_remains_non_candidate_controlled
  - exactly_one_future_gate_blob_is_approved
  - future_gate_adds_pg_and_sim_without_new_write_authority
consumer_boundaries:
  - pull_request_target_protected_base_audit
  - future_merge_group_gate_activation
mutation_operators:
  - change_only_expected_gate_blob_pin
independent_current_fact_sources:
  - protected_main_audit_workflow
  - immutable_git_blob_identity
record_derived_matching_helper:
  allowed_for_positive_happy_path: NOT_APPLICABLE
  forbidden_for_negative_authority_or_provenance_cases: NOT_APPLICABLE
finding_family_sweep:
  sibling_apis: audit_pin_and_future_gate_evidence
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: audit_rotation_then_exact_gate_activation
  restart_retry_replay_concurrency_pg_reload: NOT_APPLICABLE
  evidence:
    - future_gate_blob_130f7fa876383ec41457ed81e0f54be6f6a79c34
finding_dispositions:
  p0_p1_verified_repair_or_rejection: pending_independent_review
  p2_fixed_accepted_or_deferred: pending_independent_review
```

## Acceptance criteria

- [x] Future gate is represented by one exact lowercase 40-hex Git blob SHA.
- [x] Evidence file bytes are exactly the future gate blob bytes and therefore have blob SHA `130f7fa876383ec41457ed81e0f54be6f6a79c34`.
- [x] Audit change is limited to replacing `1e0e7b70...` with `130f7fa8...`.
- [x] Audit trigger, permissions, exact-head/same-repository checks, candidate-code non-checkout and forbidden-behavior checks remain unchanged.
- [x] Future gate preserves seven job keys and unchanged `game-gate` fan-in.
- [x] Future Linux job verifies exact synthetic head and runs PostgreSQL 17.6 durability.
- [x] Future Windows job verifies exact synthetic head and runs deterministic simulation golden fixtures.
- [x] Future gate has no `continue-on-error`, workflow dispatch, `pull_request_target`, write permission or unpinned action.
- [ ] Exact three-path diff and blob identities pass authoritative GitHub readback.
- [ ] One independent deep review has no unresolved actionable finding.
- [ ] Explicit current owner-controlled integration decision is supplied; this worker does not merge.

## Excluded scope

No activation of `merge-group-gate.yml`, no PR gate/risk classifier, no ruleset, required-status, runtime, test-source, production, secret or external-repository mutation. Do not suppress, bypass or relabel the expected protected-base audit self-modification failure.

## Implementation / findings

- Replaced only the protected audit's exact approved gate blob value.
- Added the future gate as inert evidence under `docs/agents/evidence/`; its content-addressed blob is the same object that Issue #285 must later activate unchanged.
- Future gate keeps current candidate/governance, dependency review, CodeQL, Linux, Windows, supply-chain and `game-gate` jobs. It adds PG/SIM within the existing required platform jobs, so the fan-in shape does not change.
- The protected-base audit is expected to report `candidate modifies the protected-base audit itself`; that outcome is a designed authority boundary, not a product failure and not a PASS.
- The exact candidate commit is established by authoritative branch/PR readback after publication and is not self-embedded.

## Validation

### Focused

- check: audit diff contains only the exact pin replacement
- result: pending authoritative PR diff
- check: evidence file blob identity
- result: expected `130f7fa876383ec41457ed81e0f54be6f6a79c34`
- check: future workflow top-level keys, seven jobs, exact SHA steps, PG/SIM commands, pinned actions, forbidden tokens
- result: PASS before publication

### Component/integration

- command/run: candidate repository policy and canonical `game-gate`
- result: pending exact head; protected-base audit expected to fail closed on self-modification

### E2E

- scenario: `NOT_APPLICABLE` — this rotation does not activate or execute the future merge-group workflow
- result: NOT_APPLICABLE

### Exact-head CI

- final head: established by authoritative GitHub readback
- trigger source: Draft PR pull_request and pull_request_target
- expected ordinary result: canonical `game-gate` success if unaffected checks pass
- expected exceptional result: protected-base merge-authority audit failure identifying candidate self-modification

## Self-review

- exact head: pending authoritative readback
- method/reviewer: implementing agent, exact audit/evidence/task diff
- material findings: none before publication
- verdict: PASS_PENDING_REMOTE_READBACK

## Independent review

- required: YES — protected-base control-plane authority rotation
- exact head: pending
- method/auditor: one independent deep review plus explicit human-owner integration decision
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly three allocated paths
- unresolved review threads: pending
- protected auto-merge: prohibited/not requested
- merge commit/result: explicit owner-controlled integration only
- ownership release: after protected-main readback and subsequent #285 activation handoff

## Context checkpoint

```yaml
last_progress: exact future gate blob and minimal protected pin rotation prepared
status: validating
branch: governance/merge-group-gate-pin-284
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: 2026-09-04T14:14:00Z
ci_trigger_source: pull_request_and_pull_request_target
ci_check_generation: pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: github_hosted_pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: explicit_current_owner_controlled_integration_decision_after_review
blocker: protected_base_audit_self_modification_refusal_expected_by_design
next_action: publish the exact three-path Draft rotation PR, verify audit one-line diff and evidence blob identity, obtain one independent deep review, then hand off for explicit owner-controlled integration without bypass or self-merge
```
