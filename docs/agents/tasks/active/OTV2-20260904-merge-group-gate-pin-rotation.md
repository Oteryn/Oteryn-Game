# OTV2-20260904-merge-group-gate-pin-rotation

```yaml
task_id: OTV2-20260904-merge-group-gate-pin-rotation
title: Rotate protected merge-group gate blob authority
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/merge-group-gate-pin-284
pr: 288
issue: 284
parent_issue: 277
prepares_issue: 285
base_sha: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
head_sha: null
final_head_sha: null
final_head_frozen_at: 2026-09-04T14:31:00Z
owner: ChatGPT GPT-5.6 Pro implementation worker
created_at: 2026-09-04T14:14:00Z
updated_at: 2026-09-04T14:31:00Z
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

The protected-base merge-authority audit approves one exact future Merge Queue workflow blob that adds deterministic Windows simulation and deletion-safe PostgreSQL 17.6 qualification. This task publishes the exact future bytes as inert evidence but does not activate the future gate or weaken the audit's self-modification refusal.

## Architecture and source of truth

- `PROVEN` — protected Game `main` at admission is `d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d`.
- `PROVEN` — current protected merge-group gate blob is `1e0e7b70a806fe744d394ca8abf43ee434ead3f2`.
- `PROVEN` — initial future blob `130f7fa876383ec41457ed81e0f54be6f6a79c34` added unconditional PG and was superseded before review after PR-gate execution proved that protected main does not yet contain the `durability_postgres` target.
- `PROVEN` — final future gate bytes are Git blob `16edc91ce969366640ba8bc82f224d8d11b1965f`.
- `PROVEN` — Issue #284 authorizes an owner-controlled audit rotation and requires the audit's self-modification failure to remain visible.
- `DERIVED` — exact base-SHA target presence is the correct Merge Queue discriminator: candidate target present runs PG; target absent from head but present on protected base fails; absent from both base and head is explicit `NOT_APPLICABLE`.
- `PROVEN` — this task does not modify `.github/workflows/merge-group-gate.yml`, so it does not activate the future policy.

## High-risk authority/recovery qualification

```yaml
applicable: CONTROL_PLANE_AUTHORITY_ROTATION
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - protected_base_audit_remains_non_candidate_controlled
  - exactly_one_future_gate_blob_is_approved
  - future_gate_runs_simulation
  - future_gate_runs_allocated_postgresql_target
  - future_gate_cannot_drop_a_base_postgresql_target_into_skip
  - historical_target_absence_is_not_reported_as_postgresql_pass
consumer_boundaries:
  - pull_request_target_protected_base_audit
  - future_merge_group_gate_activation
mutation_operators:
  - change_only_expected_gate_blob_pin
  - remove_or_rename_postgresql_target_from_synthetic_head
independent_current_fact_sources:
  - protected_main_audit_workflow
  - immutable_git_blob_identity
  - exact_merge_group_base_sha
  - exact_merge_group_head_checkout
record_derived_matching_helper:
  allowed_for_positive_happy_path: NOT_APPLICABLE
  forbidden_for_negative_authority_or_provenance_cases: NOT_APPLICABLE
finding_family_sweep:
  sibling_apis: PR gate and merge-group gate PostgreSQL routing
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: audit_rotation_then_exact_gate_activation
  restart_retry_replay_concurrency_pg_reload: allocated durability_postgres target
  evidence:
    - PR 287 run 33881858954 job 101052200653
    - superseded future blob 130f7fa876383ec41457ed81e0f54be6f6a79c34
    - final future blob 16edc91ce969366640ba8bc82f224d8d11b1965f
finding_dispositions:
  p0_p1_verified_repair_or_rejection: initial_unconditional_pg_design_superseded_and_repaired
  p2_fixed_accepted_or_deferred: pending_independent_review
```

## Acceptance criteria

- [x] Future gate is represented by one exact lowercase 40-hex Git blob SHA.
- [x] Evidence file bytes are exactly the final future gate blob bytes and therefore have blob SHA `16edc91ce969366640ba8bc82f224d8d11b1965f`.
- [x] Audit change from protected main is limited to replacing `1e0e7b70...` with `16edc91c...`.
- [x] Audit trigger, permissions, exact-head/same-repository checks, candidate-code non-checkout and forbidden-behavior checks remain unchanged.
- [x] Future gate preserves seven job keys and unchanged `game-gate` fan-in.
- [x] Future Linux job verifies exact synthetic head, resolves target presence from exact protected base and uses deletion-safe PostgreSQL 17.6 routing.
- [x] Future Windows job verifies exact synthetic head and runs deterministic simulation golden fixtures.
- [x] Future gate has no `continue-on-error`, workflow dispatch, `pull_request_target`, write permission or unpinned action.
- [ ] Exact three-path diff and blob identities pass authoritative GitHub readback.
- [ ] Ordinary exact-head repository validation is green or has only the expected protected-base self-modification refusal.
- [ ] One independent deep review has no unresolved actionable finding.
- [ ] Explicit current owner-controlled integration decision is supplied; this worker does not merge.

## Excluded scope

No activation of `merge-group-gate.yml`, no PR gate/risk classifier, no ruleset, required status, runtime, test-source, production, secret or external-repository mutation. Do not suppress, bypass or relabel the expected protected-base audit self-modification failure.

## Implementation / findings

### Superseded pre-review design

- Commit `e6341f282e13218fdd5ed22284e7caecdade4c17` pinned inert future blob `130f7fa8...`.
- Family sweep from PR #287 revealed that unconditional `cargo ... --test durability_postgres` fails on revisions before the target is integrated.
- No independent review had been requested on that generation; it is superseded, not qualified.

### Final candidate design

- Protected audit pin points only to `16edc91ce969366640ba8bc82f224d8d11b1965f`.
- The evidence path contains those exact bytes.
- Future merge-group Linux verifies exact head and queries the exact `base_sha` for target presence.
- Head target present → real PG E2E; base present/head absent → fail closed; base/head absent → explicit `NOT_APPLICABLE`.
- Windows exact-head SIM and all existing candidate, dependency, CodeQL, Linux, Windows client, supply-chain and aggregate semantics remain.
- Protected audit still rejects any candidate that modifies the audit itself. This expected result is authority evidence, not a condition to bypass.
- Final commit identity is established by authoritative branch/PR readback after publication rather than self-embedded.

## Validation

### Focused

- check: audit diff from protected main contains only one pin replacement
- result: pending final authoritative diff
- check: evidence file blob identity
- result: expected `16edc91ce969366640ba8bc82f224d8d11b1965f`
- check: future workflow top-level keys, seven jobs, exact SHA steps, base/head target routing, PG/SIM commands, pinned actions and forbidden tokens
- result: PASS before publication

### Component/integration

- command/run: candidate repository policy and canonical `game-gate`
- result: pending exact final head; protected-base audit self-modification refusal expected

### E2E

- scenario: `NOT_APPLICABLE` — this rotation publishes inert bytes and does not execute the future merge-group workflow
- result: NOT_APPLICABLE

### Exact-head CI

- final head: established by authoritative GitHub readback
- trigger source: Draft PR #288 `pull_request` and `pull_request_target`
- expected ordinary result: applicable repository checks execute on exact head
- expected exceptional result: protected-base merge-authority audit fails with candidate-self-modification refusal

## Self-review

- exact head: pending authoritative readback
- method/reviewer: implementing agent exact audit/evidence/task diff
- material findings: initial unconditional PG compatibility defect found by family sweep and repaired before review
- verdict: PASS_PENDING_REMOTE_READBACK

## Independent review

- required: YES — protected-base control-plane authority rotation
- exact head: pending final candidate readback
- method/auditor: one independent deep review plus explicit human-owner integration decision
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: exactly three allocated paths
- unresolved review threads: pending
- protected auto-merge: prohibited/not requested
- merge commit/result: explicit owner-controlled integration only
- ownership release: after protected-main readback and #285 activation handoff

## Context checkpoint

```yaml
last_progress: family sweep superseded unconditional future PG and produced exact deletion-safe future gate blob
status: validating
branch: governance/merge-group-gate-pin-284
head_sha: null
pr: 288
final_head_sha: null
final_head_frozen_at: 2026-09-04T14:31:00Z
ci_trigger_source: pull_request_and_pull_request_target_after_final_publication
ci_check_generation: final_pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: github_hosted_pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: explicit_current_owner_controlled_integration_decision_after_review
blocker: protected_base_audit_self_modification_refusal_expected_by_design
next_action: publish the final three-path rotation candidate, verify one-line audit diff and exact evidence blob identity, obtain one independent deep review, then hand off for explicit owner-controlled integration without bypass or self-merge
```
