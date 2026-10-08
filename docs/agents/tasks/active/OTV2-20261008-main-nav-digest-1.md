# OTV2-20261008-main-nav-digest-1

```yaml
task_id: OTV2-20261008-main-nav-digest-1
title: Re-pin the item_identity.rs navigation source digest broken on main by #1929
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/main-nav-digest-1-20261008
pr: null
issue: 1622
base_sha: 340278d84f5c5406b45c634c9750e3414844bd88
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: single writer (allocated by the CP of #1622 under D955, with owner consent)
created_at: 2026-10-08
updated_at: 2026-10-08
execution_policy: continuous_progress
owned_paths:
  - tools/content-migration/samples/navigation-seven-20261001.json
  - tools/content-migration/samples/engine-family-navigation-265.json
  - tools/content-migration/samples/official-rule-only-navigation-six.json
  - imports/tibiawiki/facts/items-bounded7-navigation-20261002.json
  - imports/tibiawiki/facts/items-family-alias26-20261001.json
  - tools/content-migration/item_bounded7_navigation.py
  - tools/content-migration/item_engine_navigation.py
  - tools/content-migration/item_navigation_source_supplement.py
  - tools/content-migration/item_official_navigation.py
  - content/items/taxonomy/items.json
  - docs/agents/tasks/active/OTV2-20261008-main-nav-digest-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

Main at `340278d8` fails `Content tree / Item+Mount equivalence` with
`NAVIGATION_SOURCE_DIGEST:apps/game-server/src/content/item_identity.rs`. #1929 changed
that file (adding `CURRENT_SOURCE_HELD_ITEM_IDS`) without re-pinning its sha256. The
62-id explicit appearance-only list and its `explicit_ids_sha256` are unchanged.

This task changes pinned digests only; no logic, no `.github/**`, `Cargo.*` or
`content/world/pins`.

## Change

- `item_identity.rs` sha256 `26941af8…` -> `0df52c39…` in the five navigation source files.
- Hash-of-hashes cascade: the four `tools/content-migration/item_*navigation*.py` constants
  for the changed source files, the `a12_helper` pin in the bounded7 facts file and the
  matching bounded7 constant.
- `content/items/taxonomy/items.json` regenerated with
  `tools/content-migration/world_project_v2_to_tree.py`.

## Validation

Reproduced first on base: taxonomy, source-supplement and bounded7 tests failed with
`NAVIGATION_SOURCE_DIGEST`.

Candidate:

- `python tools/content-migration/world_project_v2_to_tree.py` PASS
- `python tools/content-migration/test_world_project_v2_to_tree.py` OK
- `python tools/content-migration/test_item_taxonomy.py` OK
- `python tools/content-migration/test_item_navigation_source_supplement.py` OK
- `python tools/content-migration/test_item_engine_navigation.py` OK
- `python tools/content-migration/test_item_external_family_refinement.py` OK
- `python tools/content-migration/test_item_bounded7_navigation.py` OK
- `python tools/content-migration/validate_world_project_v2_to_tree.py` PASS
