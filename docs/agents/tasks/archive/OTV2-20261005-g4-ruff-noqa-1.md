# OTV2-20261005-g4-ruff-noqa-1

```yaml
task_id: OTV2-20261005-g4-ruff-noqa-1
title: Remove unused E402 noqa so G4 Item Crystal identity bindings ruff passes
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/g4-ruff-noqa-20261005
issue: 1622
pr: 1818
owned_paths:
  - tools/content-census/item_id_alias_table.py
  - tools/content-census/item_key_references.py
  - tools/content-migration/creature_admission_stage.py
  - docs/agents/tasks/archive/OTV2-20261005-g4-ruff-noqa-1.md
```

Ruff 0.16.1 (pinned) reports RUF100 for `# noqa: E402` on the `appearance_membership` imports. The comments are removed; behavior is unchanged.

## Validation

- `python -m ruff check` (pinned 0.16.1) on the workflow file list: pass
- `python -m ruff format --check` on the workflow file list: pass
- `python tools/content-census/g4_item_crystal_binding_generator_self_test.py`: pass
- `python tools/content-census/item_id_alias_table_self_test.py`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

Note: `item_key_references.py` also reports 2 DANGLING_KEY errors that exist on main independent of this change.
