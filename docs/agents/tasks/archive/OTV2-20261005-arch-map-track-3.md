# OTV2-20261005-arch-map-track-3

```yaml
task_id: OTV2-20261005-arch-map-track-3
title: "ARCH-MAP-TRACK-PACKETS-V1 §2.7: MAP-ITEM-REF-1 depends on KILL-REWARD-COMP-1"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-map-track-3-20261005
issue: 162
pr: 1842
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-map-track-3.md
public_contracts: []
depends_on: [OTV2-20261005-arch-map-track-2]
blocks: [OTV2-20261005-map-item-ref-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- The #1828 review finding on the live corpse settlement was deferred under D245, and the CP
  recorded it. It is now applied.
- MAP-ITEM-REF-1 (§2.7) depends on KILL-REWARD-COMP-1 (`OTV2-20261004-kill-reward-comp-1`) as
  well as ITEM-MOVE-1. Its base is `main` after both merge.
- Reason: its acceptance loots a killed creature's corpse, and KILL-REWARD-COMP-1 owns the
  production corpse mint and settlement. MAP-ITEM-REF-1 adds no corpse mint of its own.
- The §2.1 order table names the new predecessor and the shared `gameplay_transport/mod.rs`.
- The implementation brief (item 5 and the order line) names both predecessors too, so the
  short slice agrees with §2.1 and §2.7.
- MAP-CUTOVER-1b and MAP-CLIENT-1 are unchanged; they wait on it through MAP-ITEM-REF-1.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
