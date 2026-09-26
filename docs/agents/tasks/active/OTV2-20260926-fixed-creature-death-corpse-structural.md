# OTV2-20260926-fixed-creature-death-corpse-structural

```yaml
task_id: OTV2-20260926-fixed-creature-death-corpse-structural
title: Fixed one-creature committed death and corpse structural component
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/fixed-creature-death-corpse-structural
issue: 162
allocation_comment: 5849676308
jira_story: KAN-12
profile: NONSHIPPING_FIXED_ONE_CREATURE
base_sha: f6b267126d6a4ee505f614a7b740ab16aa86b7b6
head_sha: null
final_head_sha: null
owner: Codex Combat Lead
created_at: 2026-09-26T20:40:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/foundation/channel_owner_combat_death_tests.rs
  - docs/agents/evidence/OTV2-20260926-fixed-creature-death-corpse-structural.md
  - docs/agents/tasks/active/OTV2-20260926-fixed-creature-death-corpse-structural.md
  - docs/agents/tasks/archive/OTV2-20260926-fixed-creature-death-corpse-structural.md
```

Allocation: #162 comment 5849676308. This task composes the existing exact
actor resolver and owner-mediated Ability damage commit into one bounded,
nonshipping structural path: an opaque receipt for the committed positive-HP
to zero transition produces exactly one runtime-owned corpse projection at the
original owner position. It introduces no Movement evaluation, relocation,
spatial legality, production activation, loot, XP, pickup, Item/DUR
transaction, persistence, process-restart recovery, decay or public protocol.

Canary and CrystalServer are pinned read-only `OTS_HYPOTHESIS_ONLY` ordering
references. Oteryn-Game remains the sole implementation and authority source.
The worker authors, freezes and validates the exact candidate; independent
review, Merge Queue, integration readback and lease release remain with the
restored `OTV2_WORK_DELIVERY_COORDINATOR`.
