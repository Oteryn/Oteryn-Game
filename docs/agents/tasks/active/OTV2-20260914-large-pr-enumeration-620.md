# OTV2-20260914-large-pr-enumeration-620

```yaml
task_id: OTV2-20260914-large-pr-enumeration-620
title: Recover exact large-PR lane enumeration from immutable Git trees
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/large-pr-enumeration-620
pr: null
issue: 620
base_sha: 775a09091743af395ecb8f1e440cb9c286bc0dd2
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT sole writer
created_at: 2026-09-14T18:25:44Z
updated_at: 2026-09-14T18:32:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - docs/agents/tasks/active/OTV2-20260914-large-pr-enumeration-620.md
public_contracts:
  - Trusted-base dependency-aware fail-closed PR test-lane selection
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A PR whose transported changed-file enumeration is incomplete because of API/output-size bounds is re-enumerated from the already verified protected-base checkout and exact fetched PR head. File count alone no longer forces FULL, while ambiguous Git evidence still fails closed.

## Architecture and source of truth

- PROVEN: protected `main@775a09091743af395ecb8f1e440cb9c286bc0dd2` contains the canonical trusted-base classifier and merge gate.
- PROVEN: run `34879663102`, PR #356, reported `CHANGED_FILE_COUNT=944`, `ENUMERATION_COMPLETE=false`, and classifier reason `incomplete-enumeration`.
- PROVEN: Issue #283 contract requires unknown/mixed/incomplete evidence to fail closed and permits reduced lanes only from trusted protected-base classification.
- DERIVED: when transported enumeration is incomplete, a complete `git diff` between the verified base tree at `HEAD` and immutable exact head can replace the transport evidence without weakening candidate-mode or consumer-snapshot checks.

## High-risk authority/recovery qualification

NOT_APPLICABLE — this change selects CI test lanes only and performs no production mutation, authority handoff, persistence recovery, PREPARE/COMMIT, controller restoration or live-data operation.

## Acceptance criteria

- [ ] Existing complete transported enumeration remains unchanged.
- [ ] `ENUMERATION_COMPLETE=false` recovers records from exact immutable Git trees without a 300-file or 32 KiB transport ceiling.
- [ ] More than 300 server-only Rust paths can still prove `rust=true`, `windows=false` when all existing dependency/input invariants pass.
- [ ] A client/shared path still selects Windows/FULL.
- [ ] Invalid enumeration state, malformed Git output, unsupported Git status, missing objects or special candidate modes fail closed.
- [ ] Existing PR and post-merge classifier regressions pass.
- [ ] Repository policy/governance and exact-head hosted Merge gate pass.
- [ ] No workflow, ruleset, required status, Merge Queue, Cargo, runtime or product behavior changes.

## Excluded scope

No mutation of PR #356. No cache/sccache rollout. No relaxation of Cargo/control-plane/shared-consumer FULL semantics. No workflow, ruleset, protected setting, required-status or merge-authority change.

## Implementation / findings

The minimal implementation is intentionally classifier-local: retain the current scope transport for small PRs; only when it reports `ENUMERATION_COMPLETE=false`, ignore the truncated/empty records and enumerate the complete base-vs-exact-head tree delta with Git `--no-renames --name-status -z`. Renames therefore remain conservative delete+add path evidence. Existing candidate tree-mode validation, audited input snapshots and dependency reverse-closure classification remain authoritative.

## Validation

### Focused

- command/run: isolated Python syntax check plus real temporary Git repository with 301 server-only Rust additions, then one client addition, malformed Git status and invalid enumeration-state negatives
- result: PASS before repository mutation

### Component/integration

- command/run: pending exact branch validation
- result: pending

### E2E

- scenario: hosted merge-gate classification on exact PR head
- result: pending

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: expected FULL because classifier/control-plane files change
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent whole-diff review
- material findings: pending
- verdict: pending

## Independent review

- required: pending under `docs/governance/AI_REVIEW_POLICY.md`
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: pending authority/capability resolution
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: issue #620 and dedicated branch created; focused >300 Git-tree fallback prototype passes
status: implementing
branch: ci/large-pr-enumeration-620
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
next_action: commit classifier and regression coverage, then run exact branch validation
```
