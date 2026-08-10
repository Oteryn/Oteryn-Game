# OTV2-20260809-architecture-continuation-prompt-refresh

```yaml
task_id: OTV2-20260809-architecture-continuation-prompt-refresh
title: Refresh reusable architecture continuation prompt
mode: GOVERNANCE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/architecture-continuation-prompt-refresh
pr: 118
base_sha: 0dfde85673b985bd00d6f3dcd3690dbf068fdeed
head_sha: fd0abc0c1bdd7b647111245c54fc87c42169e5b7
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6-Sol-session
created_at: 2026-08-09T20:52:00+02:00
updated_at: 2026-08-09T21:07:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/prompts/OTV2_ARCHITECTURE_CONTINUATION_AGENT.md
  - docs/agents/prompts/README.md
  - docs/agents/tasks/active/OTV2-20260809-architecture-continuation-prompt-refresh.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The existing canonical reusable Oteryn-v2 architecture continuation prompt is refreshed rather than duplicated. The owner-supplied prompt is preserved as the detailed baseline; repository-governance, security and current-architecture guidance is additive rather than a condensed replacement. The stable short invocation remains `Oteryn: architektura`.

## Architecture and source of truth

- `PROVEN`: `docs/agents/prompts/OTV2_ARCHITECTURE_CONTINUATION_AGENT.md` is the existing canonical prompt for this purpose on `main`.
- `PROVEN`: repository governance requires a dedicated task/branch/PR for this documentation change.
- `PROVEN`: the owner-supplied prompt requires full startup PR review, architecture-analysis-only runtime gate, detailed architecture/runtime/MMO/network/security checklists, player/producer perspectives, proactive gap detection, AI-maintainable architecture, explicit compatibility, game analytics, accepted-decision recording, a decision backlog, change safety, working style and a detailed START sequence.
- `PROVEN`: the first refresh condensed some explicit owner requirements too aggressively.
- `PROVEN`: the owner requested correction on 2026-08-09; the prompt was rewritten so the detailed owner baseline is retained and new safeguards are additive.
- `PROVEN`: open PR #114 owns only FND-04A architecture/contract/task paths and does not overlap this task.
- `PROVEN`: PR #118 remains scoped to the three owned documentation/task paths.

## Acceptance criteria

- [x] Update the canonical architecture continuation prompt without creating a duplicate prompt file.
- [x] Preserve accepted Oteryn-v2 architecture, trust boundaries and runtime implementation gate.
- [x] Preserve the explicit detail and named criteria from the owner-supplied prompt rather than replacing them with broader summaries.
- [x] Retain explicit `backwards compatibility`, `dependency security`, MMO `balance`, `DECISION BACKLOG`, `WORKING STYLE`, cross-document update checking and the detailed START sequence.
- [x] Keep repository/evidence, protocol/E2E, persistence/recovery, current-foundation and decision-timing improvements as additive safeguards.
- [x] Document one stable short invocation alias: `Oteryn: architektura`.
- [ ] Re-evaluate the final prompt against `docs/agents/PROMPT_EVAL_STANDARD.md` after the owner-requested correction.
- [ ] Run applicable governance/document validation on the final exact head.

## Excluded scope

No runtime code, protocol schema, persistence implementation, architecture decision, production, Platform repository or live-operation change. Do not modify or close unrelated PR #114.

## Implementation / findings

The canonical prompt now explicitly states a preservation rule: the owner-supplied detailed requirements are the baseline, while repository governance and architecture improvements are additive. The original analysis dimensions and named requirements are restored explicitly, including backwards compatibility, dependency security, gameplay balance, decision backlog handling, cross-document consistency checking, working style and the detailed START sequence. Additional current safeguards remain present without replacing those requirements.

## Validation

### Focused

- command/run: owner-baseline-to-current-prompt content review after correction
- result: pending exact-head verification

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation-only prompt/governance change
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable runtime/user-product behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending after this checkpoint commit
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: PR #118 remains scoped to the three declared paths
- unresolved review threads: pending final check
- related/superseded PRs: #114 reviewed; no overlap
- protected auto-merge: not requested
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: owner-requested full-detail correction applied; explicit original requirements restored and enhancements made additive
status: validating
branch: docs/architecture-continuation-prompt-refresh
head_sha: fd0abc0c1bdd7b647111245c54fc87c42169e5b7
pr: 118
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
next_action: verify exact-head diff, owner-baseline coverage and repository checks for PR #118
```
