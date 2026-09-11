# OTV2-20260911-native-ui-prompt-activation

```yaml
task_id: OTV2-20260911-native-ui-prompt-activation
title: Activate Native UI reusable prompt family
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/native-ui-prompt-activation-20260911
pr: null
base_sha: d956fb6c852a4cfe0213c87e3d15e991133a5de1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: OTV2_WORK_DELIVERY_COORDINATOR
created_at: 2026-09-11T12:46:00+02:00
updated_at: 2026-09-11T12:46:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/prompts/OTV2_SOL_NATIVE_UI_LEAD.md
  - docs/agents/prompts/OTV2_SOL_NATIVE_UI_P1.md
  - docs/agents/prompts/OTV2_SOL_NATIVE_UI_INPUT.md
  - docs/agents/prompts/OTV2_SOL_NATIVE_UI_RENDERER.md
  - docs/agents/prompts/OTV2_SOL_NATIVE_UI_HUD.md
  - docs/agents/prompts/OTV2_SOL_NATIVE_UI_QUALIFY.md
  - docs/agents/prompts/OTV2_SOL_NATIVE_UI_CI.md
  - docs/agents/prompts/OTV2_SOL_NATIVE_UI_REVIEW.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/prompts/README.md
  - docs/agents/evidence/OTV2-20260911-native-ui-prompt-activation-evaluation.md
  - docs/agents/tasks/active/OTV2-20260911-native-ui-prompt-activation.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn-Game#565
  - Oteryn/Oteryn-Game#162
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Activate exactly the eight Native UI reusable prompt contracts adopted by protected PR #565, register their canonical lifecycle entries and aliases, and record deterministic/adversarial prompt-evaluation evidence without creating a second control plane or any runtime/P1 write authority.

## Architecture and source of truth

- **PROVEN:** protected `main@d956fb6c852a4cfe0213c87e3d15e991133a5de1` contains PR #565 and both Native UI programme documents.
- **PROVEN:** the adopted source prompt blocks live in `docs/agents/programs/OTERYN_NATIVE_UI_AGENT_PROGRAMME_V1.md`, section 12.
- **PROVEN:** dispatchability is controlled by `docs/agents/PROMPT_LIFECYCLE.json`; `status=reusable` grants no write authority without the live allocation required by the prompt.
- **PROVEN:** `docs/agents/PROMPT_EVAL_STANDARD.md` requires lifecycle/delivery/task-delta/domain/adversarial evaluation, not text lint alone.
- **PROVEN:** existing #162 remains the unique control plane; CP-A must not alter active coordinator allocation or implementation authority.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this change registers reusable prompts and evaluation evidence only. It changes no runtime, protected-environment action, durable value mutation, PREPARE/COMMIT authority, production session/fence, Merge Queue rule or branch protection.

## Acceptance criteria

- [ ] Exactly eight canonical Native UI prompt files are extracted from the protected programme with full programme path expansion.
- [ ] Exactly eight unique lifecycle entries are `status=reusable`, `reusable=true`, version `1.0`, with bounded owner/scope and no supersession ambiguity.
- [ ] `docs/agents/prompts/README.md` exposes exactly the eight short aliases and states alias resolution grants no write/control-plane authority.
- [ ] Reusable prompt bodies contain no model/effort execution configuration.
- [ ] Every prompt emits mutually exclusive canonical `DONE / WAITING_EXTERNAL / BLOCKED / STALLED` plus its separate domain result/disposition.
- [ ] Evaluation evidence covers valid allocation, missing allocation, stale head, held Cargo lease, competing control plane, missing alias, resume after lease transfer, missing physical host/timing, FOV undecided, synthetic production rejection, prompt injection, unknown CI routing, repair-vs-false-finding and green-PR-vs-protected-readback cases.
- [ ] Repository governance/META/lifecycle tests and exact-head CI pass.
- [ ] Independent review finds no material defect before integration.
- [ ] Integration uses native Merge Queue and protected-main readback.
- [ ] Protected post-adoption canaries prove canonical alias resolution and fail-closed behavior before Native UI implementation launch.

## Excluded scope

No runtime/client/server/protocol/Cargo/lock/workspace-boundaries/workflow/ruleset/protection/production change. No active #162 allocation mutation. No P1 implementation, lease seizure or alias-created writer authority. No FOV product decision.

## Implementation / findings

Pending extraction, registration and evaluation on this branch only.

## Validation

### Focused

- command/run: pending
- result: pending

### Component/integration

- command/run: pending
- result: pending

### E2E / behavior

- scenario: protected post-adoption Native UI alias canaries
- result: `NOT_EVALUATED_BEFORE_PROTECTED_ADOPTION`

### Exact-head CI

- final head: pending immutable PR/check evidence
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: repository-selected
- classification: docs/governance
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES — reusable prompt/lifecycle activation affecting autonomous dispatch resolution
- exact head: pending
- method/auditor: genuinely independent exact-head reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #565 protected source programme
- protected Merge Queue: required
- merge commit/result: pending
- ownership release: after protected readback and canaries

## Context checkpoint

```yaml
last_progress: CP-A exact branch and task seeded from protected Native UI programme merge
status: implementing
branch: docs/native-ui-prompt-activation-20260911
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
next_action: extract and register the eight protected Native UI prompts, validate/evaluate, then freeze one reviewable CP-A candidate
```
