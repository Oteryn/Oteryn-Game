# OTV2-YYYYMMDD-short-slug

```yaml
task_id: OTV2-YYYYMMDD-short-slug
title: <short title>
mode: IMPLEMENT | AUDIT | CONTRACT | REPAIR | COORDINATE | MIGRATE | GOVERNANCE
status: investigating | implementing | validating | ready | waiting | blocked | completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: <dedicated branch>
pr: null
base_sha: null
head_sha: null
owner: <agent/session identity>
created_at: <ISO-8601>
updated_at: <ISO-8601>
execution_budget_minutes: 60
large_budget_reason: null
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Describe the observable repository/product result, not only files to edit.

## Architecture and source of truth

List accepted ADRs/contracts and exact external revisions. Label material statements `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT`.

## Acceptance criteria

- [ ] Concrete criterion with named evidence.

## Excluded scope

State what this task must not change or claim.

## Implementation / findings

Maintain concise durable progress and decisions.

## Validation

### Focused

- command/run: pending
- result: pending

### Component/integration

- command/run: pending or `NOT_APPLICABLE` with reason
- result: pending

### E2E

- scenario: pending or `NOT_APPLICABLE` with reason
- result: pending

### Exact-head CI

- head: pending
- workflow/run: pending
- result: pending

## Independent audit

- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: <material event>
status: investigating
branch: <branch>
head_sha: null
pr: null
ci_check_generation: null
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: <exactly one concrete action>
```
