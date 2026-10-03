# OTV2-20261003-p2-followups-batch6

```yaml
task_id: OTV2-20261003-p2-followups-batch6
title: Deferred P2 follow-ups batch 6 (NPC promotion validator)
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/p2-followups-batch6
pr: 1666
base_sha: 3d1e97b
owner: implementation worker for the CP (#1622), D359
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/npc-authoring/test_promotion.py
  - docs/agents/tasks/archive/OTV2-20261003-p2-followups-batch6.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome

Reviewed values are compared as exact JSON, and the docstring states that only the rebuild binds an offer's source provenance.

## Findings

| Finding | Verified on main `3d1e97b` | Disposition |
|---|---|---|
| #1660 4174148287: type-sensitive equality | reproduces: `floor_change: 0` and `addons: false` passed | fixed |
| #1660 4174148280: relabelled source direction | a limitation without the source bundles | documented (rebuild-only) |
| #1358 4145434054: score of the arbitrated colours | already fixed on main (`arbitrate()` scores `out`) | no-op |
| #1358 4146112903: canonical palette rounding | already fixed on main (`PAL` uses `outfit_color()`) | no-op |

## Excluded scope

No sample report, runtime, protocol or persistence change.

## Validation

- `python -m unittest test_promotion.py` (in `tools/content-schema/npc-authoring`): 75 tests OK; the new cases fail without the fix
- `python -m unittest test_npc_authoring.py`: OK
- `python validate_promotion.py samples/promotion-candidates-v1.json`: 1112/1112 candidates valid
- `ruff check validate_promotion.py test_promotion.py`: clean
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: clean

## Self-review

`same_json` uses `json.dumps(sort_keys=True)`, so dict order does not matter but types do. The pinned values and the sample are unchanged.

## Independent review

- required: per the bound review policy, on the frozen head (requested by the CP)

## PR and closeout

- PR: #1666. Exact frozen head: the one in the CP FREEZE entry. Merge commit/result: squash merge of #1666.
