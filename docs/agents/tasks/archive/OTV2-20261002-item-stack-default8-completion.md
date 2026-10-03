# OTV2-20261002-item-stack-default8-completion

```yaml
task_id: OTV2-20261002-item-stack-default8-completion
title: Add eight documented Item stack defaults
mode: DATA_ENRICHMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/item-stack-default8-20261002
pr: 1560
base_sha: b0f91e2883007861f5556c44cce13701bb447283
owner: owner-directed Codex session
created_at: 2026-10-02
updated_at: 2026-10-02
execution_policy: continuous_progress
depends_on: [1530]
owned_paths:
  - .github/workflows/item-authoring-schema.yml
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/src/content/item_stack_default_successor8_promotion.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/abilities/definitions/index.json
  - content/abilities/effects/index.json
  - content/abilities/formulas/index.json
  - content/behaviors/index.json
  - content/content.lock.json
  - content/creatures/definitions/index.json
  - content/items/definitions/items-12000-12499.json
  - content/items/index.json
  - content/loot/index.json
  - content/presentations/definitions/index.json
  - content/world/content.lock.json
  - content/world/definitions/reference.json
  - content/world/manifest.json
  - content/world/project.json
  - docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-current-parent-receipt-v1.json
  - docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-stack-default8-completion.md
  - docs/agents/tasks/archive/OTV2-20261002-item-stack-default8-completion.md
  - imports/tibiawiki/source-evidence/stack-default-successor8/global-infobox-object-source-part-098.json
  - imports/tibiawiki/source-evidence/stack-default-successor8/global-own-itemid-index.json
  - tools/content-schema/item-authoring/lower_wiki_stack_default_successor8_packet.py
  - tools/content-schema/item-authoring/test_stack_default_successor8.py
  - tools/content-schema/item-authoring/verify_default8_exact_delta.py
public_contracts: []
```

Exactly eight Items with absent own stack parameter gain stackable=false from the explicitly documented InfoboxObject default. These are derived source defaults, not literal official flags. Preserve stack maxima and every other Native field, all57320 records/34031 canonical Items, all411 source owners, all250 imbuement source rows/325 references and World. Historical1487 and seven defaults remain separately verified and unchanged. Full checks and original repeat generation passed.

Independent review and final selected checks PASS. Publication SHA/full frozen readback are retained externally. CI/protected integration and full Item readiness remain pending. Original source checkpoints and published recovery archive are preserved.
