# OTV2-20260912-wp3-v2-multi-agent-programme-registration

```yaml
task_id: OTV2-20260912-wp3-v2-multi-agent-programme-registration
title: Register WP3-v2 multi-agent delivery programme and prompts
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/wp3-v2-multi-agent-delivery-20260912
pr: null
base_sha: 489e3e390a1bce1ce3439c66521ab75f8a826cd8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT documentation coordinator
created_at: 2026-09-12T20:05:00+02:00
updated_at: 2026-09-12T20:05:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/programs/OTV2_WP3_V2_MULTI_AGENT_DELIVERY_PROGRAMME.md
  - docs/agents/programs/OTV2_WP3_V2_AGENT_LAUNCH_RUNBOOK.md
  - docs/agents/evidence/OTV2_WP3_GAME_PLATFORM_CROSS_REPO_AUDIT_R01_R21_20260912.md
  - docs/agents/prompts/OTV2_ASTRA_WP3_V2_PROGRAMME_COORDINATOR.md
  - docs/agents/prompts/OTV2_ASTRA_WP3_V2_ARCHITECTURE_LEAD.md
  - docs/agents/prompts/OTV2_SOL_WP3_V2_EVIDENCE_AUDITOR.md
  - docs/agents/prompts/OTV2_ASTRA_PLATFORM_NATIVE_EVIDENCE_HARDENING.md
  - docs/agents/prompts/OTV2_ASTRA_WP3_V2_IMPLEMENTATION_LEAD.md
  - docs/agents/prompts/OTV2_ASTRA_CHILD_B_DURABILITY_LEAD.md
  - docs/agents/prompts/OTV2_ASTRA_WP5_SOURCE_COMPOSITION_LEAD.md
  - docs/agents/tasks/active/OTV2-20260912-wp3-v2-multi-agent-programme-registration.md
public_contracts: []
depends_on:
  - issue:162
  - issue:364
  - pr:588
blocks: []
cross_repository_coordination_id: WP3-V2-GAME-PLATFORM-20260912
external_repositories:
  - Oteryn/Oteryn-Platform
```

## Outcome

Persist the owner-requested WP3-v2 multi-agent execution split, launch aliases and the retained Game+Platform R01-R21 audit in new documentation-only paths without changing runtime, accepted architecture, historical evidence, existing worker branches or external repositories.

## Architecture and source of truth

- `PROVEN`: protected Game main at admission was `489e3e390a1bce1ce3439c66521ab75f8a826cd8`.
- `PROVEN`: #162 remains the Game coordinator and #364 remains the active remediation programme.
- `PROVEN`: PR #588 is evidence only and recommends a WP3-v2 superseding decision.
- `PROVEN`: #247 preserves the existing canonical Server Seam worker/branch/alias.
- `PROVEN`: Platform is a separate repository owner boundary; this task performs no Platform mutation.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: documentation-only registration. This task performs no production mutation, authority-bearing runtime transition, PREPARE/COMMIT, persisted recovery interpretation or protected integration action.

## Acceptance criteria

- [x] Programme DAG separates A0-A7 responsibilities and gates.
- [x] Parallelism is bounded to path-disjoint work with one mutating owner per material lane.
- [x] Existing canonical #335 and #247 lineages are preserved.
- [x] Existing Q01-Q75 remains the WP3 qualification matrix rather than being silently replaced.
- [x] Cross-repository findings R01-R21 are retained with evidence classification and explicit limitations.
- [x] A0-A6 have task-specific prompts; A7 reuses the existing canonical Server Seam prompt.
- [x] No historical #588 artifact is rewritten.
- [x] No runtime/Cargo/vendor/SQL/workflow/Platform/production mutation is included.

## Excluded scope

No architecture acceptance, WP3 implementation, Child B implementation, source implementation, Platform write, Server Seam mutation, production action, secret/certificate work, check weakening, merge or Merge Queue submission.

## Validation

### Focused

- source/reference readback: pending final branch readback
- result: pending

### Component/integration

`NOT_APPLICABLE`: documentation-only registration with no product behavior change.

### E2E

`NOT_APPLICABLE`: documentation-only registration.

### Exact-head CI

- final head: pending
- trigger source: PR
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending under applicable documentation/governance risk policy
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #588 evidence remains separate; #356/#335/#247 untouched
- protected auto-merge: NOT_REQUESTED
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: registered new programme/evidence/prompt documentation on dedicated branch
status: validating
branch: agent/wp3-v2-multi-agent-delivery-20260912
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
next_action: inspect exact branch diff, create draft PR, then record PR live state
```