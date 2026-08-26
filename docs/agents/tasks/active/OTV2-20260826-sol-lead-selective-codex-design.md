# OTV2-20260826-sol-lead-selective-codex-design

```yaml
task_id: OTV2-20260826-sol-lead-selective-codex-design
title: Record Sol lead + selective Codex execution architecture
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/sol-lead-selective-codex-179
issue: 179
pr: null
base_sha: cb9c5f4f53dd880c9d338dafd21b6184a4419993
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT supervising architecture session
created_at: 2026-08-26T07:20:08Z
updated_at: 2026-08-26T07:22:00Z
execution_budget_minutes: 90
large_budget_reason: execution-model architecture, reusable prompt packaging and governance validation
owned_paths:
  - docs/superpowers/specs/2026-08-26-oteryn-game-sol-lead-selective-codex-execution-design.md
  - docs/agents/prompts/OTV2_SOL_EXECUTION_ARCHITECTURE_CONTINUATION.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/tasks/active/OTV2-20260826-sol-lead-selective-codex-design.md
public_contracts: []
depends_on:
  - owner-approved execution direction in Issue #179
  - OTV2_WORK_DELIVERY_COORDINATOR
  - OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR
blocks:
  - implementation-plan and Sol lane prompt package until written spec owner review
cross_repository_coordination_id: OTV2-SOL-LEAD-SELECTIVE-CODEX
external_repositories: []
```

## Outcome

Persist the owner-approved execution-model design and a reusable architecture-continuation prompt so a fresh Sol Extra High agent can continue from repository truth without relying on this chat transcript.

## Architecture and source of truth

- `PROVEN`: admission protected `main` is `cb9c5f4f53dd880c9d338dafd21b6184a4419993`.
- `PROVEN`: Issue #179 records the owner-approved direction: Work control plane + Sol lane leads + selective Codex assistance.
- `PROVEN`: Ability #171, Interaction #172 and AI #178 are merged at the admission snapshot; Durability #167 is still the next critical implementation lane.
- `PROVEN`: the current Work auditor prompt is read-only and remains an independent control.
- `DERIVED`: this design delivery does not widen runtime/production/cross-repository authority because it only records the execution model and a continuation prompt; actual Sol worker write authority remains future allocation-gated.

## Acceptance criteria

- [x] Record the Sol-lead/selective-Codex execution design under `docs/superpowers/specs/`.
- [x] Define Work as control plane, Sol leads as bounded reasoning/implementation owners, and Codex as selective implementation assistance.
- [x] Define five-chat useful concurrency, narrower mutating concurrency and serialized shared surfaces.
- [x] Preserve Durability -> Server Seam -> Client/QA -> Movement -> Combat dependency order.
- [x] Preserve audit/reconciliation findings rather than rewriting history.
- [x] Add a continuation prompt that requires live GitHub, written-spec owner review, writing-plans, prompt evaluation and fail-closed authority.
- [ ] Register the continuation prompt in README/lifecycle metadata.
- [ ] Self-review the written spec for placeholders, contradictions, scope and unsupported authority.
- [ ] Owner reviews the written spec before implementation-plan authoring.
- [ ] Exact-head governance/repository-policy/semantic/merge-authority/game-gate qualification is complete before merge.

## Excluded scope

No gameplay/runtime, Cargo/workspace, `Cargo.lock`, composition, resource registry, stable IDs, protocol/schema, workflow, production, secrets, Platform/Atlas/META or external-repository mutation. No Sol lane implementation begins from this task.

## Validation

### Focused

- command/run: prompt self-evaluation against `docs/agents/PROMPT_EVAL_STANDARD.md`
- result: pending final package review

### Component/integration

- command/run: repository governance/repository-policy exact-head CI after PR creation
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — execution architecture and prompt packaging only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: docs/governance only
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing architecture session
- material findings: pending
- verdict: pending

## Independent review

- required: pending final governance risk classification; no runtime/merge authority expansion is intended
- exact head: pending or `NOT_APPLICABLE`
- method/auditor: pending or `NOT_APPLICABLE`
- material findings: pending or `NOT_APPLICABLE`
- verdict: pending or `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: open PR #150 is unrelated root-governance work and must not be modified by this task
- protected auto-merge: not requested
- merge commit/result: pending written-spec owner review and exact-head qualification
- ownership release: pending terminal delivery

## Context checkpoint

```yaml
last_progress: owner-approved execution direction persisted as design spec and continuation prompt on Issue #179 branch
status: validating
branch: docs/sol-lead-selective-codex-179
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
owner_action_required: written_spec_review
blocker: implementation plan must not be authored until owner reviews the written spec
next_action: register continuation prompt in README and lifecycle registry, then present written spec for owner review
```
