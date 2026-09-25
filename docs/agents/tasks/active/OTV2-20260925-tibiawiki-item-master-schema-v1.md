---
task_id: OTV2-20260925-tibiawiki-item-master-schema-v1
title: TibiaWiki Item Master Schema v1 census
mode: ARCHITECTURE_EVIDENCE
status: authoring
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/tibiawiki-item-master-schema-v1-20260925
base_sha: 2389c6671000b8b0efe341540a62e303e307ad15
issue: 162
jira: KAN-16
allocation_comment: 5834862904
created_at: 2026-09-25T17:08:00+02:00
owned_paths:
  - docs/agents/tasks/active/OTV2-20260925-tibiawiki-item-master-schema-v1.md
  - docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json
  - docs/architecture/OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md
  - tools/content-schema/validate_item_master_schema.py
  - tools/content-schema/test_validate_item_master_schema.py
---

# TibiaWiki Item Master Schema v1 census

## Outcome

Produce one machine-readable superset Item authoring schema derived from the current
TibiaWiki `Infobox Item` parameter surface and current Item navigation families,
mapped onto existing Oteryn Item ownership.

Acceptance:

- every discovered current template parameter has exactly one disposition;
- rendering/control parameters are explicit and cannot become gameplay fields;
- reverse relationships (loot, NPC offers, task membership) are not duplicated as
  authoritative Item fields;
- commercial/external values remain outside gameplay Item authority;
- free-form `attrib` is retained losslessly as source observation while known semantics
  are promoted into typed capability candidates;
- current main Item semantics are reused rather than replaced;
- family navigation coverage has no unassigned family;
- deterministic validator reports zero unassigned fields and zero unassigned families.

## Source locators

- https://www.tibiawiki.com.br/index.php?action=edit&title=Predefini%C3%A7%C3%A3o%3AInfobox_Item
- https://www.tibiawiki.com.br/index.php?action=edit&title=Predefini%C3%A7%C3%A3o%3AItens
- protected Oteryn Item semantics at admission main:
  `apps/game-server/src/content/reference_playable.rs`
- protected WorldProject/v2 Item authoring:
  `apps/game-server/src/content/project/v2.rs`

## Boundaries

This task does not migrate the 38,157 Item corpus, change WorldProject/v2 physical
storage, change runtime/compiler/artifact/protocol/persistence behavior, or promote
TibiaWiki/XML numeric identifiers into canonical gameplay identity.

The master schema may identify typed authoring gaps (for example light/sleep/use facts
currently carried only in free-form Wiki attributes). Recording such a gap is not
runtime authority; executable adoption requires its owning later implementation slice.
