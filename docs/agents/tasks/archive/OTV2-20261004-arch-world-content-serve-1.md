# OTV2-20261004-arch-world-content-serve-1

```yaml
task_id: OTV2-20261004-arch-world-content-serve-1
title: "ARCH-WORLD-CONTENT-SERVE-1: serving the imported world content on a node"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-world-content-serve-20261004
issue: 162
pr: 0
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_WORLD_CONTENT_SERVE_PACKETS_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-world-content-serve-1.md
public_contracts: []
depends_on: []
blocks: [SPAWN-ADMIT-1, WORLD-BUNDLE-CI-1, CHEST-PLACE-BIND-1, WORLD-CONTENT-SERVE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Five gaps separate the imported content from a quest step on a node: the bundle artifact and
  pin, the spawn source, the chest-to-map binding, the node composition and the quest binding
  (§0.1). The last is CHEST-QUEST-BIND-1 (#1789).
- **SPAWN-ADMIT-1** (§2.1) replaces the Canary spawns with the CrystalServer `00ce02a5`
  candidates of #1791, without its 33 held groups.
- **WORLD-BUNDLE-CI-1** (§2.2) puts the World pin in `content/world/pins/` and builds the bundle
  as a digest-named CI artifact.
- **CHEST-PLACE-BIND-1** (§2.3) binds each ready plain RewardClaim placement to exactly one
  bundle entry by cell, CrystalServer unique id and appearance.
- **WORLD-CONTENT-SERVE-1** (§2.4) serves the bound claims in a separate imported World, with a
  production boot gate, after MAP-CUTOVER-1.
- Durable rows keep canonical identities; the bundle `placement_key` stays in memory. No
  migration, wire or contract change (§1.6-§1.8).

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
