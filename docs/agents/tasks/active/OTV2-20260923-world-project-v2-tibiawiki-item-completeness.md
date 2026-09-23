# OTV2-20260923 — WorldProject/v2 TibiaWiki Item completeness

```yaml
task_id: OTV2-20260923-world-project-v2-tibiawiki-item-completeness
title: Canonical WorldProject/v2 + contemporary Item completeness
mode: IMPLEMENT
status: authoring
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/world-project-v2-tibiawiki-item-completeness-20260923
base_sha: 74c626509a0efd7c80157189b9d74c59ab0b3894
owner: "single autonomous Content/World implementation agent"
owned_paths:
  - apps/game-server/src/content/project_v2.rs
  - apps/game-server/src/content/mod.rs
  - docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2.md
  - docs/agents/tasks/active/OTV2-20260923-world-project-v2-tibiawiki-item-completeness.md
```

## Outcome

Define and implement the canonical declarative `WorldProject/v2` carrier without replacing the accepted v1 runtime model. V2 must carry the full target authoring structure (NPC, Dialogue, Service, Interaction, Quest, House, Encounter, world placements/transitions, presentation/assets/provenance/editor roles) and close Item authoring gaps exposed by the contemporary TibiaWiki audit.

The Item v2 authoring layer must cover appearance binding, aliases/tags/taxonomy, Forge definition metadata, Weapon Proficiency profiles/perks/shaping declarations, typed augment bindings, typed on-use/consumable interaction bindings, rune magic-level/ability binding, enchantable/destructible lifecycle capabilities and source lifecycle provenance. Mutable forge tier, proficiency XP/selected perks/ranks, active imbues, charges/timers and other instance state remain outside WorldProject definitions.

NPC prices/buy/sell facts and dropped-by facts are relationships owned by Service/Loot/NPC/Creature content, not Item semantics.

## Source audit

Primary public reference inputs:
- TibiaWiki `Predefinição:Infobox_Item` current field vocabulary;
- TibiaWiki current Weapon Proficiency and Perk Shaping Options;
- TibiaWiki Exaltation Forge classification/max-tier behavior;
- representative 2025/2026 item pages carrying proficiency, forge and Elemental Bond.

TibiaWiki remains structured reference evidence only; this task imports no wiki values and makes no Reference truth claim.

## Compatibility

- Preserve `OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v1` and existing v1 writer/parser unchanged.
- Reuse existing `ProjectReferenceRecord` for already executable Reference families.
- V2-only/deferred families stay declarative and cannot silently lower into runtime.
- Paths and document roles are organizational; stable keys remain semantic identity.

## Validation

- Rust unit tests in the v2 module must prove strict serde, duplicate identity rejection, typed-reference family checks, Item instance-state exclusion, relationship routing, and v1 runtime record reuse.
- Repository exact-head CI / game-gate must qualify the frozen PR head before integration.
