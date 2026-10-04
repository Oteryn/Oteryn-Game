# OTV2-20261004-prof-snowball-rekey

```yaml
task_id: PROF-SNOWBALL-REKEY-1
title: "Admit Snowball (53855) appearance-only and re-key the items-stats cascade"
mode: AUTHORING
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prof-snowball-rekey-20261003
base_sha: 8588b1bb
owner: claude-code-session-01YYfF5Rgp5EEG6NFs4aFJED (control-plane allocation D418/D419)
created_at: 2026-10-03
updated_at: 2026-10-04
owned_paths:
  - apps/game-server/src/content/item_capacity_promotion.rs
  - apps/game-server/src/content/item_description_promotion.rs
  - apps/game-server/src/content/item_description_wiki_promotion.rs
  - apps/game-server/src/content/item_document_promotion.rs
  - apps/game-server/src/content/item_elemental_magic_modifier_promotion.rs
  - apps/game-server/src/content/item_forge289_promotion.rs
  - apps/game-server/src/content/item_forge3332_promotion.rs
  - apps/game-server/src/content/item_hit_magic_promotion.rs
  - apps/game-server/src/content/item_identity.rs
  - apps/game-server/src/content/project/v2/proficiency/import.rs
  - apps/game-server/src/content/item_mantra_bond_modifier_promotion.rs
  - apps/game-server/src/content/item_market_true_promotion.rs
  - apps/game-server/src/content/item_movable_promotion.rs
  - apps/game-server/src/content/item_name15_promotion.rs
  - apps/game-server/src/content/item_name_promotion.rs
  - apps/game-server/src/content/item_numeric_modifier_promotion.rs
  - apps/game-server/src/content/item_physical_promotion.rs
  - apps/game-server/src/content/item_stack_default_promotion.rs
  - apps/game-server/src/content/item_stack_default_successor8_promotion.rs
  - apps/game-server/src/content/item_stack_false_promotion.rs
  - apps/game-server/src/content/item_stack_historical_promotion.rs
  - apps/game-server/src/content/item_stats_promotion.rs
  - apps/game-server/src/content/item_timed_promotion.rs
  - apps/game-server/src/content/item_use_observation_promotion.rs
  - apps/game-server/src/content/item_weapon_metadata_promotion.rs
  - apps/game-server/tests/content_item_identity.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/abilities/definitions/index.json
  - content/abilities/effects/index.json
  - content/abilities/formulas/index.json
  - content/behaviors/index.json
  - content/content.lock.json
  - content/creatures/definitions/index.json
  - content/items/definitions/items-28500-28999.json
  - content/items/definitions/items-29000-29499.json
  - content/items/definitions/items-29500-29999.json
  - content/items/definitions/items-30000-30499.json
  - content/items/definitions/items-30500-30999.json
  - content/items/definitions/items-31000-31499.json
  - content/items/definitions/items-31500-31999.json
  - content/items/definitions/items-32000-32499.json
  - content/items/definitions/items-32500-32999.json
  - content/items/definitions/items-33000-33499.json
  - content/items/definitions/items-33500-33999.json
  - content/items/definitions/items-34000-34030.json
  - content/items/definitions/items-34000-34031.json
  - content/items/index.json
  - content/items/taxonomy/items.json
  - content/loot/index.json
  - content/manifest.json
  - content/presentations/definitions/index.json
  - content/proficiencies/bindings.json
  - content/world/content.lock.json
  - content/world/definitions/reference.json
  - content/world/manifest.json
  - content/world/project.json
  - content/world/provenance/imports.json
  - docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json
  - docs/agents/evidence/OTV2-20261001-item-capacity-promotion-v1.json
  - docs/agents/evidence/OTV2-20261001-item-document-promotion-v1.json
  - docs/agents/evidence/OTV2-20261001-item-document-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261001-item-forge3332-promotion-v1.json
  - docs/agents/evidence/OTV2-20261001-item-market-true-promotion-v1.json
  - docs/agents/evidence/OTV2-20261001-item-market-true-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261001-item-movable-promotion-v1.json
  - docs/agents/evidence/OTV2-20261001-item-movable-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261001-item-physical-promotion-v1.json
  - docs/agents/evidence/OTV2-20261001-item-stack-default-promotion-v1.json
  - docs/agents/evidence/OTV2-20261001-item-stack-false-promotion-v1.json
  - docs/agents/evidence/OTV2-20261001-item-stack-historical-promotion-v1.json
  - docs/agents/evidence/OTV2-20261001-item5801-temporal-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-description-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-description-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-description-wiki-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-description-wiki-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-forge289-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-forge289-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-hit-magic-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-hit-magic-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-mantra-bond-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-mantra-bond-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-name-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-name-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-name15-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-name15-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-current-receipt-v1.json
  - docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-use-observation-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-use-observation-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-weapon-metadata-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-weapon-metadata-source-qualification-v2.json
  - docs/agents/evidence/OTV2-20261002-stack-default-historical-native-context-v1.json
  - docs/agents/evidence/OTV2-20261002-tibiawiki165-historical-import-context-v1.json
  - docs/agents/evidence/OTV2-20261003-item-equipment-requirements-v1.json
  - docs/agents/evidence/OTV2-20261003-item-market-true-promotion-v2.json
  - docs/agents/evidence/OTV2-20261003-item-movable-promotion-v2.json
  - docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v2.json
  - docs/agents/evidence/OTV2-20261003-timed-item-facts-v1.json
  - docs/agents/evidence/OTV2-20261004-prof-snowball-rekey-repin-receipt-v1.json
  - docs/agents/tasks/archive/OTV2-20261004-prof-snowball-rekey.md
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/facts/items-bounded7-navigation-20261002.json
  - imports/tibiawiki/facts/items-external-family18-20261001.json
  - imports/tibiawiki/facts/items-family-alias26-20261001.json
  - imports/tibiawiki/facts/items-stats.json
  - imports/tibiawiki/sources.json
  - tools/content-migration/item_bounded7_navigation.py
  - tools/content-migration/item_engine_navigation.py
  - tools/content-migration/item_external_family_refinement.py
  - tools/content-migration/item_navigation_source_supplement.py
  - tools/content-migration/item_official_navigation.py
  - tools/content-migration/samples/engine-family-navigation-265.json
  - tools/content-migration/samples/navigation-seven-20261001.json
  - tools/content-migration/samples/official-rule-only-navigation-six.json
  - tools/content-migration/test_item_official_navigation.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-schema/imbuement-authoring/imbuement_authoring.py
  - tools/content-schema/imbuement-authoring/samples/imbuement-bindings.json
  - tools/content-schema/imbuement-authoring/samples/imbuement-source-comparison.json
  - tools/content-schema/imbuement-authoring/samples/imbuements-candidate.json
  - tools/content-schema/imbuement-authoring/samples/missing-item-definitions.json
  - tools/content-schema/item-authoring/check_stack_default_historical_context.py
  - tools/content-schema/item-authoring/check_tibiawiki165_historical_context.py
  - tools/content-schema/item-authoring/key_ring5801_source_selection.py
  - tools/content-schema/item-authoring/lower_client_market_packet.py
  - tools/content-schema/item-authoring/lower_elemental_magic_modifier_packet.py
  - tools/content-schema/item-authoring/lower_item_description_packet.py
  - tools/content-schema/item-authoring/lower_item_description_wiki_packet.py
  - tools/content-schema/item-authoring/lower_item_document_packet.py
  - tools/content-schema/item-authoring/lower_item_forge289_packet.py
  - tools/content-schema/item-authoring/lower_item_forge3332_packet.py
  - tools/content-schema/item-authoring/lower_item_hit_magic_packet.py
  - tools/content-schema/item-authoring/lower_item_name15_packet.py
  - tools/content-schema/item-authoring/lower_item_name_packet.py
  - tools/content-schema/item-authoring/lower_item_use_observation_packet.py
  - tools/content-schema/item-authoring/lower_item_weapon_metadata_packet.py
  - tools/content-schema/item-authoring/lower_mantra_bond_modifier_packet.py
  - tools/content-schema/item-authoring/lower_numeric_modifier17_packet.py
  - tools/content-schema/item-authoring/lower_wiki_movable_packet.py
  - tools/content-schema/item-authoring/lower_wiki_stack_default_successor8_packet.py
  - tools/content-schema/item-authoring/samples/item-weapon-proficiency-15-30-7fea90ec.json
  - tools/content-schema/proficiency-authoring/README.md
  - tools/content-schema/proficiency-authoring/test_proficiency_authoring.py
  - tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json
  - tools/content-schema/world-object-authoring/official_corpses.py
  - tools/content-schema/world-object-authoring/qualified_world.py
  - tools/content-schema/world-object-authoring/samples/census-official-corpses-15.30.json
  - tools/content-schema/world-object-authoring/samples/census-world-wiki46-fixed3-15.30.json
  - tools/content-schema/world-object-authoring/samples/qualified-official-corpses-15.30.json
  - tools/content-schema/world-object-authoring/samples/qualified-world-wiki46-fixed3-15.30.json
public_contracts: []
```

## Outcome

Carries the D401 Snowball WIP (`743983e5`) onto `main`. Snowball 53855 becomes the 61st appearance-only
Item record (34,032 Items, 665 proficiency bindings). The items-stats snapshot is re-keyed, so its records
digest moves from `5fc20ff7…` to `2e6e083d…`.

- D441/1a: every digest that is live-checked and invalidated by the re-key is re-pinned mechanically, to a
  fixed point. The pins are listed in `docs/agents/evidence/OTV2-20261004-prof-snowball-rekey-repin-receipt-v1.json`.
- D441/2a: the TibiaWiki-165 historical context, the numeric-modifier17 current receipt, the successor8
  current-parent receipt v2 and the stack-default historical native context are in scope.
- The stack default/historical, timed-item facts and reward-claim variant packets were regenerated by their
  own tools. Only the snapshot digest and the renamed shard path changed.
- No validator was changed. Historical receipts, archived task records and evidence that no check reads
  keep their original pins.
- Merged `main` after #1712. The owner approved the `--theirs` resolution of 79 conflicts, the reset of
  pin-only files to `main`, and a regenerate-and-cascade pass.

## Validation

- `regenerate_content.py` (materialize, tree, authoring tools and checks): pass.
- Every `item-authoring-schema` workflow step, run locally in its workflow working directory: pass.
- Steps of the content-tree-migration, canonical WorldProject seed, crystal bindings, wave1 capture, proficiency,
  quest, reward-claim, wheel and world-metadata workflows: pass. The crystal-bindings ruff step reports the same
  two findings on `main`.
- `cargo test --locked --no-fail-fast -p oteryn-game-server`: 19541 passed, 0 failed.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings` and `cargo fmt --all --check`: clean.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.
