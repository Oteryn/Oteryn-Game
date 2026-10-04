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
- #1792 Codex round 4 (CP D619):
  - P1 4179411647: the pin check is a `world_bundle` job inside `game-gate`, in
    `merge-gate.yml` and `merge-group-gate.yml`, gated by a `world_bundle_required` lane and
    failing closed when required. There is no separate `world-bundle.yml`. WORLD-BUNDLE-CI-1
    owns both gate workflows' job and needs entries, the classifier lane, the pinned job
    digests and their tests (§0.2, §1.2, §2.2).
  - P1 4179411651: `WorldActivationServerV1` carries an achievement-catalogue projection: the
    catalogue entry count, then each referenced achievement key with its state and revision.
    A change to only one of them refuses the prior issuance (§1.5, §2.4).
  - Sweep: the artifact also binds the `ruleset_revision` and `sim_revision`, each served
    claim's definition, policy, readiness, chest definition, map revision, reward counts,
    backpack and achievement key. A coverage rule lists every Content input of `chest_use`, the
    quest refresh and SPAWN-1b, and makes a later new input join the artifact in the same PR
    (§1.5).
- #1792 Codex round 5 (CP D621):
  - P1 4179449275: the artifact binds the `ItemDefinitionFacts` of every eligible backpack (an
    admitted definition with a container capacity and the container-slot pattern), not a
    per-claim backpack, because `prepare_chest_use` reads the equipped backpack. A §2.4 test
    changes a non-reward backpack (§1.5, §2.4).
  - P1 4179449279: `content/world/pins/<world slug>.identity.json`, owned by
    WORLD-BUNDLE-CI-1, carries the whole compiler `Identity`. Each field has a named committed
    source; a `derive-identity` mode writes it and the job fails on a difference (§1.2, §2.2).
  - P1 4179449282: the pin carries `inputs_digest`, a SHA-256 over the git blob ids and paths of
    the compiler inputs, pin files excluded, instead of a commit SHA (§1.2, §2.2, §3).
- #1792 Codex round 6 (CP D623):
  - P1 4179485199: the compiler inputs, and so `world_bundle_required` and `inputs_digest`, add
    the root `Cargo.toml`, `rust-toolchain.toml` and `.cargo/**` to `Cargo.lock`. `vendor/**` is
    excluded with its reason, and a new build configuration file joins the list in the same PR
    (§1.2, §2.2).
  - P1 4179485202: `min_runtime_version` comes from `world_bundle::RUNTIME_VERSION`, created at
    `1` by WORLD-BUNDLE-CI-1 with a bump rule independent of the format `VERSION`; the reader
    refuses a higher value (§1.2, §2.2).
  - P2 4179485206: `provenance_summary` starts with `revision_digest_token` as stored, with no
    second `lock:` prefix (§1.2).
- #1792 Codex round 7 (CP D627):
  - P1 4179526410: `ruleset_compatibility` comes from the committed compiler input
    `content/world/pins/<world slug>.ruleset.json`, not from `native_entry.rs`. The node refuses
    boot on a bundle without its `REVISIONS[2]`, and a unit test ties the file to the constant.
    A sweep records that every identity source is under a compiler input path (§1.2, §2.2, §2.4).
- #1792 Codex round 8 (CP):
  - P1 4179564957: `WorldActivationServerV1` binds the pin's `inputs_digest` and a
    creature-facts projection over the whole canonical definition, as the node loads it from its
    embedded shards, of every creature the spawn frame names. The coverage rule no longer says the
    bundle digest binds creature facts. A speed-only mutation must change the digest (§1.5, §2.4).
  - P1 4179564960: the typed `WorldId` is the first field of both the server and client
    artifacts. Changing only the `WorldId` must change both digests (§1.5, §2.4).
- Durable rows keep canonical identities; the bundle `placement_key` stays in memory. No
  migration, wire or contract change (§1.6-§1.8).

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
