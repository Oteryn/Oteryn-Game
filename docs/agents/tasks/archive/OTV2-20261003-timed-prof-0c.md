# OTV2-20261003-timed-prof-0c

```yaml
task_id: OTV2-20261003-timed-prof-0c
title: "TIMED-PROF-0C: #1662 P2 follow-ups (deadline lock order, shaping retention)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/timed-prof-0c
pr: 1667
base_sha: 92cb1e55
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01QFRdKvFbNsNiCzMyR75FrC (Sol Supervising Architect, second lane)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY1B_PERK_MODIFICATION_VALUE_SHAPES_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-timed-prof-0c.md
public_contracts: []
depends_on: ["#1662 merged"]
blocks: [TIMED-RT-1, PROF-SHAPE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation: D370 (#1622). Two P2 findings deferred from #1662 under D245:

- 4174228270: TIMED-ITEM-0B §10.3 tile-item lock order is now fence, item row, location row,
  timed row, as ITEM-MOVE-WIRE-1 §7.2; test for a concurrent expiry and Ground move.
- 4174228267: PROFICIENCY-1B verifies costs from each line's stored `dust_cost` and `orb_cost`;
  content is read only at commit, under §8's lock, and by reconcile while the revision is still
  retained. Retention stays row-based.

## Validation

- `git diff --check`; `python tools/agents/validate_governance.py`;
  `python -m unittest discover -s tools/agents/tests`
