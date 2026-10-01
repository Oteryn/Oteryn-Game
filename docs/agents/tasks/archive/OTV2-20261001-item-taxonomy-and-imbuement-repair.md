# OTV2-20261001-item-taxonomy-and-imbuement-repair

```yaml
task_id: OTV2-20261001-item-taxonomy-and-imbuement-repair
title: Repair source-qualified Item taxonomy, imbuement relations and nested authoring limits
mode: REPAIR
status: completed_pending_merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/item-catalog-audit-20261001
pr: 1437
base_sha: fafc51efd71e9f0e23e7725b75c1e0e86fe5f281
owner: owner-directed Codex session
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - tools/content-migration/{item_taxonomy.py,test_item_taxonomy.py,world_project_v2_to_tree.py,validate_world_project_v2_to_tree.py,test_world_project_v2_to_tree.py}
  - tools/content-schema/item-authoring/**
  - content/items/{taxonomy,relations}/**
  - content/content.lock.json
  - docs/agents/reports/OTV2-20261001-item-catalog-audit-and-map-handover.md
  - docs/agents/evidence/OTV2-20261001-client-1530-item-taxonomy-check.md
  - docs/agents/evidence/OTV2-20261001-quest-item-category-continuation.md
public_contracts:
  - OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1
depends_on: []
```

## Outcome and evidence

- PROVEN: taxonomy 164 -> 9,436, all 22 profiles; eleven throwing-weapon null profiles repaired.
  The supplements admit 81 validated family fallback rows, two official client
  market/clothing agreements and 18 Extra Slot equipment categories using the existing
  profile catalog. Contradictory or otherwise unadmitted newer wiki categories remain held;
  3,265 current Items have neither taxonomy nor an existing map owner.
- UNKNOWN: fresh contents of the owner's Quests/Quest Items/Quest Log URLs. Tavily returned
  432 (usage limit); direct HTTP and MediaWiki reads returned 402. Retained snapshots supply
  the category facts. i40522/i44432/i44433 identity holds and i2984 wiki conflict remain held.
  The quest-category follow-up creates no new Quest identities or runtime relations.
- PROVEN: all 656 positive imbuement-slot definitions have their relation; 582 missing edges
  repaired, total 729 relation sources / 803 edges. Forge/enchanting evidence remains retained.
- PROVEN: nested XML imbuement family ceilings survive parsing and candidate conversion.
  Malformed/duplicate children block conversion; absent children remain unknown.
- PROVEN: the formal schema rejects incompatible family/taxonomy classes and retains admitted
  ammunition, consumable and fluid-container variants. Missing common capabilities remain
  warnings under the existing authoring contract, not a new completeness acceptance rule.
- Wiki supplement uses `items-stats.json` snapshot digest
  `5fc20ff76f65ab8e0a6bb8f52bc366a0bc0b0b617aaaf3bc8b1c987604a2a2d6` with per-page revision
  and content digest. Existing BR rows, appearance-only holds and Terrain/WorldObject owners
  retain their boundaries. Crystal/Canary observations remain `OtsHypothesisOnly`.
- Ownership/merge/review routing: live #162 STATE was read. The owner's direct continuation
  request authorizes this repair; the active control plane retains integration and review
  dispatch. The draft remains open for review; no merge or external AI review was triggered.

## Validation

Tree regeneration/validator, materialized tree validator, tree tests and seven taxonomy
boundary regressions passed. The 252 formal-schema checks, 599 engine checks, three nested
limit regressions, Ruff checks/formatting, governance validator, 36 governance tests and
whitespace checks passed. Exact remote head and repository CI are recorded in PR/check
evidence after the atomic authoring write. Full-world E2E is not applicable to this batch.

Self-review covered source disagreement, unknown primary categories, appearance-only Items,
existing non-Item ownership, untouched legacy authoring, zero/unknown slot counts and nested
ceiling loss. This is navigation/authoring enrichment and generated relations; it performs
no live production mutation, recovery authorization, protocol or durable-identity change.

## Continuing work

The broad owner request continues with equipment requirements and further source qualification
as separate bounded batches. Native restriction admission, modifier/model gaps, seven ruleset
systems, held map identities and source conflicts are still open. This archived record describes
the repair batch in this PR, not completion of the entire catalogue audit.
