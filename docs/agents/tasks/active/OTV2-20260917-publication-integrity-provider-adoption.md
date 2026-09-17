# OTV2-20260917-publication-integrity-provider-adoption

```yaml
task_id: OTV2-20260917-publication-integrity-provider-adoption
title: Adopt protected META publication-integrity authority
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/publication-integrity-adoption-638
pr: null
base_sha: ee4a13212d392dd00f8adcd47b103997687b1d6c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: coordination-agent
created_at: 2026-09-17T05:39:16Z
updated_at: 2026-09-17T05:39:16Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - docs/agents/AGENTS.md
  - tools/agents/tests/test_meta_agent_policy_adoption.py
  - docs/agents/tasks/active/OTV2-20260917-publication-integrity-provider-adoption.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn#212
  - Oteryn/Oteryn#213
blocks: []
cross_repository_coordination_id: PUBLICATION-INTEGRITY-PROVIDER-ROLLOUT
external_repositories:
  - Oteryn/Oteryn
```

## Outcome

Game binds protected META publication-integrity authority `33b212e652c680bd4047be3b414c9a358b8bf26f`, keeps its repository-specific publication fallback fail-closed, and deterministically checks that raw Git Data or per-file reconstruction is not an authorized fallback when normal publication is unavailable.

## Architecture and source of truth

- PROVEN: protected META PR #213 integrated as `Oteryn/Oteryn@33b212e652c680bd4047be3b414c9a358b8bf26f`.
- PROVEN: META rollout order selects Game first because #356 supplied the motivating failure evidence.
- PROVEN: Game protected main at task creation is `ee4a13212d392dd00f8adcd47b103997687b1d6c`.
- DERIVED: existing Game `docs/agents/AGENTS.md` already has the correct fail-closed custody rule; only narrow wording alignment is needed.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this task changes governance binding/documentation/tests only. It performs no production mutation, authority-bearing runtime session replacement, PREPARE/COMMIT operation, controller installation, persistence recovery interpretation, secret use or protected-environment mutation.

## Acceptance criteria

- [ ] Binding points exactly to protected META `33b212e652c680bd4047be3b414c9a358b8bf26f` while retaining policy `3.1.0`.
- [ ] Game publication-safety wording requires fail-closed custody and forbids raw Git Data/per-file reconstruction fallback.
- [ ] `tools.agents.tests.test_meta_agent_policy_adoption` passes with representative publication-integrity coverage.
- [ ] Bound provider validator authenticates the selected META revision.
- [ ] Exact-head required CI and review are clean.
- [ ] Integration uses protected Merge Queue and real merge-group `game-gate`, followed by protected-main readback.

## Excluded scope

No Game runtime/product/Cargo/vendor/protocol/persistence changes. No WP3/#356 source mutation. No workflow/ruleset/protection/production/secret mutation. No direct merge, generic auto-merge, force/rebase/reset or no-op/retrigger commits.

## Implementation / findings

- Updated the provider binding to the protected META publication-integrity authority.
- Narrowly aligned the existing local publication fallback sentence to explicitly include raw Git Data and per-file API reconstruction.
- Added deterministic provider-adoption coverage binding the exact META revision and fail-closed wording.

## Validation

### Focused

- command/run: pending exact-head CI
- result: pending

### Component/integration

- command/run: provider policy validator in Agent Governance
- result: pending

### E2E

- scenario: NOT_APPLICABLE — governance-consumer adoption only; representative behavior is deterministic policy-consumption coverage
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: governance-only
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending under bound META risk policy
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none found at task creation
- protected auto-merge: forbidden substitute; use governed/native Merge Queue
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: provider binding and representative regression prepared
status: validating
branch: governance/publication-integrity-adoption-638
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
next_action: open one Draft PR for exact changed-file review and CI
```
