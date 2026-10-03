# OTV2-20261003-main-red-content-fix

```yaml
task_id: OTV2-20261003-main-red-content-fix
title: Regenerate taxonomy and world base left stale by #1628 (D362)
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/main-red-content-fix
pr: 1664
base_sha: d30e271
owner: implementation worker for the CP (#1622), D362
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - content/items/taxonomy/items.json
  - content/world/placements/index.json
  - content/world/areas/islands/index.json
  - tools/content-schema/world-authoring/samples/world-base-capture-v1.json
  - tools/content-schema/world-authoring/samples/appearance-only-ids-v1.json
  - tools/content-schema/world-authoring/samples/islands-capture-v1.json
  - tools/content-schema/world-authoring/convert_world_base.py
  - tools/content-schema/world-authoring/test_world_base.py
  - docs/agents/tasks/archive/OTV2-20261003-main-red-content-fix.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome

The world-metadata and content-tree validators pass on main again. The only changes are regenerated derived files.

## Findings

| Check | Breaking commit | Cause | Disposition |
|---|---|---|---|
| validate_world_base, palette id 35600 | #1628 `1ea4868` (also fails on its head `88edf552`) | #1628 added WorldObject i35600; the palette kept the provisional donor key | fixed: `convert_world_base.py` (pinned crystalserver `00ce02a5`, pinned tibiamaps), then `convert_appearance_only_ids.py` and `convert_islands.py`. Codex P1 5972107907 (D377): the first candidate gave it the WorldObject key, which the bundle compiler cannot resolve because that record points at Item i35600; `catalogue_keys` now returns the pointed Item key for a catalogue record with an `item_pointer` (Q1b as `resolve.rs` applies it), so 35600 takes `oteryn:item.tibia.i35600`; regression test added |
| validate_world_project_v2_to_tree, TAXONOMY_SOURCE_COVERAGE | #1628 merged after #1596 `d05a459` (passes on its own head) | #1596 changed the Item definitions; 119 `source_evidence` rows held a stale `canonical_definitions_sha256` | fixed: `regenerate_content.py` |
| G4 item_key_references, 14 RETIRED_KEY | #1628 (also fails on its head) | owner-review `reason` prose quotes retired registry keys, copied from `owner-item-family-decisions.json` | not fixed: needs a decision, reported to the CP |

## Excluded scope

No edits to owner decision text, to checks, or to code.

## Validation

- world-authoring: the 8 workflow checks PASS; 6 test files OK (the new item-pointer test fails without the fix); `ruff check` and `ruff format --check` clean
- content tree: `test_world_project_v2_to_tree` and 5 taxonomy/navigation tests OK; `validate_world_project_v2_to_tree` PASS
- `test_engine_items` PASS; `g4_item_crystal_binding_generator.py --check` PASS; `starter_kit_authoring.py content --check` ok
- `tools/content-census/item_key_references.py`: 14 RETIRED_KEY (excluded)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: clean

## Self-review

Compared each regenerated file field by field with main. Palette: one entry changed (19313, to the Item key). World-base summary: item +1, provisional -1. Every other pointed catalogue record already resolves to its bound Item key, so nothing else moves. Appearance-only: one id leaves the decoration class. Islands: the base-map sha256 is re-pinned. Taxonomy: only `canonical_definitions_sha256` in 119 rows. No old digest is pinned anywhere else.

## Independent review

- required: per the bound review policy, on the frozen head (requested by the CP)

## PR and closeout

- PR: #1664. Exact frozen head: the one in the CP FREEZE entry. Merge commit/result: squash merge of #1664.
