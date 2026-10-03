# OTV2-20261001-item-equipment-facts

```yaml
task_id: OTV2-20261001-item-equipment-facts
mode: REPAIR
status: completed_pending_merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: d3cfb2468510255d2495514ec975c23c1d76f32a
branch: codex/item-equipment-facts-20261001
owner: owner-directed Codex session
created_at: 2026-10-01
owned_paths:
  - apps/game-server/src/content/item_stats_promotion.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-schema/item-authoring/**
  - docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json
  - content/world/{project.json,manifest.json,content.lock.json,definitions/reference.json}
  - content/items/{index.json,definitions/**}
  - content/{content.lock.json,abilities/**/index.json,behaviors/index.json,creatures/definitions/index.json,loot/index.json,presentations/definitions/index.json}
```

The owner requested continued Item categorization, schema repair and enrichment using
Crystal, Canary, client data and external sources. This bounded successor to #1437 adds
1,784 wiki-qualified equipment patterns: 1,210 known levels, 817 vocation lists and 742
hand-occupancy declarations. Absent restrictions remain UNKNOWN. Runic use requirements,
ammunition, contradictory claims and vague vocation labels remain held. The separately
qualified starter backpack admission is preserved. The pinned packet retains all 21
source holds, page/revision coordinates and the compiler/source digests. A mixed gear and
Rune/Ammunition observation now fails closed. Explicit client semantic slots corroborate
the new patterns without resolving generic hand/ammo flags. No identity, materialization
or destination admission changes; exactly 1,784 equipment fields differ from the base.

The engine authoring converter also preserves partial life/mana-leech observations:
missing chance/amount stays absent; explicit zero remains zero. Both engines have fixture
coverage; the authoring schema requires at least one observed percentage. Engine modifiers
remain hypotheses, rather than promoted gameplay or proficiency-adjusted baseline values.

Validation: native materialization, full library tests (1,280 passed; two ignored),
4 focused stat-promotion tests, packet/source qualification regressions and deterministic
rebuild, 253 schema checks, 599 engine checks plus three partial-leech regressions,
Item/tree migration and materialized-tree validation, Ruff, formatting and governance.
The broader package compilation exhausted local scratch storage; it is not claimed as
passing. Exact candidate CI remains required. Source conflicts, runtime rulesets and
physical admission remain open. The programme control plane retains review/integration;
this session publishes a draft and does not merge or dispatch metered external review.
