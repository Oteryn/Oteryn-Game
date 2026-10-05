# OTV2-20261005-arch-map-track-0

```yaml
task_id: OTV2-20261005-arch-map-track-0
title: "ARCH-MAP-TRACK-PACKETS-V1: map-track packets (viewport budget, cutover 1a/1b/1c, client)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-map-track-20261005
issue: 162
pr: 1819
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_WIRE_PACKETS_2026-10-04.md
  - docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-map-track-0.md
public_contracts:
  - docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md
depends_on: []
blocks: [MAP-VIEWPORT-PERF-1, MAP-CUTOVER-1a, MAP-CUTOVER-1b, MAP-CUTOVER-1c, MAP-CLIENT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- CP D725. Decision `ARCH-MAP-TRACK-PACKETS-V1` in
  `docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md`.
- Viewport gate (§1.1): the capability-18 offer gate already binds composition plus encode to
  `MAP01-VIEWPORT-US` (100 us p99); MAP-VIEWPORT-PERF-1 targets the whole snapshot with unchanged
  wire bytes. No registry change.
- Bundle World before Ground persistence (§1.2): MAP-CUTOVER-1a boots a pinned testing or
  preproduction bundle World (Bundle collision variant, configured start, fail-closed pins) and
  serves nobody; MAP-CUTOVER-1b serves domain 17 and offers capability 18 on a bundle World only;
  MAP-CUTOVER-1c is the old SOCIAL-MAP §2.8.
- Amendment MAPW-A1 (§1.3), proposed in the MAP-WIRE-1 contract §3 and effective only on owner
  answer 1a: capability 18 requires 6 only; item-handle entries are `display_only` without 4.
- MAP-CLIENT-1 re-issued as `OTV2-20261005-map-client-1` (draws in `play.rs`, joint ground-speed
  switch). Pointer notes in SOCIAL-MAP §2.8 and ARCH-MAP-WIRE §2.3.
- Owner question: (1) capability 18 without capability 4.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
