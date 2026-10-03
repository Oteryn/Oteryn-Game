# OTV2-20261003-p2-followups-batch3

```yaml
task_id: OTV2-20261003-p2-followups-batch3
title: Deferred P2 follow-ups batch 3 (NPC promotion validator)
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/p2-followups-batch3
pr: 1660
base_sha: d30e271
owner: implementation worker for the CP (#1622), D355
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/npc-authoring/test_promotion.py
  - docs/agents/tasks/archive/OTV2-20261003-p2-followups-batch3.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome

`validate_promotion.py` `errors()` rejects a `WIKI_MAJORITY_ARBITER` row whose directions and prices differ from the admitted offer its fact names. It also rejects a `WIKI_IMAGE` / `OWNER_REVIEW` row whose selected field differs from the pinned `REVIEWED_VALUES` value of its chosen source, without needing the rebuild inputs.

## Findings

| Finding | Verified on main `d30e271` | Disposition |
|---|---|---|
| #1358 4149546495: every admitted offer direction needs a status | reproduces: dropping Baltim's `BuyFromPlayer` entry passed | fixed |
| #1358 4149546503: enforce the value OWNER_REVIEW selects | reproduces: changing Storkus's outfit or Flickering Soul's movement passed | fixed (also covers WIKI_IMAGE) |

Batch-3 items outside this module, reported to the CP: #1435 4155353123 (wheel authoring, a separate module); #1391 4153900550 (premium consumer fence, authority); CYCLOPEDIA-0 4173086801 (the replacement snapshot carries in-range POIs, protocol).

## Excluded scope

No change to the sample report, to `promotion_candidates.py` output, to runtime, protocol or persistence.

## Validation

- `python -m unittest test_promotion.py` (in `tools/content-schema/npc-authoring`): 75 tests OK; both new tests failed before the fix
- `python -m unittest test_npc_authoring.py`: OK
- `python validate_promotion.py samples/promotion-candidates-v1.json`: 1112/1112 candidates valid
- `ruff check validate_promotion.py test_promotion.py`: clean
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: clean

## Self-review

Read the whole diff. All 42 arbiter rows in the sample match their offers exactly. The `REVIEWED_VALUES` pins are the six reviewed rows' current sample values, and the rebuild path binds them to the sources. A new reviewed row without a pin fails closed. Sorting uses `key=str`, so a malformed price never raises.

## Independent review

- required: per the bound review policy, on the frozen head (requested by the CP)

## PR and closeout

- PR: #1660. Exact frozen head: the one in the CP FREEZE entry. Merge commit/result: squash merge of #1660.
