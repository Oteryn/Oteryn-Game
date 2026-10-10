# CONTENT-REGEN-FIX-1

```yaml
task_id: CONTENT-REGEN-FIX-1
title: Make regenerate_content.py exit 0 on main by materializing from the pinned predecessor
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/content-regen-fix-1-20261008
pr: 1950
issue: 1926
base_sha: 1e77ca22
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: single writer (allocated by the control plane)
created_at: 2026-10-08
updated_at: 2026-10-10
execution_policy: continuous_progress
owned_paths:
  - tools/content-migration/**
  - content/**
  - docs/agents/tasks/archive/CONTENT-REGEN-FIX-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

`tools/content-migration/regenerate_content.py` failed on clean main. The from-scratch
materializer chain cannot produce the #1807 monster/spell delta or the D3-7 corpse admission
(MAIN-SEED-DRIFT-1, #1926), so regeneration lost 1,271 records (10 items) and the imbuement
check reported `equipment evidence differs from the current content tree`. CI avoids this by
passing `--predecessor-root` with the package at pinned `d78f0c80`; the local tool did not.

## Change

`regenerate_content.py` exports the 11 package documents from the pin
(`git show <pin>:content/world/<locator>`, fetching the pin with depth 1 if absent) and passes
`--predecessor-root` to the materializer, which digest-verifies it. No Rust, `imports/**`,
workflow or derived-content edits: the regenerated tree equals the committed tree.

## Validation

- `python3 tools/content-migration/regenerate_content.py` exit 0, no content diff pass
- `python tools/content-migration/test_engine_items.py` OK
- `python tools/content-migration/item_weapon_proficiency.py --check` OK
- `python tools/content-migration/g4_item_crystal_binding_generator.py --check` OK
- `cargo test --locked -p oteryn-game-server --test content_world_project_repository` 5/5 OK
- `python tools/agents/validate_governance.py` pass
- `python -m unittest discover -s tools/agents/tests` OK

Merge: derived-content tooling PR; the control plane orders it in the Merge Queue before NPC-PLACE-1a.
