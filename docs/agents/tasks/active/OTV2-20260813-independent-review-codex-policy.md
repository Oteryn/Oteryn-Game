# OTV2-20260813-independent-review-codex-policy

```yaml
task_id: OTV2-20260813-independent-review-codex-policy
title: Prefer fresh independent agents and gate Codex recommendations
mode: GOVERNANCE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260813-independent-review-codex-policy
pr: 216
base_sha: c2c692b3b522bcee3c081aba9c8114e4c67fe818
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: architecture-coordinator/current-session
created_at: 2026-08-13T11:48:00+02:00
updated_at: 2026-08-13T13:40:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - AGENTS.md
  - docs/agents/OWNER_FUNDED_AI_POLICY.md
  - docs/agents/tasks/active/OTV2-20260813-independent-review-codex-policy.md
public_contracts: []
depends_on:
  - root AGENTS.md review/audit and owner-funded-AI policy
  - docs/agents/AGENTS.md task-record and governance requirements
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Record the repository-owner preference that genuinely independent review should default to a qualified fresh second agent/session rather than Codex, while preserving every existing review trigger and evidence requirement. When Codex is expected to provide a material efficiency or review-quality advantage, the coordinator must inform the owner and provide a bounded ready-to-run prompt before any Codex invocation; explicit authorization remains required for that exact use.

## Architecture and source of truth

- `PROVEN`: root `AGENTS.md` already requires mandatory self-review and genuinely independent second review for named high-risk classes.
- `PROVEN`: root `AGENTS.md` already forbids owner-funded Codex/OpenAI API use without exact current-use authorization.
- `PROVEN`: `docs/agents/OWNER_FUNDED_AI_POLICY.md` records the same deny-by-default budget rule.
- `PROVEN`: `docs/agents/AGENTS.md` requires active tasks to follow the task template, declare execution budget/ownership/dependencies/blockers, classify validation layers and retain one concrete `next_action`.
- `PROVEN`: owner instruction on 2026-08-13 selects a fresh second agent/session as the default independent-review mechanism and requires the coordinator to recommend Codex with a ready prompt when Codex would be materially more efficient/effective.
- `PROVEN`: open PR #162 has no path overlap with this task's three owned paths.

## Acceptance criteria

- [x] Root governance names a qualified fresh separate agent/session as the default independent-review mechanism when available.
- [x] A fresh reviewer must verify the exact final SHA and governing requirements independently and must not inherit implementing-agent conclusions as trusted facts.
- [x] Codex remains optional and deny-by-default under owner-funded-AI policy.
- [x] When Codex is judged materially more efficient/effective, the coordinator must inform the owner, explain the advantage and provide a bounded ready-to-run prompt before invocation.
- [x] Providing a prompt or recommendation does not authorize Codex; prior permission is non-standing.
- [x] Existing review triggers, safety gates, evidence requirements and repository/write/merge/production/cross-repository authority remain unchanged.
- [x] No runtime, client, protocol, persistence, gameplay, content, Platform or production semantics change.

## Excluded scope

This task must not:

- weaken or waive any existing independent-review requirement;
- make self-review count as independent review;
- create standing permission for Codex/OpenAI API/owner-funded AI use;
- change repository, merge, production, protected-environment or cross-repository authority;
- change architecture/runtime/client/protocol/persistence/gameplay/content semantics;
- change Codex repository settings or automatic review configuration;
- invoke any additional Codex/OpenAI API/owner-funded AI beyond the exact automatic review explicitly authorized by the owner for PR #216 at superseded head `27067c79dcc701244c46931fcf40c1dfdbef9334`.

## Implementation / findings

The policy change is intentionally narrow:

1. root `AGENTS.md` now prefers a qualified fresh separate agent/session as the default independent-review mechanism;
2. root `AGENTS.md` requires Codex recommendation + expected-advantage explanation + bounded prompt before any exact-use authorization;
3. `docs/agents/OWNER_FUNDED_AI_POLICY.md` records the same review-cost/default-review rule and clarifies that a declined optional Codex recommendation is not itself a blocker when another permitted mechanism can satisfy the gate.

A direct squash-merge attempt while PR #216 was a draft was rejected by GitHub with HTTP 405. The owner was informed that moving the PR from draft to ready would automatically trigger the repository's enabled Codex Review and then instructed the coordinator to finish the task. That authorization was limited to the automatic Codex review for PR #216 at exact head `27067c79dcc701244c46931fcf40c1dfdbef9334` and is not standing permission.

The automatic Codex review completed on that superseded head and found one material P1 in this task record only: the active task used an abbreviated schema and omitted template-required execution budget, dependency/blocking metadata, cross-repository coordination, excluded scope and explicit component/E2E classifications. No finding was made against the root review policy or owner-funded-AI policy semantics. This repair completes the task record from the canonical template without changing those policy semantics.

## Validation

### Focused

- preflight live main `c2c692b3b522bcee3c081aba9c8114e4c67fe818`: `PASS`.
- open PR #162 owned-path overlap check: `PASS`, no overlap.
- full changed-file scope before task-record repair: exactly the three owned paths, `behind_by=0`.
- automatic Codex review on superseded exact head `27067c79dcc701244c46931fcf40c1dfdbef9334`: `1 material P1` limited to incomplete task-record schema; policy semantics had no reported finding.
- task-record repair against `docs/agents/tasks/TASK_TEMPLATE.md`: applied; final-head verification pending.

### Component/integration

- command/run: `NOT_APPLICABLE` — governance/documentation-only change with no executable component or integration behavior.
- result: `NOT_APPLICABLE`.

### E2E

- scenario: `NOT_APPLICABLE` — no runtime/client/browser/service behavior changes.
- result: `NOT_APPLICABLE`.

### Exact-head CI

- final head: pending after this task-record repair.
- trigger source: `pull_request`.
- workflow/run/job: Agent Governance / Dependency Review / CodeQL pending for repaired final head.
- runner assignment: pending.
- classification: governance/documentation-only.
- result: pending.

## Self-review

- exact head: pending after this task-record repair.
- method/reviewer: implementing/coordinating agent.
- material findings: pending.
- verdict: pending.

## Independent review

- required: `NO` under the root risk policy because this change reduces no safety gate, expands no authority and weakens no evidence requirement.
- exact head: `NOT_APPLICABLE` for mandatory independent-gate purposes.
- method/auditor: `NOT_APPLICABLE` for mandatory gate. Supplemental owner-authorized automatic Codex review occurred on superseded head `27067c79...` and found the task-record P1 repaired here.
- material findings: `NOT_APPLICABLE` for mandatory gate; supplemental Codex finding count on superseded head: `1`, task-record-only.
- verdict: `NOT_APPLICABLE` for mandatory gate.

## PR and closeout

- changed-file review: pending on repaired final head.
- unresolved review threads: one Codex P1 thread pending exact-head repair verification/resolution.
- related/superseded PRs: open PR #162 checked for overlap; none.
- protected auto-merge: not used.
- merge commit/result: pending.
- ownership release: pending after merge + task archive.

## Context checkpoint

```yaml
last_progress: Owner-authorized automatic Codex Review on superseded head 27067c79 found one P1 limited to missing canonical task-record fields; this commit repairs the task record from TASK_TEMPLATE without changing policy semantics.
status: validating
branch: docs/OTV2-20260813-independent-review-codex-policy
head_sha: null
pr: 216
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_after_task_record_repair
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
owner_action_required: false
blocker: null
next_action: Freeze and self-review the repaired exact three-path head, verify fresh Agent Governance / Dependency Review / CodeQL, resolve the repaired Codex P1 thread with exact-head evidence, then squash-merge PR #216 and archive/release the governance task without triggering any additional Codex use.
```
