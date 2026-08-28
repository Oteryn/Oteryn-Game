# OTV2-20260828-owner-execution-guide

```yaml
task_id: OTV2-20260828-owner-execution-guide
title: Canonicalize owner agent launch and status guidance
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/owner-execution-guide-20260828
issue: 253
pr: 254
base_sha: 12d4ca5326d62a7a2c46d80cd5e167e99f109d1d
head_sha: external GitHub PR evidence
final_head_sha: external GitHub PR evidence
final_head_frozen_at: 2026-08-28T23:15:00+02:00
owner: owner-execution-guide-author-session
created_at: 2026-08-28T22:56:24+02:00
updated_at: 2026-08-28T23:15:00+02:00
execution_budget_minutes: 90
large_budget_reason: cross-prompt operational clarification and canonical owner runbook
owned_paths:
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/programs/OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md
  - docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md
  - docs/agents/prompts/OTV2_OWNER_EXECUTION_STATUS_ADVISOR.md
  - docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md
  - docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md
  - docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md
  - docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md
  - docs/agents/prompts/README.md
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

Give the owner and active execution entry profiles one canonical operational view of aliases, model/effort selection, Work-vs-chat placement, lane launch order, autonomous Codex review, Work Auditor evidence, and live GitHub status reconciliation. This package clarifies existing authority only; it does not create a new control plane, lane allocation, merge authority or owner-funded AI permission.

## Architecture and source of truth

- `PROVEN`: protected admission/current validation `main` is `12d4ca5326d62a7a2c46d80cd5e167e99f109d1d`.
- `PROVEN`: PR #230 is merged and `docs/agents/CODEX_REVIEW_POLICY.json` is canonical.
- `PROVEN`: Issue #253 records owner authorization for this clarification package.
- `PROVEN`: root `AGENTS.md` remains the authority source for the standing Codex review authorization and single active control-plane behavior.
- `PROVEN`: current Work/Terra/Supervising-Architect/Work-Auditor prompts already contain canonical Codex routing from the merged reusable-prompt sweep, including lane-owner routing, no technical discretion, no nested Codex dispatch by auditors and no architect candidate-owner promotion.
- `DERIVED`: changing those already-aligned prompts again would add churn without changing behavior; this package links the owner-facing runbook through README/scheduler and standardizes only the five mutating Sol handoffs that lacked explicit `codex_review` fields.

## Acceptance criteria

- [x] Canonical runbook lists aliases, recommended surface/model/effort, mutation mode and launch order.
- [x] Runbook defines fresh-GitHub classification of `DONE`, `ACTIVE`, `BLOCKED`, `READY_NEXT`, `DO_NOT_LAUNCH` and `UNKNOWN`.
- [x] New reusable owner execution/status advisor is read-only, live-GitHub-first and returns exact Work/chat launch instructions plus exactly one next action.
- [x] Existing Work/Terra prompts were freshly verified to mechanically verify canonical Codex routing/evidence without replacing lane-lead review ownership; no content change was needed.
- [x] Existing Sol Supervising Architect was freshly verified not to become a candidate/review-request owner or implementation/merge role through standing Codex authorization; no content change was needed.
- [x] Five Sol VSL leads explicitly own the already-canonical policy-required review loop and now expose standardized Codex evidence in handoff.
- [x] Existing Work Auditor was freshly verified to inspect candidate-owner Codex evidence while prohibiting nested Codex dispatch and implementation; no content change was needed.
- [x] Prompt README, scheduler and lifecycle registry align with the new owner execution/status advisor and runbook.
- [x] No runtime/product/Cargo/protocol/schema/registry/workflow/production/external-repository semantics change.
- [ ] Exact-head governance/architecture/merge CI passes, whole-diff self-review is clean, fresh independent exact-head review is clean, and required review threads are zero before merge.

## Excluded scope

No runtime/product code, Cargo/workspace, protocol/schema/resource registry, workflows, production/protected environment, live data, external repository, coordinator transfer, lane allocation, merge-authority expansion or new owner-funded AI authority.

## Implementation / findings

- Added `OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md` as the canonical owner-facing operational map, explicitly subordinate to live GitHub/governance.
- Added reusable read-only `Oteryn: owner execution guide` / `OTV2_OWNER_EXECUTION_STATUS_ADVISOR` and lifecycle-registered it.
- Standardized the five mutating Sol lead handoffs with `codex_review` route/source/head/evidence/findings/thread/status fields.
- Linked README and Terra/Sol scheduler to the owner runbook and live-state classification discipline.
- Removed historical Issue numbers from being treated as current-state authority in the scheduler; they remain provenance only.
- Verified-no-change: Work Coordinator, Terra Control Plane, Sol Supervising Architect and Work Auditor already carry the required post-#230/#235 canonical review semantics.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending exact-head PR CI

### Component/integration

- command/run: lifecycle completeness + semantic cross-check of runbook, prompt README, scheduler, new advisor and five lane handoffs
- result: author review pending

### E2E

- scenario: `NOT_APPLICABLE` — governance/prompts/runbook only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: external GitHub PR evidence
- trigger source: PR #254
- workflow/run/job: pending
- result: pending

## Self-review

- exact head: pending external evidence
- method/reviewer: author whole-diff review
- material findings: pending
- verdict: pending

## Independent review

- required: YES — reusable prompt/governance package; exact-head independent review required by Issue #253
- exact head: pending
- method/auditor: fresh non-authoring native Codex reviewer under protected-main standing review authorization
- material findings: pending
- verdict: pending

## Context checkpoint

```yaml
last_progress: PR #254 is open; content is frozen after binding the task to the PR and must now be qualified without evidence-only commits
status: validating
branch: docs/owner-execution-guide-20260828
head_sha: external GitHub PR evidence
pr: 254
final_head_sha: external GitHub PR evidence
final_head_frozen_at: 2026-08-28T23:15:00+02:00
blocker: exact-head CI, author whole-diff self-review and fresh independent Codex review remain required
next_action: qualify the unchanged PR #254 head with exact-head CI, author whole-diff self-review and fresh independent Codex review
```
