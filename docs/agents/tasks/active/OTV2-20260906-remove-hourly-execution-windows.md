# OTV2-20260906-remove-hourly-execution-windows

```yaml
task_id: OTV2-20260906-remove-hourly-execution-windows
title: Remove wall-clock worker execution windows
mode: GOVERNANCE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/remove-hourly-execution-windows
pr: 362
issue: null
owner_scope: explicit owner instruction 2026-09-06
base_sha: 7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3
owned_paths:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md
  - docs/agents/prompts/OTV2_CLOSE_NEXT_WAVE_BLOCKERS.md
  - docs/agents/prompts/OTV2_CONTENT_FORMAT_SPIKE.md
  - docs/agents/prompts/OTV2_IMPL_ANALYTICS.md
  - docs/agents/prompts/OTV2_IMPL_DOMAIN_CORE.md
  - docs/agents/prompts/OTV2_IMPL_FOUNDATION_RUNTIME.md
  - docs/agents/prompts/OTV2_IMPL_GAME_ABILITY.md
  - docs/agents/prompts/OTV2_IMPL_GAME_AI.md
  - docs/agents/prompts/OTV2_IMPL_GAME_CHANNEL.md
  - docs/agents/prompts/OTV2_IMPL_GAME_INTERACTION.md
  - docs/agents/prompts/OTV2_IMPL_NATIVE_CLIENT.md
  - docs/agents/prompts/OTV2_IMPL_QA_E2E.md
  - docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md
  - docs/agents/prompts/OTV2_IMPL_SIMULATION.md
  - docs/agents/prompts/OTV2_IMPL_VSL_COMBAT.md
  - docs/agents/prompts/OTV2_IMPL_VSL_CONTENT.md
  - docs/agents/prompts/OTV2_IMPL_VSL_MOVEMENT.md
  - docs/agents/prompts/OTV2_IMPL_WORKSPACE_BOOTSTRAP.md
  - docs/agents/tasks/active/OTV2-20260906-remove-hourly-execution-windows.md
```

## Goal

Remove the repository-created 60/120-minute implementation stop/rotation mechanism immediately while preserving bounded no-progress, repeated-failure and CI-wait protections.

## Result

- productive authorized work has no wall-clock execution window;
- elapsed time alone cannot trigger `ROTATE`, freeze, discarded minutes, re-admission or a fresh Work grant;
- repository-root instructions make legacy `windowN`/minute counters in existing tasks/plans/programme overlays non-operative across the whole repository, including `docs/superpowers/**`;
- new tasks no longer receive `execution_budget_minutes` / `large_budget_reason` from the mandatory template;
- the autonomous programme policy now treats elapsed implementation time as non-terminal;
- every reusable prompt entry previously found with a default 60/120-minute foreground budget now states continuous productive execution instead;
- no-progress, repeated-failure, bounded passive exact-head CI waiting, ownership, scope, validation, review, Merge Queue and safety gates remain intact.

Historical evidence, archived tasks and old plans are intentionally not rewritten; root `AGENTS.md` preserves them as provenance while explicitly making their old execution-time counters non-operative.

## Validation required

- review complete diff and ensure no authority/path/test/merge gate is weakened beyond removal of the wall-clock stop;
- run `python tools/agents/validate_governance.py`;
- run repository policy validation if selected by current governance;
- exact-head `agent-governance` / protected PR checks;
- independent exact-head review because this is a material governance change.

## Review findings

- P2 `3944819203`: fixed — mandatory `TASK_TEMPLATE.md` no longer creates 60/120-minute execution-budget metadata; it uses `execution_policy: continuous_progress` and points time-independent anti-stall behavior to the central policy.
- P2 `3944819204`: fixed — task status is the registered `ready` state.
- P2 `3944828549`: fixed — all reusable executor/coordinator prompts returned by the repository search for the default 60-minute foreground budget were updated directly; no direct reusable entry point relies only on the blanket override.

## Context checkpoint

`next_action`: requalify PR #362 on the repaired exact head through checks and independent review, then integrate normally without bypass.
