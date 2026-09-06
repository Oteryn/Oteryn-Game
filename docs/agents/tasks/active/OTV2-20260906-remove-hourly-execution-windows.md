# OTV2-20260906-remove-hourly-execution-windows

```yaml
task_id: OTV2-20260906-remove-hourly-execution-windows
title: Remove wall-clock worker execution windows
mode: GOVERNANCE
status: READY_FOR_REVIEW
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/remove-hourly-execution-windows
pr: 362
issue: null
owner_scope: explicit owner instruction 2026-09-06
base_sha: 7ce1d88ba7eb83033c4f0c11a5ccd1cb5030fac3
owned_paths:
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md
  - docs/agents/prompts/OTV2_IMPL_FOUNDATION_RUNTIME.md
  - docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md
  - docs/agents/prompts/OTV2_IMPL_WORKSPACE_BOOTSTRAP.md
  - docs/agents/tasks/active/OTV2-20260906-remove-hourly-execution-windows.md
```

## Goal

Remove the repository-created 60/120-minute implementation stop/rotation mechanism immediately while preserving bounded no-progress, repeated-failure and CI-wait protections.

## Result

- productive authorized work has no wall-clock execution window;
- elapsed time alone cannot trigger `ROTATE`, freeze, discarded minutes, re-admission or a fresh Work grant;
- legacy `windowN` and minute counters remain historical provenance only;
- no-progress, repeated-failure, exact-head CI waiting, ownership, scope, validation, review, Merge Queue and safety gates remain intact;
- Server Seam, Foundation, Bootstrap and the Implementation Coordinator prompts state the continuous-execution rule directly.

## Validation required

- review complete diff and ensure no authority/path/test/merge gate is weakened beyond removal of the wall-clock stop;
- run `python tools/agents/validate_governance.py`;
- run repository policy validation if selected by current governance;
- exact-head `agent-governance` / protected PR checks;
- independent exact-head review because this is a material governance change.

## Context checkpoint

`next_action`: qualify PR #362 through exact-head checks and independent review, then integrate normally without bypass.
