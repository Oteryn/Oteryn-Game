# OTV2-20260923 — WorldProject/v2 TibiaWiki Item completeness

```yaml
task_id: OTV2-20260923-world-project-v2-tibiawiki-item-completeness
title: Canonical WorldProject/v2 + contemporary Item completeness
mode: IMPLEMENT
status: authoring
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/world-project-v2-tibiawiki-item-completeness-r2-20260923
base_sha: ec8803bd1a37600acd0ff544811871e8fb2c40cf
head_sha: pending
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

## Implemented v2 coverage

- Existing v1 executable ProjectReferenceRecord remains the only runtime-lowerable representation for accepted families.
- Added declarative v2 carrier in `apps/game-server/src/content/project_v2.rs`.
- Added typed Item v2 overlay for appearance/aliases/tags/taxonomy, Forge metadata, proficiency binding, augment binding, on-use interactions, magic-level requirements, enchantable/destructible lifecycle and source implementation/removal provenance.
- Added typed ProficiencyDefinition with levels, perks and Perk Shaping bounds (refine/reshape/clear/lunar ascension declarations).
- Added typed AugmentDefinition with Ability/auto-attack/offensive-rune/creature-class/generic target domains and exact rational rank values.
- Added typed InteractionDefinition with closed trigger family and Ability/Effect/Service/Quest/Item-transform/NativeRule execution bindings.
- Added NPC/Dialogue/Service/Quest/House/Encounter, assets, placements/transitions, provenance and editor metadata carriers.
- Mutable ItemInstance/progression state is structurally excluded by strict serde.
- NPC prices/shop offers route through Service; drop relationships route through Loot/Creature/Encounter instead of Item semantics.

## Validation status

Local compilation is not claimed. The branch uses repository-native authoring and will rely on exact-head hosted Rust/semantic/game-gate qualification after candidate freeze.
