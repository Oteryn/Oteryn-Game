# OTV2-20260824-next-wave-handoff-hardening

```yaml
task_id: OTV2-20260824-next-wave-handoff-hardening
title: Harden next-wave execution handoffs
mode: COORDINATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/next-wave-handoff-hardening-20260824
pr: null
base_sha: 0ab7c7d08b8e532af1633b7aa80a800b1935cf1a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT coordination session
created_at: 2026-08-24T15:37:05Z
updated_at: 2026-08-24T15:37:05Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/superpowers/plans/2026-08-24-oteryn-game-next-wave-master-plan.md
  - docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md
  - docs/agents/prompts/README.md
  - docs/agents/tasks/active/OTV2-20260824-next-wave-handoff-hardening.md
public_contracts: []
depends_on:
  - PR #99 / merge 0ab7c7d08b8e532af1633b7aa80a800b1935cf1a
blocks:
  - unambiguous next-wave worker release
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Harden the merged next-wave master plan so the existing production Server Seam integration task has an explicit reusable executor/alias and Movement cannot be allocated before its #93 hard-max subset is closed. Preserve lane-level parallel release so #95/#97 do not become accidental global blockers.

## Architecture and source of truth

- `PROVEN`: PR #99 merged the master plan at `main@0ab7c7d08b8e532af1633b7aa80a800b1935cf1a`.
- `PROVEN`: `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM` already exists in live allocations as a deferred integration task; this task does not create new architecture authority.
- `PROVEN`: Issue #93 inventories Movement dimensions but the merged master plan did not state an explicit Movement closure step before Movement allocation.
- `PROVEN`: Issue #96 is preparation-only and the merged prompt index had no dedicated Server Seam executor.
- Live GitHub state, root/nearest governance, canonical DAG and live allocations outrank this task packet.

## Acceptance criteria

- [ ] Master plan states that it is orchestration-only and every concrete implementation lane requires a child plan after exact allocation.
- [ ] #93 lifecycle distinguishes early generic-engine release from mandatory Movement-limit closure before Movement allocation.
- [ ] Any unresolved DUR-03 hard-max dimension surfaced by #94 is an explicit pre-Durability blocker.
- [ ] Server Seam has a reusable prompt/alias that is read-only without a merged exact allocation and child plan.
- [ ] Agent Launch Matrix and first coordinator actions model Server Seam as a parallel-capable worker before Client.
- [ ] #95/#97 completeness is explicitly non-blocking for unrelated path-disjoint ready lanes.
- [ ] Prompt index resolves `Oteryn: impl server seam`.
- [ ] `python tools/agents/validate_governance.py` and `git diff --check` pass.
- [ ] Whole-diff self-review and exact-head repository gates including `game-gate` pass.

## Excluded scope

No runtime implementation, protocol/event/state IDs, resource-limit numeric choices, contract/ADR semantics, Cargo/workspace/workflow changes, production access, Platform mutation or external-repository write.

## Implementation / findings

Issue #100 is the governing task. The change narrows execution ambiguity only; it does not create implementation write authority.

Issue #93 was also reconciled in GitHub to state the staged release explicitly: Ability/Interaction/AI may close independently, while Movement remains blocked until every dimension exercised by its exact child plan is registered or excluded fail-closed. No numeric value or runtime authority was added.

Prompt evaluation for `OTV2_IMPL_SERVER_SEAM.md`: PASS against Authority, Resolution, Ownership, Architecture, Completeness, Evidence, Validation, Autonomy, Handover and Safety criteria in `PROMPT_EVAL_STANDARD.md`.

## Validation

### Focused

- command/run: `git diff --cached --check`; `python tools/agents/validate_governance.py`; placeholder scan on the four owned files
- result: PASS — diff check clean; governance 25 required policy documents / 9 lanes PASS; placeholder scan PASS

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation/prompt-only delivery
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime/product mutation
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending until commit freeze; final exact-head verdict will also be recorded on the PR
- method/reviewer: implementing/coordinating agent, complete staged whole-diff review
- material findings: five write-chunk formatting joins found and repaired; no remaining P0/P1/P2 scope, authority, dependency or handoff finding
- verdict: PASS on the current staged tree; repeat/anchor after final commit SHA

## Independent review

- required: NO — docs/prompt clarification only; no authority expansion, accepted architecture change or product/runtime mutation
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: PR #99
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Issue #100 and exact-base branch created; plan/prompt hardening in progress
status: implementing
branch: docs/next-wave-handoff-hardening-20260824
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
next_action: finish the bounded plan/prompt edits and run local validation
```
