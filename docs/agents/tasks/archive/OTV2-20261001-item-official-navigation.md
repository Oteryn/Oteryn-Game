# OTV2-20261001-item-official-navigation

```yaml
task_id: OTV2-20261001-item-official-navigation
title: Classify six existing official-only Items through accepted A12 identity
mode: DATA_ENRICHMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/item-official-navigation-20261001
pr: 1486
base_sha: d690e122b0818b64528188e2c6a66e9c46b86a56
owner: owner-directed Codex session
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
depends_on: [1483, 1437]
owned_paths:
  - tools/content-migration/{item_official_navigation.py,item_taxonomy.py,test_item_official_navigation.py,test_world_project_v2_to_tree.py}
  - tools/content-migration/samples/official-rule-only-navigation-six.json
  - content/items/taxonomy/items.json
  - content/content.lock.json
  - docs/agents/evidence/OTV2-20261001-item-official-navigation.md
  - docs/agents/tasks/archive/OTV2-20261001-item-official-navigation.md
public_contracts: []
```

Exactly six NAVIGATION_ONLY records add coverage without fabricating donor/Wiki bindings
or promoting native stats. Earlier taxonomy, native definitions, world catalogs and engine
decoder are preserved. Totals are 9,561 taxonomy entries and 3,015 unclassified/unowned
records. Current membership and the explicit admitted identity cohort are mandatory guards.

Six new tests, eight taxonomy regressions, aggregate/migration drift, 97/97 materialized
validation, Ruff, formatting and whitespace pass. Exact returned SHA and full delta readback
are recorded externally after authoring freeze. Draft #1486 remains integration-pending;
this closes only its bounded authoring batch, not Item completeness or protected integration.
