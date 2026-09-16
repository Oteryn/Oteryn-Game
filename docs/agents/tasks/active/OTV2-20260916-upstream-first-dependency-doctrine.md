# OTV2-20260916-upstream-first-dependency-doctrine

```yaml
task_id: OTV2-20260916-upstream-first-dependency-doctrine
title: Establish repository-wide upstream-first dependency doctrine
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/upstream-first-dependency-doctrine-20260916
pr: null
base_sha: 82534b2d33550b2a5ff1ef923526cb6dd8d30673
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-09-16T09:24:00+02:00
updated_at: 2026-09-16T09:24:00+02:00
execution_policy: continuous_progress
owned_paths:
  - AGENTS.md
  - CONTRIBUTING.md
  - docs/repository/UPSTREAM_FIRST_DEPENDENCY_POLICY.md
  - docs/agents/tasks/active/OTV2-20260916-upstream-first-dependency-doctrine.md
public_contracts:
  - docs/repository/UPSTREAM_FIRST_DEPENDENCY_POLICY.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Establish one repository-wide rule that mature upstream implementations are the default and that forks, vendored modifications, deep dependency instrumentation or local reimplementations require concrete evidence of a real accepted requirement gap. Preserve the ability to introduce the smallest necessary exception when upstream genuinely cannot meet an accepted Oteryn invariant.

## Architecture and source of truth

- `PROVEN`: root `AGENTS.md` is the always-loaded Game bootstrap for repository work.
- `PROVEN`: `CONTRIBUTING.md` is the repository contribution workflow for human and tool-assisted changes.
- `PROVEN`: bound META policy requires one rule/one authority and permits provider-local durable product constraints.
- `DERIVED`: a single canonical policy under `docs/repository/` plus short references from the bootstrap/contribution guide avoids independent duplicate policy prose.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: documentation/governance only. No production mutation, authority-bearing session, PREPARE/COMMIT path, persisted recovery interpretation, protocol change or credential action.

## Acceptance criteria

- [x] Define `UPSTREAM_FIRST / PATCH_ON_PROVEN_NEED` as the repository default.
- [x] Require concrete evidence against an exact upstream version before dependency customization.
- [x] Define the decision order from upstream configuration through minimal patch to last-resort fork.
- [x] Require minimal, provenance-pinned, regression-tested and removable downstream patches.
- [x] Require representative measurement for performance-based exceptions and concrete threat/reproducer evidence for security-based exceptions.
- [x] State that accepted correctness/security invariants must not be silently weakened merely to remove a patch.
- [x] Apply the doctrine to agents through root `AGENTS.md`.
- [x] Apply the doctrine to contributors through `CONTRIBUTING.md`.
- [x] State that existing forks are re-evaluated rather than automatically grandfathered, while preserving useful history/tests/research.

## Excluded scope

- No WP3 source or dependency implementation change.
- No removal of existing forks or vendor trees in this task.
- No architecture acceptance or threat-model change.
- No workflow, ruleset, Merge Queue, production, deployment, secret or external-repository mutation.
- No claim that every dependency must remain unmodified; proven minimal exceptions remain allowed.

## Implementation / findings

The policy explicitly prevents both extremes: forcing custom solutions without evidence and forcing pure upstream by weakening a real Oteryn invariant. The preferred resolution order is upstream configuration/API, Oteryn-owned adapter, upstream contribution where practical, minimal downstream patch, and only then a maintained fork.

Existing dependency customizations remain historical/current implementation evidence; when touched or superseded they must be reassessed under the new doctrine rather than deleted automatically.

## Validation

### Focused

- changed paths and policy cross-references: pending exact-head readback
- result: pending

### Component/integration

- `NOT_APPLICABLE`: documentation/governance policy only; no runtime component changed.

### E2E

- `NOT_APPLICABLE`: no product behavior changed.

### Exact-head CI

- final head: pending
- trigger source: pull request
- workflow/run/job: pending
- runner assignment: pending
- classification: documentation/governance
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent whole-diff policy/duplication/scope review
- material findings: pending
- verdict: pending

## Independent review

- required: pending
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: WP3 audit evidence PR #633 is related evidence only and is not superseded
- protected auto-merge: not requested by this task
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: repository-wide upstream-first dependency doctrine authored
status: validating
branch: governance/upstream-first-dependency-doctrine-20260916
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
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
```
