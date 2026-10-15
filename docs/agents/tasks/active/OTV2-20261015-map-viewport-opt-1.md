# OTV2-20261015-map-viewport-opt-1

```yaml
task_id: OTV2-20261015-map-viewport-opt-1
title: "MAP-VIEWPORT-OPT-1 map-view budget under owner option B (walking players only)"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: map
base_branch: main
branch: claude/map-viewport-opt-1-20261015
pr: null
base_sha: 7a61f366
owner: claude-code-session_0181Dg1mgG57hxwWBACEks7U (worker)
control_plane: claude-code-session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
packet: "control plane allocation of 2026-10-15; owner decision option B (2026-10-15), https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6186538185 point 2"
head_sha: "exact frozen head in the report to the control plane"
owned_paths:
  - apps/game-server/src/gameplay_transport/world_map_real_tests.rs
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md
  - docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md   # row MAP-VIEWPORT-OPT-1 only
  - docs/agents/evidence/MAP-VIEWPORT-OPT-1-budget.md
  - docs/agents/tasks/active/OTV2-20261015-map-viewport-opt-1.md
  - docs/agents/tasks/archive/OTV2-20261015-map-viewport-opt-1.md
follow_up: MAP-VIEWPORT-BUDGET-REVAL
```

## Scope

MAP-VIEWPORT-MEASURE-1 (#1873) measured 162.5-174.0 us of CPU per update: 0.41-0.44 core when all 500
players step every 200 ms (option A), against 20% of one core. Owner decision 2026-10-15, option B (https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6186538185, point 2): keep the
20% share and count only players who walk, at walking pace. This task records that model in
`ARCH-MAP-VIEWPORT-BUDGET-V1` §1.5, adds the option B line and a unit test to the measurement, and records the
evidence. No code optimisation, no wire, registry, runtime or limit change.

## Result

Walking pace 2.86 steps/s (SPEED-1 step of a level-100 player, effective speed 209, on ground speed 150:
350 ms), 50% of players walking: 0.124 core on the largest measured cost, within 0.20. Break-even: 80% walking
at that pace. Both the pace and the moving share are provisional assumptions, re-validated by
MAP-VIEWPORT-BUDGET-REVAL with player-simulating bots (load-sim).

## Validation

See the evidence file, *Validation*.

Evidence: `docs/agents/evidence/MAP-VIEWPORT-OPT-1-budget.md`.
