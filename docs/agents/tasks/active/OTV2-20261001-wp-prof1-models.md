# WP-PROF1-1a — checked track values and pending lines
```yaml
task_id: OTV2-20261001-wp-prof1-models
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: codex/weapon-proficiency-durable-models-20261001
base_branch: codex/weapon-proficiency-native-selection-20261001
base_sha: 6959f405d924785a614b8589c8aaa763d318f37f
issue: 162
pr: null
owner: Weapon Proficiency Codex worker, packet 5936420312
owned_paths: [apps/game-server/src/durability/character_proficiency.rs, apps/game-server/src/durability/mod.rs, docs/agents/tasks/active/OTV2-20261001-wp-prof1-models.md, docs/agents/tasks/archive/OTV2-20261001-wp-prof1-models.md]
depends_on: [PR1478, PROFICIENCY-0 accepted model, D281 A2]
```
Inert checked values preserve nonnegative BIGINT, nullable choices and strict cause direction.
Raw keys are lexically bounded; canonical identity/content/fence/mapping are independently
verified by future owners, never established by these constructors. No write/load/SQL API.
Migration0032 remains reserved and unapplied; no wire, shaping or activation in this batch.
Validation and independent review: final PR and #162 FREEZE packet; full PROF-1 continues.
