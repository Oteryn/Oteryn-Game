# OTV2-20261004-map-load-packet-1

```yaml
task_id: OTV2-20261004-map-load-packet-1
title: "MAP-LOAD-PACKET-1: the World Bundle loader packet (MAP-LOAD-1)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/map-load-packet-1-20261004
issue: 162
pr: 1744
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_MAP_LOAD_PACKET1_BUNDLE_LOADER_DECISION_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-map-load-packet-1.md
public_contracts: []
depends_on: []
blocks: [MAP-LOAD-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Control plane D491 (owner 5a): MAP-LOAD-1 moves out of #1733 (ARCH-BATCH-ROOT-PACKETS-V1
  §1.2-§1.4, §2.1) into this decision. It covers bundle staging (§1.1), the shared reader crate
  (§1.2), the ground item and speed source (§1.3), the Terrain catalogue input (§1.4) and the
  packet (§2.1). The earlier #1733 fix P1 4176957737 (the catalogue as a second loader input)
  comes with it.
- #1733 P1 4177026515: the catalogue is pinned by `terrain_catalogue_digest`, a SHA-256 over the
  exact shard bytes. The compiler and the server share the definition. The lock revision check is
  a consistency check only (§1.4).
- #1733 P2 4177026518: six registered MAP01-TERRAIN-* limits, each enforced before allocation and
  tested at max and max+1 (§1.5).
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
