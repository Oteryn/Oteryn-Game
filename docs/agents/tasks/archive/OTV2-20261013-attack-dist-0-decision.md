# OTV2-20261013-attack-dist-0-decision

```yaml
task_id: OTV2-20261013-attack-dist-0-decision
title: "ATTACK-DIST-0: auto-attack draw distribution decision"
mode: DOCUMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/attack-dist-0-20261013
pr: 2095
base_sha: main
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: oteryn-impl-worker (CP #1622, LOOP-COMBAT-MODES-1 Wave 1)
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
migration_lease: none
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ATTACK_DIST0_AUTO_ATTACK_DRAW_DECISION_2026-10-13.md
  - docs/agents/tasks/archive/OTV2-20261013-attack-dist-0-decision.md
public_contracts: []
depends_on: [OTV2-20261004-attack-parity-1b-formulas]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Decision packet `OTERYN_GAME_ATTACK_DIST0_AUTO_ATTACK_DRAW_DECISION_2026-10-13.md`: the uniform draw leaves a mean residual of -69 to -1 on all 1536 rows; a linear-decreasing density lands within +-1/3 of `raw.avg` but is not floor-exact (312/1536 rows fail packet 1.4), so no `MATCHED_TIBIAPAL` relabel follows and attack values stay `PARITY_PENDING`. Recommendation B (ATTACK-DIST-1), pending control plane acceptance and an owner decision on the mean gate. No code, no registry number.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
