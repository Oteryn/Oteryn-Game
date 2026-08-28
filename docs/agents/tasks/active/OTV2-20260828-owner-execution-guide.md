# OTV2-20260828-owner-execution-guide

```yaml
task_id: OTV2-20260828-owner-execution-guide
title: Canonicalize owner agent launch and status guidance
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/owner-execution-guide-20260828
issue: 253
pr: null
base_sha: 12d4ca5326d62a7a2c46d80cd5e167e99f109d1d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-execution-guide-author-session
created_at: 2026-08-28T22:56:24+02:00
updated_at: 2026-08-28T22:56:24+02:00
execution_budget_minutes: 90
large_budget_reason: cross-prompt operational clarification and canonical owner runbook
owned_paths:
  - docs/agents/programs/OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md
  - docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/prompts/OTV2_OWNER_EXECUTION_STATUS_ADVISOR.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/prompts/OTV2_TERRA_GAME_CONTROL_PLANE.md
  - docs/agents/prompts/OTV2_SOL_SUPERVISING_ARCHITECT.md
  - docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md
  - docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md
  - docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md
  - docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/tasks/active/OTV2-20260828-owner-execution-guide.md
public_contracts: []
depends_on:
  - issue: 253
  - merged_pr: 230
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Give the owner and all active execution entry profiles one canonical operational view of aliases, model/effort selection, Work-vs-chat placement, lane launch order, autonomous Codex review, Work Auditor evidence, and live GitHub status reconciliation. This package clarifies existing authority only; it must not create a new control plane, lane allocation, merge authority or owner-funded AI permission.

## Architecture and source of truth

- `PROVEN`: protected admission `main` is `12d4ca5326d62a7a2c46d80cd5e167e99f109d1d`.
- `PROVEN`: PR #230 is merged and `docs/agents/CODEX_REVIEW_POLICY.json` is canonical.
- `PROVEN`: Issue #253 records owner authorization for this clarification package.
- `PROVEN`: root `AGENTS.md` remains the authority source for the standing Codex review authorization and single active control-plane behavior.
- `DERIVED`: duplicating only the operational steps needed at prompt entry points reduces conservative false-blocking without widening authority.

## Acceptance criteria

- [ ] Canonical runbook lists aliases, recommended surface/model/effort, mutation mode and launch order.
- [ ] Runbook defines fresh-GitHub classification of `DONE`, `ACTIVE`, `BLOCKED`, `READY_NEXT` and `DO_NOT_LAUNCH`.
- [ ] New reusable owner execution/status advisor is read-only, live-GitHub-first and returns exact Work/chat launch instructions plus exactly one next action.
- [ ] Work/Terra prompts explicitly read the runbook and Codex policy, mechanically verify routing/evidence and do not replace lane-lead review ownership.
- [ ] Sol Supervising Architect explicitly supports canonical risk classification where authorized without taking over review/implementation/merge.
- [ ] Five Sol VSL leads explicitly own policy-required `freeze -> @codex review -> repair -> fresh re-review` and expose Codex evidence in handoff.
- [ ] Work Auditor verifies Codex evidence but does not become a nested Codex dispatcher or implementation worker.
- [ ] Prompt README, scheduler and lifecycle registry align with the new owner execution/status advisor and runbook.
- [ ] No runtime/product/Cargo/protocol/schema/registry/production/external-repository semantics change.
- [ ] Exact-head governance/architecture/merge CI passes, whole-diff self-review is clean, fresh independent exact-head review is clean, and required review threads are zero before merge.

## Excluded scope

No runtime/product code, Cargo/workspace, protocol/schema/resource registry, production/protected environment, live data, external repository, coordinator transfer, lane allocation, merge-authority expansion or new owner-funded AI authority.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending exact-head CI

### Component/integration

- command/run: semantic cross-check of runbook, prompt README, lifecycle registry, scheduler and all updated entry prompts
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — governance/prompts/runbook only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: PR
- workflow/run/job: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: author whole-diff review
- material findings: pending
- verdict: pending

## Independent review

- required: YES — broad reusable prompt/governance package; conservative exact-head independent review requested by Issue #253
- exact head: pending
- method/auditor: fresh non-authoring reviewer
- material findings: pending
- verdict: pending

## Context checkpoint

```yaml
last_progress: Issue #253 opened and dedicated branch created from fresh protected main
status: implementing
branch: docs/owner-execution-guide-20260828
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
blocker: null
next_action: create the canonical operator runbook and owner execution/status advisor, then propagate explicit review-loop instructions to active entry prompts
```
