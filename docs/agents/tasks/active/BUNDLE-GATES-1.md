# BUNDLE-GATES-1

```yaml
task_id: BUNDLE-GATES-1
title: "BUNDLE-GATES-1: audited, test-enforced inventory of the qualified_room gates on a bundle World"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/bundle-gates-1-20261014
issue: 1622
governing_issue: 1622
pr: null
base_sha: 75ec7985
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
depends_on: ["BUNDLE-COMBAT-ENABLE-1 (#2025)"]
owned_paths:
  - apps/game-server/src/gameplay_transport/bundle_gates_tests.rs
  - apps/game-server/src/gameplay_transport/mod.rs   # test module declaration only
  - docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md   # BUNDLE-GATES-1 row only
  - docs/agents/tasks/active/BUNDLE-GATES-1.md
  - docs/agents/tasks/archive/BUNDLE-GATES-1.md
public_contracts: []
external_repositories: []
```

## Scope

Plan row BUNDLE-GATES-1 (residue of #2025): inventory every `qualified_room` gate of the gameplay seam on main, record which are open and by which merged PR, name the batch that opens each closed one, and add a guard test. No runtime, wire, persistence, migration or registry change.
