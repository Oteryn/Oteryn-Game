# OTV2-20260827-autonomous-codex-review-loop

```yaml
task_id: OTV2-20260827-autonomous-codex-review-loop
title: Authorize autonomous GitHub Codex review loop
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/autonomous-codex-review-loop-20260827
pr: null
base_sha: 4b6656f688868aa2fb59c18392c2f859f1c5a1c7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: governance-authoring-session
created_at: 2026-08-27T21:55:30Z
updated_at: 2026-08-27T21:55:30Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/OWNER_FUNDED_AI_POLICY.md
  - docs/agents/CODEX_REVIEW_POLICY.json
  - docs/agents/tasks/active/OTV2-20260827-autonomous-codex-review-loop.md
  - docs/superpowers/specs/2026-08-27-oteryn-game-autonomous-codex-review-loop-design.md
  - docs/superpowers/plans/2026-08-27-oteryn-game-autonomous-codex-review-loop.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

An allocated lane lead can autonomously request and consume a fresh independent exact-head Codex review through the canonical GitHub PR without requiring the owner to relay prompts or approve every covered review invocation. The lane lead retains repair/re-review ownership until required review passes, while Terra/Work mechanically verify the resulting exact-head evidence.

## Architecture and source of truth

- `PROVEN`: protected admission main is `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`.
- `PROVEN`: root `AGENTS.md` currently requires explicit owner authorization for every owner-funded AI invocation.
- `PROVEN`: `docs/agents/OWNER_FUNDED_AI_POLICY.md` currently forbids standing authorization.
- `PROVEN`: official Codex GitHub code review supports PR review and explicit `@codex review` requests when enabled/configured; repository capability still must be proven at actual invocation time.
- `DERIVED`: without a standing authorization, the owner remains a manual relay in every Codex review loop.
- `PROVEN`: Issue #229 records explicit owner scope for this governance change.

## Acceptance criteria

- [x] Machine-readable review policy defines exact standing-authorized and prohibited operations.
- [x] Risk matrix defines deterministic `CODEX_REQUIRED` classes.
- [x] Fresh non-authoring reviewer and exact-head re-review rules are explicit.
- [ ] Root governance recognizes the standing authorization only after protected-main merge.
- [ ] Nearer docs-agent policy does not conflict with covered review invocations.
- [ ] Owner-funded AI policy preserves per-invocation approval for every non-covered AI use.
- [ ] Lane lead owns candidate -> Codex review -> repair -> fresh review loop without owner relay.
- [ ] Terra/Work use deterministic policy evidence and do not adjudicate technical findings.
- [ ] Exact-head governance/architecture/merge-authority/merge-gate checks pass.
- [ ] Whole-diff author self-review has no blocking finding.
- [ ] Genuinely independent non-authoring exact-head review has no P0/P1 finding.
- [ ] Zero unresolved review threads before merge.

## Excluded scope

No runtime, Cargo/workspace, protocol/schema/registry, production/protected environment, secret, live-data, external-repository or branch-protection mutation. No Codex invocation is authorized by the unmerged branch itself. No implementation or merge authority is granted to a Codex reviewer.

## Implementation / findings

Issue #229 and branch are active. The policy/spec/plan are committed. Normative root/docs-agent authorization edits remain in progress.

## Validation

### Focused

- command/run: pending GitHub governance validation
- result: pending

### Component/integration

- command/run: governance/architecture/authority exact-head workflows
- result: pending

### E2E

- scenario: NOT_APPLICABLE — governance/docs only
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: governance
- result: pending

## Self-review

- exact head: pending
- method/reviewer: governance-authoring-session
- material findings: pending
- verdict: pending

## Independent review

- required: YES — owner-funded AI authority-policy expansion/change
- exact head: pending
- method/auditor: genuinely independent non-authoring reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #214 execution architecture; #223 auditor evidence-write authority
- protected auto-merge: disabled until independent exact-head review
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: policy/spec/plan/task package created
status: implementing
branch: docs/autonomous-codex-review-loop-20260827
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
next_action: update normative root and docs-agent owner-funded AI authorization rules
```
