# OTV2-20261005-g4-dangling-neg-1

```yaml
task_id: OTV2-20261005-g4-dangling-neg-1
title: G4 item key check - apps/ Tibia key above 65535 is synthetic
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/g4-dangling-neg-20261005
issue: 1622
pr: 1821
owned_paths:
  - tools/content-census/item_key_references.py
  - tools/content-census/item_key_references_self_test.py
  - docs/agents/tasks/archive/OTV2-20261005-g4-dangling-neg-1.md
```

In the `apps/` branch only, a Tibia key with id above 65535 can never be an appearance or Item id and is treated as synthetic. In-range non-admitted `apps/` keys still fail DANGLING_KEY; `content/` and `imports/` rules are unchanged.

## Validation

- `python tools/content-census/item_key_references.py`: pass
- `python tools/content-census/item_key_references_self_test.py`: pass
- `python -m ruff check` and `ruff format --check` on both files: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
