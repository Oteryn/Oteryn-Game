# OTV2-20260827-terra-sol-parallel-execution

```yaml
task_id: OTV2-20260827-terra-sol-parallel-execution
title: Package Terra control plane and Sol parallel lead architecture
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/terra-sol-parallel-agent-architecture-20260827
issue: 213
pr: null
base_sha: 4c395ece416c3c56aed5607653a0730c52dcb3fd
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-08-27T09:27:00+02:00
updated_at: 2026-08-27T09:27:00+02:00
execution_budget_minutes: 120
large_budget_reason: authority-sensitive multi-agent execution architecture, prompt family and lifecycle registration
owned_paths:
  - docs/superpowers/specs/2026-08-27-oteryn-game-terra-sol-parallel-execution-design.md
  - docs/superpowers/plans/2026-08-27-oteryn-game-terra-sol-parallel-execution.md
  - docs/agents/prompts/OTV2_TERRA_GAME_CONTROL_PLANE.md
  - docs/agents/prompts/OTV2_SOL_SUPERVISING_ARCHITECT.md
  - docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md
  - docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md
  - docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md
  - docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_POST_VSL_EXPANSION.md
  - docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/tasks/active/OTV2-20260827-terra-sol-parallel-execution.md
public_contracts: []
depends_on:
  - owner direction in Issue #213
  - existing Sol-lead design and plan on protected main
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Publish a reusable execution-governance package in which ChatGPT Work on Terra High is a deterministic control plane with zero technical/architecture discretion, while exact Sol Extra High lane leads own implementation reasoning and a Sol Supervising Architect owns material cross-lane architecture decisions.

## Architecture and source of truth

- `PROVEN`: protected admission main is `4c395ece416c3c56aed5607653a0730c52dcb3fd`.
- `PROVEN`: Issue #213 records owner scope for this execution architecture.
- `PROVEN`: the existing owner-approved Sol-lead execution design and adoption plan are already on protected main.
- `PROVEN`: current Durability work is live under Issue #167 and draft PR #212 at task admission; live GitHub may advance during this governance task.
- `DERIVED`: adding a new Terra profile is safer than rewriting the currently reusable Work coordinator prompt while an active programme is in flight.

## Acceptance criteria

- [ ] Terra coordinator prompt has explicit zero technical/architecture discretion.
- [ ] Sol Supervising Architect prompt routes material architecture decisions without granting implicit runtime writes.
- [ ] Sol Durability, Server Seam, Client/QA, Movement and Combat lead prompts are present and bind mutation to exact live allocation.
- [ ] Post-VSL expansion prompt can decompose later accepted work without speculative runtime authority.
- [ ] Scheduler defines V0-V4 promotion, useful parallel preparation and writer limits.
- [ ] README and prompt lifecycle register every new alias.
- [ ] Every new prompt passes `PROMPT_EVAL_STANDARD.md` self-evaluation.
- [ ] Exact-head agent governance and applicable repository governance checks pass.
- [ ] Genuinely independent exact-head review passes before merge because execution/merge authority boundaries change.

## Excluded scope

No gameplay/runtime, Cargo/workspace, protocol/schema/registry, production/protected environment, secrets, live data, Platform/Atlas/META or external-repository mutation. Do not modify active Durability #167/#212 implementation history or current product worker branches.

## Implementation / findings

The existing architecture already anticipated Work control-plane + Sol lane leads, but current `main` has no canonical Sol lane-lead prompt family. This task packages the missing profiles and further narrows the Terra coordinator so it cannot make technical judgments.

## Validation

### Focused

- command/run: prompt self-review against `docs/agents/PROMPT_EVAL_STANDARD.md`
- result: pending

### Component/integration

- command/run: `python tools/agents/validate_governance.py` or exact-head GitHub equivalent
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — governance/prompt package only, no executable product behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: PR pending
- workflow/run/job: pending
- runner assignment: pending
- classification: governance-only
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES — execution/merge authority boundary change under `docs/agents/AGENTS.md`
- exact head: pending
- method/auditor: independent non-authoring reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none; additive profile package
- protected auto-merge: not requested before independent review
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Issue #213 and dedicated branch created; design/spec and implementation plan started
status: implementing
branch: docs/terra-sol-parallel-agent-architecture-20260827
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
next_action: create Terra coordinator, Sol architect and Sol lane-lead prompt family
```
