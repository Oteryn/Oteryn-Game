# OTV2-20260928-item-donor-wiki-evidence-b2

```yaml
task_id: OTV2-20260928-item-donor-wiki-evidence-b2
title: B2 wiki evidence for the epoch-2 donor ids
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 1191
base_sha: 86116adf
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session (Item authoring lane)
created_at: 2026-09-28T22:55:00Z
updated_at: 2026-09-28T23:30:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/donor_census.py
  - tools/content-schema/item-authoring/test_donor_census.py
  - tools/content-schema/item-authoring/test_engine_items.py
  - tools/content-schema/item-authoring/README.md
  - tools/content-schema/item-authoring/samples/donor-census-crystal-summer-update-00ce02a5.json
  - tools/content-schema/item-authoring/samples/population-canary-47dfd51f.json
  - tools/content-census/item_wiki_family_capture.py
  - imports/tibiawiki/facts/items-family-fallback.json
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - docs/agents/tasks/archive/OTV2-20260928-item-donor-wiki-evidence-b2.md
public_contracts: []
depends_on:
  - "B1b #1179 (ff498b58): epoch-2 donor Item keys in imports/crystalserver/bindings/items.json"
blocks:
  - "B3 (15.30 appearance metadata)"
cross_repository_coordination_id: null
external_repositories:
  - "zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f (summer-update donor, facts only)"
  - "tibia.fandom.com (current revisions at capture time; pinned per record)"
jira: KAN-16
```

## Allocation

The claim is #162 comment 5880183663. B2 returns to the Item authoring lane per 5876385001, and the owner's standing
direction is to announce and proceed.

## Outcome

- **Donor census.** `donor_census.py` reads each donor id's committed key (`registry_key`, B1b epoch 2), and every row
  carries `key`. Wiki evidence and the owner table therefore join under that key. The 8 held ids (PROBABLE_MATCH and
  AMBIGUOUS) keep the provisional `donor:` key.
- **Wiki capture.** `item_wiki_family_capture.py --donor-only --donor-source` captures exact-itemid evidence for the
  17 donor ids that reach the wiki step and appends the records to the snapshot. The existing 1,482 records stay
  byte-identical, and a key collision is a hard error.
- **Result.** 8 records were appended:
  - 53695 and 54480 → `material_valuable` (Valuables);
  - 54638 and 54651 → `tool` (Taming Items);
  - 50213–50216 `sickbed` → `decoration` (Furniture), the same profile they already had, now through wiki evidence
    instead of the wrap target.

  9 ids with an empty `primarytype` stay unresolved.
- **Donor census totals.** 265 resolved, 138 routed, 9 unresolved (was 261 / 138 / 13).
- **Snapshot pins.** The snapshot digest is now `c47aadf2…`, updated in `imports/tibiawiki/{batches,sources}.json`,
  and the capture tool's `mapper_sha256` was re-pinned.
- **Canary sample.** `population-canary-47dfd51f.json` was regenerated. It was already stale on main since B1b gave 9
  of Canary's own ids a key. Its wiki count is unchanged.
- **Formal doc §5l** records the status.

## Architecture and source of truth

- **Unchanged, byte-identical:**
  - the Crystal population census;
  - the promotion packet;
  - every pinned-engine snapshot record.
- **Not touched:**
  - identity, the bindings, `apps/**`, `content/**`.
- **Sources:** the Fandom evidence is pinned per record (page id, revision id, sha1, content sha256), and upstream
  stays a source of facts only.

## Validation

- `donor_census.py --self-check` and `--check`: PASS, with converter parity over 36,632 ids.
- `population_census.py --check` for Crystal: PASS. Canary was regenerated, and its `--self-check` passes.
- `lower_promotion_packet.py --check`: PASS.
- `test_engine_items.py`: 593 checks PASS. `test_donor_census.py`: 47 checks PASS. `test_lower_promotion_packet.py`
  and `test_world_objects.py`: PASS.
- `build_formal_schema.py` output is byte-identical, and `verify_formal_schema.py` passes 242 of 242.
- The Item Master census passes, as do ruff 0.16.1 `check` and `format`.
- `validate_materialized_game_tree.py`: PASS (97/97).
