# OTV2-20261006-arch-map-viewport-budget-0

```yaml
task_id: OTV2-20261006-arch-map-viewport-budget-0
title: "ARCH-MAP-VIEWPORT-BUDGET-V1: assembly budget kept, temporary snapshot gate, MAP-VIEWPORT-MEASURE-1"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: cand/map-viewport-budget
issue: 162
pr: 1864
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_WIRE_PACKETS_2026-10-04.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20261006-arch-map-viewport-budget-0.md
public_contracts:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
depends_on: []
blocks: [MAP-VIEWPORT-MEASURE-1, MAP-CUTOVER-1b]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Control plane request of 2026-10-06 on #1859 (MAP-VIEWPORT-PERF-2, snapshot p99 1.683 ms);
  owner answer 1a.
- `MAP01-VIEWPORT-US` stays 100 us p99 for assembly. New row `MAP01-VIEWPORT-SNAPSHOT-US`,
  2,000 us p99 for the whole domain-17 snapshot, testing and preproduction only.
- Capability 18 `offer_gate` names the new row and the production condition. Pointer notes in
  ARCH-MAP-TRACK §1.1 and ARCH-MAP-WIRE §2.1.
- Packet MAP-VIEWPORT-MEASURE-1 (real map, realistic positions and walks, concurrency, D128 at
  20% of one core). View work stays off the Channel writer (§1.3).

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
