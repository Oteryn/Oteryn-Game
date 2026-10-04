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
pr: 1792
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
- #1792 Codex round 1:
  - P1 4179282475: SPAWN-ADMIT-1 now runs after WORLD-BUNDLE-CI-1 and refreshes the imported
    World's pin in its own PR. Any digest-changing PR refreshes the pin. WORLD-CONTENT-SERVE-1
    depends on SPAWN-ADMIT-1 (§0.3, §1.2, §2.1, §2.4).
  - P1 4179282482: CHEST-PLACE-BIND-1 owns a sparse top-level `unique` table and
    `TileView::unique` in `map/mod.rs` (§0.2, §2.3).
  - P1 4179282483: WORLD-CONTENT-SERVE-1 owns `content/world_activation.rs` (the three bundle-World
    digests) and the bundle-World mode of `oteryn-game-ops content activate`. The node checks the
    same digests at boot (§1.5, §2.4).
- #1792 Codex round 2:
  - P1 4179322261: the binding compares the entry's palette appearance id (from the
    `oteryn:{item,terrain}.tibia.i<id>` key, or `source_item_id` of a provisional donor key)
    with `appearance_tibia_id`. It does not resolve an Item, so non-Item chest appearances
    such as 28827 and 28828 bind (§1.4, §2.3).
  - P1 4179322266: `world-bundle.yml` and the pin refresh cover every compiler input:
    `content/world/**`, `content/houses/**`, `content/creatures/definitions/**`,
    `content/items/definitions/**`, the compiler, `crates/world-bundle/**` and `Cargo.lock`
    (§1.2).
  - P1 4179322271: the server activation artifact includes the served claims' quest
    transitions, the `ItemDefinitionFacts` of each reward Item and backpack, and the quest
    catalogue digest (§1.5).
  - P1 4179322272: a bundle World is activated through `ContentActivationController`. It
    stages a canonical `WorldActivationServerV1` artifact through `stage_primary`, then
    calls `activate`, which yields a non-Clone `WorldBundleContentPin`. WORLD-CONTENT-SERVE-1
    owns `activate_world_bundle` in `content/activation.rs` and the staging branch in
    `content/production.rs` (§0.2, §1.5, §2.4).
- #1792 Codex round 3 (CP D617):
  - P1 4179378371: the compiler drops provisional entries, so the 2 ready claims on 28827 and
    28828 are not ready to serve until admission. On the non-production pin they are
    `NO_ENTRY`, left out and counted. CHEST-APPEARANCE-ADMIT-1, a control-plane-allocated
    content packet, admits them (§1.4).
  - P1 4179378375: `entry_start` is part of `WorldActivationServerV1`. An issuer and node test
    shows that a change to it alone refuses the prior issuance (§1.5, §2.4).
- Durable rows keep canonical identities; the bundle `placement_key` stays in memory. No
  migration, wire or contract change (§1.6-§1.8).

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
