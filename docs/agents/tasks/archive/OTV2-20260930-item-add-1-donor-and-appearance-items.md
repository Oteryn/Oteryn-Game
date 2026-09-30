# OTV2-20260930-item-add-1-donor-and-appearance-items

```yaml
task_id: OTV2-20260930-item-add-1-donor-and-appearance-items
title: Materialize the donor epoch-2 Items and the appearance-only Items (ITEM-ADD-1)
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null
base_sha: d9c341ce
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session (worker of the coordinating session)
created_at: 2026-09-30T00:00:00Z
updated_at: 2026-09-30T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - apps/game-server/tests/content_item_identity.rs
  - content/world/content.lock.json
  - content/world/manifest.json
  - content/world/project.json
  - content/world/definitions/reference.json
  - content/world/provenance/imports.json
  - content/items/definitions/
  - content/items/index.json
  - content/abilities/definitions/index.json
  - content/abilities/effects/index.json
  - content/abilities/formulas/index.json
  - content/behaviors/index.json
  - content/creatures/definitions/index.json
  - content/loot/index.json
  - content/presentations/definitions/index.json
  - content/manifest.json
  - content/content.lock.json
  - imports/crystalserver/batches.json
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - tools/content-census/g4_item_crystal_binding_generator.py
  - tools/content-census/g4_item_crystal_binding_generator_self_test.py
  - docs/agents/tasks/archive/OTV2-20260930-item-add-1-donor-and-appearance-items.md
  - docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json   # regenerated after merging #1336
  - apps/game-server/src/content/item_stats_promotion.rs   # packet sha and counts
  - tools/content-schema/item-authoring/samples/item-weapon-proficiency-15-30-7fea90ec.json   # regenerated
  - imports/tibiawiki/{facts/items-stats.json,sources.json,batches.json}   # #1325 key rule re-applied (--rekey)
public_contracts: []
depends_on:
  - "docs/architecture/reviews/OTERYN_GAME_A12_ITEM_IDENTITY_TIBIA_ID_DECISION_2026-09-29.md"
  - "docs/architecture/reviews/OTERYN_GAME_A8_DONOR_ITEM_IDENTITY_EPOCH_DECISION_2026-09-28.md"
blocks:
  - creature admission of the 95 creatures deferred as unregistered_items
cross_repository_coordination_id: null
external_repositories: []
jira: null
```

## Outcome

95 creatures of creature admission wave A
(`docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json`,
`deferred.unregistered_items`) wait because they name 218 client item ids with no Item record
in content (50 corpse ids, 168 loot ids). This task makes 217 of them Items, so the creature
owner can re-run census, stage, materializer and tree and admit them. It does not change
creature admission.

- 404 Item records: the whole B1b donor identity epoch 2 (157 of the 218 ids are in it).
- 60 Item records: appearance-only ids (29 corpse, 31 loot).
- 48296 stays deferred (owner decision 4a).

Content goes from 33,567 to 34,031 Item records. No existing record, key, alias entry or
binding changed.

After #1336 (ITEM-SEM-2b-1) merged, the branch merged `main`. Two derived artifacts follow the
new Items:
- The TibiaWiki stat packet (`OTV2-20260930-item-stats-promotion-v2.json`, lowered from content
  Item ids) now covers 159 donor epoch-2 Items: 10,523 fields on 6,541 Items, up from 10,260 on
  6,382. `item_stats_promotion.rs` re-pins its sha and counts; earlier values are unchanged.
- The weapon proficiency sample (`item-weapon-proficiency-15-30-7fea90ec.json`) finds an Item
  definition for 22 more bindings (`bindings_without_item_definition` 23 → 1). No binding
  changed.
- The TibiaWiki stat snapshot is re-keyed offline (`item_wiki_stats_capture.py --rekey`, #1325
  key rule): records of ids that are now Items take their Item key. Observations are unchanged;
  `snapshot_sha256` becomes `5fc20ff7…a2d6` in `sources.json` and `batches.json`.

## Owner decisions (2026-09-30, via the coordinating session)

- **1a.** Add the donor-defined Items by the same import route: the pinned donor revision
  `zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f` (`summer-update` HEAD).
  No new pin.
- **2a.** The 60 appearance-only ids become minimal Items keyed `oteryn:item.tibia.i<id>`
  (A12 §4.1). An admitted CipSoft appearance is sufficient identity evidence. Semantics are
  `UNKNOWN` except what the appearance proves. The corpses among them decay by removal after
  the default corpse duration.
- **3a.** A self-decay (`decayTo` equal to the item's own id) is terminal: the item is removed
  when its duration elapses. This breaks the 52559 loop.
- **4a.** 48296 has no admitted appearance and stays deferred.
- Corpse decay chains must be complete: every `decayTo` target of an added item must itself be
  an Item.

## Architecture and source of truth

- PROVEN: B1b (#1179, `protected_cw2_b1_donor_identity_epoch_2_import` in
  `apps/game-server/src/content/cw2_b1_import.rs`, decision A8, re-keyed by A12/ITEM-ID-1b)
  minted identity for 404 donor ids of the donor revision above (items.xml sha256,
  LF-normalized, `13a8773e…`; census
  `tools/content-schema/item-authoring/samples/donor-census-crystal-summer-update-00ce02a5.json`).
  Each has an `ALIAS` entry in `content/items/aliases.json` to `oteryn:item.tibia.i<id>` and an
  `EXACT` binding in `imports/crystalserver/bindings/items.json`, but none had an Item record.
- PROVEN: all 404 are defined in the donor items.xml (local checkout verified against the
  pinned LF-normalized sha256) and all 404 are current in the client-15.30 manifest.
- PROVEN: 60 ids are defined in no items.xml (Crystal `ff7ede5`, donor `00ce02a5`, Canary
  main) and are current in the admitted CipSoft set
  (`imports/official/appearance-membership/admitted.json`, newest `client-15.30`).
  Corpse: 21887 35384 35388 35600 35846 36929 39949 43762 43771 43959 44447 44667 44671 44687
  44709 44713 44717 48108 48112 48271 48349 48353 48366 48382 48406 48414 48416 49124 51560.
  Loot: 40522 43666 43778 43779 43780 43781 43782 43946 43947 44048 44432 44433 44664 44665
  44666 44668 44669 44670 44684 44685 44686 48403 48404 48405 51276 51302 53197 53199 53201
  53203 53205.
- PROVEN: 48296 is in no admitted appearance manifest.
- PROVEN: item 52559 ("dead roaming dread") has `decayTo=52559`, `duration=300` in the donor
  items.xml. 49148 ("dead rootthing", duration 5) is the only other self-decay there. Both are
  existing epoch-1 Items. This task adds neither and changes neither.
- PROVEN: no Item record in content carries temporal semantics. `temporal` is `UNKNOWN` or
  absent on all 34,031 records, so no decay target or decay loop is materialized in content.
  Decisions 2a and 3a bind the Item temporal lowering when it lands. They need no record
  change now.

## Implementation

- Materializer (`materialize_content_world_project_v2.rs`):
  - runs `protected_cw2_b1_donor_identity_epoch_2_import` on its digest-pinned census and
    alias-gate crosswalk;
  - checks the minted count (404) and the allocation digest;
  - adds the 404 records and their source ids before the A12 §4.1 switch, which re-keys them
    through their existing alias entries;
  - adds the donor batch `cw2-b1-donor-identity-epoch-2-r1` to the provenance imports, with
    candidates cleared as for epoch 1.
- The batch gets no v2 source. `oteryn:source.crystalserver` at `00ce02a5` already names the
  NPC supplement batch, and nothing binds through a donor Item source.
- Appearance-only records join after the switch, because §4.1 keys have no alias entry. The
  materializer checks the digests of `admitted.json` and the client-15.30 manifest. Each id
  must be current, be unknown to the CW2-B1 and epoch-2 source ids, and not be a record yet.
- Every new record is identity-only, like the epoch-1 identity records: `materializable:
  false`, `stack_class: Unknown`, no semantics.
- The content tree was regenerated with `world_project_v2_to_tree.py` (69 Item shards). The
  tree validator and its test now pin 34,031 Items.
- `g4_item_crystal_binding_generator.py`: its definition-key closure now also admits the
  epoch-2 targets and the keys of current CipSoft ids that no Crystal row names. The binding
  output is unchanged (33,971 bindings, same bytes). A self-test covers the rule.
- `content_world_project_repository.rs`:
  - re-pins the package documents, `TREE_SHA256`, the Item count and the import batch
    indices;
  - asserts the donor batch provenance, one donor key, one appearance-only key, and that
    48296 is absent.
- `content_item_identity.rs` re-pins the canonical Item key count.

## Acceptance criteria

- [x] All 217 target ids (218 less 48296) are Item records.
- [x] Every `decayTo` target of the 404 added donor items is an Item. The check is transitive
  over the donor items.xml, 59 of them have decay chains, and none is missing or loops. The
  60 appearance-only items have no `decayTo` in any items.xml, so they decay by removal (2a).
- [x] 52559 loops nowhere in content. Content carries no decay, and 3a governs its lowering.
- [x] No existing Item record, key, alias entry or binding changed. The 56,464 previous
  records are byte-identical and in the same order.
- [ ] Independent identity review on the frozen head. It is required because this change
  mints 60 new canonical keys under A12 §4.1.

## Excluded scope

- Re-running creature census, stage, materializer and tree to admit the 95 creatures. That is
  for the creature owner. The stage's item map
  (`export_reference_item_identity_map`, epoch 1 only, pinned allocation digest) does not yet
  name the 464 new ids.
- Item temporal/decay lowering, and any change to ITEM-ID-1 migration logic or existing keys.

## Validation

All of the following PASS on the local candidate:

- `cargo fmt --all --check`.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`.
- `cargo test --test content_world_project_repository`: 3 passed.
- `cargo test --test content_item_identity`: 5 passed.
- `cargo test --lib content::`: 106 passed.
- `validate_world_project_v2_to_tree.py` and `test_world_project_v2_to_tree.py`.
- `validate_materialized_game_tree.py`.
- `item_key_references.py`.
- `g4_item_wave1_stage.py --check`.
- `g4_item_crystal_binding_generator.py --check` and its self-test.
- `item_id_alias_table.py --check`.
- `npc_admission_stage.py`: the re-staged output is byte-identical to the committed packet.
- `validate_repository_policy_core.py`, `validate_governance.py`, `git diff --check`.

## Independent review

- required: YES. This change mints identity for 60 new canonical Item keys
  (A12 §4.1, owner decision 2a) and materializes 404 records for existing epoch-2 keys.
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #1179 (B1b), #1305 (ITEM-ID-1b)
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending
