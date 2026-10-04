# OTV2-20261004-item-sem-use-packet

```yaml
task_id: OTV2-20261004-item-sem-use-packet
title: "ITEM-SEM-USE-PACKET-1: packet food and potion semantics"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-sem-use-packet-20261004
issue: 162
pr: 0
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_USE_FOOD_AND_POTION_SEMANTICS_PACKET_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-item-sem-use-packet.md
public_contracts: []
depends_on: [ITEM-USE-0, ACCEPT-ITEMUSE-BANK-0, ITEM-SEM-2b-3]
blocks: [ITEM-SEM-USE-1, ITEM-SEM-USE-2, ITEM-USE-1, FOOD-REGEN-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- ITEM-SEM-USE splits into a hard contract slice and a content slice. The typed model has no food,
  potion or flask field (packet §1.1).
- ITEM-SEM-USE-1 adds group 18 `consumption` (`Food`, `Potion` with ability and flask), the effect
  family `ManaRestore` and artifact v6 with recomputed ceilings (§1.2-§1.4, §2.1).
- ITEM-SEM-USE-2 lowers the edible food set, the eleven healing and mana potions, their abilities,
  flasks and requirements, from TibiaWiki first and Canary as fallback (§1.5, §2.2).
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
