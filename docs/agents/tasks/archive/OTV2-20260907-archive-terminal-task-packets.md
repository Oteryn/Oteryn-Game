# OTV2-20260907-archive-terminal-task-packets

> Lifecycle: HISTORICAL / COMPLETED. Delivery PR #376 merged through the normal Merge Queue as `b32ede1a8950463207e39d9b87a14654470c59ce`. Its qualified delivery head was `cd1c8b8ba437a72fd3ee2abef078448c17a15d4c`; [Merge Queue run 34117407066](https://github.com/Oteryn/Oteryn-Game/actions/runs/34117407066) succeeded. The record below preserves implementation history; any former pending fields are superseded by this verified closeout. It does not allocate further work.

```yaml
task_id: OTV2-20260907-archive-terminal-task-packets
title: Archive terminal task packets
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agents/r5-game-d4-374
issue: 374
pr: null
base_sha: a6f69427d663539c6a8e23f166e69147b66ec078
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex / R5 Game W1
created_at: 2026-09-07T08:51:25Z
updated_at: 2026-09-07T08:51:25Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260907-archive-terminal-task-packets.md
  - docs/agents/tasks/active/OTV2-20260826-repair-foundation-terminal-reconciliation.md
  - docs/agents/tasks/active/OTV2-20260828-terminal-session-replacement-allocation.md
  - docs/agents/tasks/active/OTV2-20260904-authority-api-floor.md
  - docs/agents/tasks/active/OTV2-20260904-authority-qualification-governance.md
  - docs/agents/tasks/active/OTV2-20260904-canonical-pr-pg-sim-gate.md
  - docs/agents/tasks/active/OTV2-20260904-merge-group-audit-pin-rotation.md
  - docs/agents/tasks/active/OTV2-20260905-authority-invariant-harness.md
  - docs/agents/tasks/active/OTV2-20260905-authority-recovery-matrix.md
  - docs/agents/tasks/active/OTV2-20260905-fresh-admission-architecture-313.md
  - docs/agents/tasks/active/OTV2-20260905-merge-group-pg-sim-activation.md
  - docs/agents/tasks/active/OTV2-20260905-risk-scoped-test-lanes.md
  - docs/agents/tasks/active/OTV2-20260906-remove-hourly-execution-windows.md
  - docs/agents/tasks/archive/OTV2-20260826-repair-foundation-terminal-reconciliation.md
  - docs/agents/tasks/archive/OTV2-20260828-terminal-session-replacement-allocation.md
  - docs/agents/tasks/archive/OTV2-20260904-authority-api-floor.md
  - docs/agents/tasks/archive/OTV2-20260904-authority-qualification-governance.md
  - docs/agents/tasks/archive/OTV2-20260904-canonical-pr-pg-sim-gate.md
  - docs/agents/tasks/archive/OTV2-20260904-merge-group-audit-pin-rotation.md
  - docs/agents/tasks/archive/OTV2-20260905-authority-invariant-harness.md
  - docs/agents/tasks/archive/OTV2-20260905-authority-recovery-matrix.md
  - docs/agents/tasks/archive/OTV2-20260905-fresh-admission-architecture-313.md
  - docs/agents/tasks/archive/OTV2-20260905-merge-group-pg-sim-activation.md
  - docs/agents/tasks/archive/OTV2-20260905-risk-scoped-test-lanes.md
  - docs/agents/tasks/archive/OTV2-20260906-remove-hourly-execution-windows.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn-Game#367
blocks: []
cross_repository_coordination_id: OTERYN_META_POLICY_ADOPTION_V3
external_repositories: []
```

## Outcome

The active task inventory contains only packets whose owning GitHub issues remain open. Twelve packets owned by terminal issues move unchanged to the archive inventory. The packet for reopened Issue #364 remains active.

## Architecture and source of truth

- PROVEN: GitHub Issue #374 is the bounded task authority.
- PROVEN: protected `main` at admission is `a6f69427d663539c6a8e23f166e69147b66ec078`.
- PROVEN: PR #373 owns the META policy migration and does not change any of the twelve moved packet paths.
- PROVEN: Issue #364 is open with state reason `reopened`, updated at `2026-09-07T08:38:37Z`; its packet remains active.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this change only reconciles Markdown task lifecycle paths; it performs no runtime, production, authority, persistence, recovery, protocol or protected-setting mutation.

## Acceptance criteria

- [x] Confirm the twelve owning GitHub issues are terminal and Issue #364 is open.
- [x] Move the twelve terminal packets from `active/` to `archive/` without changing their contents.
- [x] Pass focused lifecycle tests, governance validation, repository-policy validation and `git diff --check`.
- [x] Publish a Draft PR and obtain exact-head `game-gate` evidence.

## Excluded scope

Provider migration, bootstrap instructions, reusable prompts, consumers, workflows, evidence, runtime code, protected settings and merge actions are excluded.

## Implementation / findings

Issue #374 records the exact twelve source paths and owning terminal issue numbers. The repair is intentionally disjoint from PR #373 and retains the open Issue #364 packet under `active/`.

## Validation

### Focused

- command/run: `python -m unittest discover -s tools/agents/tests -p '*governance_lifecycle*.py' -v`
- result: PASS; 5 tests.

### Component/integration

- command/run: `python tools/agents/validate_governance.py` and `python tools/repository/validate_repository_policy.py`
- result: PASS; governance validated 26 required documents and 9 project lanes; repository policy validated 22 files and 17 workflows.

### E2E

- scenario: NOT_APPLICABLE; documentation lifecycle moves have no runtime path.
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent, full changed-file and diff review
- material findings: Initial inventory included Issue #364; live readback showed it had been reopened, so its packet was retained under `active/` and the authoritative issue/comment count was corrected from thirteen to twelve before publication.
- verdict: PASS after repair; twelve R100 moves and one bounded active task record only.

## Independent review

- required: NO; lifecycle-only Markdown moves are low-risk under the current AI review policy.
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: complete; twelve byte-identical task moves and this task record only
- unresolved review threads: pending
- related/superseded PRs: PR #373 owns the separate META policy migration
- protected auto-merge: disabled; no merge authorized
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: All required local validation passed for the exact bounded lifecycle diff.
status: completed
branch: agents/r5-game-d4-374
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
owner_action_required: null
blocker: null
next_action: Preserve this terminal record; any further work requires its own live task allocation.
```
