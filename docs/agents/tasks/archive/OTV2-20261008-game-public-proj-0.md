# OTV2-20261008-game-public-proj-0

```yaml
task_id: OTV2-20261008-game-public-proj-0
title: GAME-PUBLIC-PROJ-0 Game public projections contract v1 (candidate)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/game-public-proj-0-20261008
issue: 1622
pr: 1937
base_sha: 340278d8
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_frozen_at: null
owner: GAME-PUBLIC-PROJ-0 writer (control plane P5, coordination #1622)
created_at: 2026-10-08T00:00:00Z
updated_at: 2026-10-08T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/agents/tasks/active/OTV2-20261008-game-public-proj-0.md
  - docs/agents/tasks/archive/OTV2-20261008-game-public-proj-0.md
public_contracts:
  - oteryn-game-public-projections-v1
depends_on: []
blocks:
  - Oteryn/Oteryn-Platform#1476 (P8 DECANARY-PUBLIC-1)
cross_repository_coordination_id: 1622
external_repositories: []
```

## Outcome

A candidate docs-only contract for six Game-owned public projections that Game pushes into
Platform read models: highscores, character profile, deaths, guilds, houses and the online
list. Each has a schema, privacy rules reviewed against the social presence baseline, ordering
and idempotency, a watermark, error handling and bounds.

## Architecture and source of truth

- Control-plane rulings Q3=A (push, LCFA style, no live query on page render) and D965 Q3=B
  (guilds and house rent in the core set).
- Transport, ordering, epoch fence and watermark reuse the accepted LCFA contract.
- Field sources: HIGHSCORES-0, GUILD-0, HOUSE-OWN-0, DEATH-0 and PARTY-PVP0 (all candidates;
  fields marked with their source). Privacy: the owner-accepted social presence baseline.

## Acceptance criteria

- [x] Contract file with all six families, privacy review, ordering, watermark, errors and limits.
- [x] `PENDING_CANONICAL_MERGE` lock entry naming this PR.
- [x] No code, migrations, Platform files or GAME-CHAR-CMD-0 files changed.

## Excluded scope

Code, migrations, the resource-limits registry (registered by the producer child), the Platform
repository and GAME-CHAR-CMD-0 files.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

## PR and closeout

- Review repair (CP ruling D607): the world-online watermark is held at the last completed scan; stale families are not served (fail closed); highscores `computed_at` is content; a guild with every member hidden publishes an empty roster (D245).
- Second review repair (CP ruling D607 1a 2a): `world_online` scans start every `PUBPROJ-ONLINE-INTERVAL`, and a late scan stalls the watermark; Platform shows a highscores row only while the same-name `character_profile` entry is served.
- PR #1937; closeout by squash merge of #1937 (pending). Platform consumption follows in Oteryn/Oteryn-Platform#1476.
