# OTV2-20260827-terra-sol-selector-handoff-repair

```yaml
task_id: OTV2-20260827-terra-sol-selector-handoff-repair
title: Repair selector-resolved Terra/Sol handoffs and reconcile terminal evidence
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/terra-sol-selector-handoff-repair-217
issue: 217
pr: null
base_sha: a36513fa12725b1fd4abfdf18a4891ddc6021b85
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol repair session
created_at: 2026-08-27T16:04:25+02:00
updated_at: 2026-08-27T16:04:25+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md
  - docs/agents/prompts/OTV2_SOL_SUPERVISING_ARCHITECT.md
  - docs/agents/tasks/archive/OTV2-20260827-terra-sol-parallel-execution.md
  - docs/agents/tasks/active/OTV2-20260827-terra-sol-selector-handoff-repair.md
public_contracts: []
depends_on:
  - issue:217
  - pr:214 merge:6a062bf05a91461abd7c79a9761f3b58605e1cb3
  - pr:215 merge:a36513fa12725b1fd4abfdf18a4891ddc6021b85
  - issue:162 remains Work-controlled
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Restore the Terra + Sol package's single-active-control-plane invariant without changing runtime authority: reusable Sol Durability and Supervising Architect handoffs resolve the uniquely active control-plane profile from the current coordinator Issue/task instead of hard-coding Terra, and the archived #213 task truthfully distinguishes historical pre-merge PASS evidence from the post-merge P1 requiring this correction.

## Architecture and source of truth

- `PROVEN`: protected admission `main` is `a36513fa12725b1fd4abfdf18a4891ddc6021b85`.
- `PROVEN`: Issue #162 remains open and explicitly selects `OTV2_WORK_DELIVERY_COORDINATOR`; no durable Work-to-Terra transfer exists.
- `PROVEN`: canonical Terra, Work and scheduler policies require exactly one mutating control-plane profile and make the inactive profile `RECOVERY_READ_ONLY`.
- `PROVEN`: post-merge independent review `5040825913` and PR #214 comments `5439248113` / `5439289752` found that Durability and Supervising Architect still hard-coded Terra in integration/lane-state handoffs.
- `DERIVED`: the root cause is not the selector itself; it is two reusable Sol prompts bypassing that selector in their terminal handoff wording.
- `PROVEN`: Issue #167 and draft PR #212 remain runtime implementation work and are outside this repair's owned paths.

## Acceptance criteria

- [x] Durability integration handoff resolves the uniquely active control-plane profile and fails closed as `POLICY_CONFLICT` when uniqueness is not proven.
- [x] Supervising Architect mission/evidence/return wording is selector-resolved and does not route Work-controlled lifecycles through inactive Terra.
- [x] Original #213 archive preserves historical evidence but marks its current terminal-clean verdict superseded pending this corrective lifecycle.
- [ ] PR number and complete changed-file set are frozen before exact-head review.
- [ ] Agent governance, Architecture semantic audit, Merge authority audit and Merge gate/game-gate pass on the unchanged final head.
- [ ] Mandatory whole-diff author self-review returns P0/P1/P2=0/0/0 on the exact final head.
- [ ] Genuinely independent non-authoring exact-head review returns PASS with P0/P1/P2=0/0/0.
- [ ] Zero unresolved review threads and current-main compatibility are proven immediately before expected-head squash merge.
- [ ] Post-merge lifecycle closeout archives this corrective task and updates the historical archive's current terminal verdict only from external exact-head evidence.

## Excluded scope

No runtime/product, Cargo/workspace, protocol/schema/registry, production/protected-environment, secret, live-data, #167/#212 branch/history, Platform/Atlas/META or external-repository mutation. This repair does not transfer #162 from Work to Terra.

## Implementation / findings

The repair changes only the selector bypass and its stale terminal evidence. No unrelated prompt wording, architecture semantics or lane allocation is widened.

The original pre-merge review records remain historical facts. They are not deleted or rewritten; the archive now explicitly records that a later current-state review superseded their terminal-clean interpretation until Issue #217 is qualified and merged.

## Validation

### Focused

- command/run: exact textual review against current #162 single-active selector semantics
- result: PASS for the implemented wording; external exact-head repository validation pending PR publication

### Component/integration

- command/run: repository governance/architecture/authority workflows
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — governance/prompt/evidence-only repair; no executable runtime outcome
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: GitHub-hosted policy
- classification: governance-only corrective repair
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing repair session; full changed-file review against Issue #217 and post-merge P1 evidence
- material findings: pending
- verdict: pending

## Independent review

- required: YES — this repair changes execution-authority routing and corrects a previously terminal governance package
- exact head: pending
- method/auditor: separate non-authoring reviewer/session
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #214 historical delivery; #215 historical closeout; #216 redundant closed-not-merged
- protected auto-merge: disabled/not requested
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: selector-resolved Durability and Architect handoffs implemented; original #213 archive reconciled as SUPERSEDED_PENDING_CORRECTION
status: validating
branch: docs/terra-sol-selector-handoff-repair-217
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
next_action: open the corrective PR for Issue #217, then freeze PR metadata and exact changed paths before final-head qualification
```
