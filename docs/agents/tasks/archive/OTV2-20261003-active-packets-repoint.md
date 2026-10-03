# OTV2-20261003-active-packets-repoint

```yaml
task_id: OTV2-20261003-active-packets-repoint
title: Repoint active packets from closed #162 and #1479 (batch 8)
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/active-packets-repoint
pr: 1676
base_sha: 2226c09
owner: implementation worker for the CP (#1622)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
  - docs/agents/tasks/active/OTV2-20260929-item-id-1-tibia-id-keys.md
  - docs/agents/tasks/active/OTV2-20261001-charm-runtime-completion.md
  - docs/agents/tasks/archive/OTV2-20261003-active-packets-repoint.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome

The Agent governance live-state check passes again: no active packet names a closed Issue without an open PR, or a closed PR.

## Findings

| Packet | Was | Now |
|---|---|---|
| work-delivery-coordinator | `issue: 162` (closed) | `issue: 1622` |
| item-id-1-tibia-id-keys | `issue: 162` (closed) | `issue: 1622` |
| charm-runtime-completion | `issue: 162`, `pr: 1479` (closed unmerged) | `issue: 1622`, `pr: null`; still active, multi-PR |

## Excluded scope

Only the canonical issue/PR fields and the coordinator's lifecycle sentence change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `validate_active_task_live_state` against live GitHub (all active packets): OK, no errors
- `git diff --check`: clean

## Self-review

The issue regex accepts no trailing comment, so the issue lines carry none. The charm task is not terminal, so it stays active instead of being archived.

## Independent review

- required: per the bound review policy, on the frozen head (requested by the CP)

## PR and closeout

- PR: #1676. Exact frozen head: the one in the CP FREEZE entry. Merge commit/result: squash merge of #1676.
