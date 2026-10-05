# ARCH-CHEST-APPEARANCE-ADMIT-0: admitting the two provisional chest appearances

- Decision: `ARCH-CHEST-APPEARANCE-ADMIT-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings, the ADR-0021 §4.5 amendment and
  the packet. They implement accepted semantics only: A12 §4.1 (D146),
  `ARCH-WORLD-CONTENT-SERVE` §1.4, and the appearance-only Item admission of ITEM-KEY-I40450-1
  (#1795).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane's readiness request for CHEST-APPEARANCE-ADMIT-1. That request
  found that `ARCH-WORLD-CONTENT-SERVE` §1.4 names the packet but no packet body exists.
- Amends: ADR-0021 §4.5 (palette keys: Item keys only through a binding), in this PR.
- Content, runtime and production authority: NONE. The packet needs its #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **This PR** adds one palette-key note to ADR-0021 §4.5 and this packet. No code.
2. **Findings on `main` (`9cb66e38`):**
   - Two ready RewardClaims sit on appearances 28827 and 28828. Their palette entries in
     `content/world/placements/index.json` are provisional donor keys.
   - Neither id is in Crystal `items.xml`. Neither has an `ots/item_server_id` binding in
     `imports/crystalserver/bindings/items.json` or an Item record.
   - Both ids are in the admitted 15.30 appearance set (A12 §4.1).
   - Every other chest appearance of a served claim is an `oteryn:item.tibia.i<id>` Item key.
3. **Admission (§1.1).** The packet adds 28827 and 28828 to `APPEARANCE_ONLY_ITEM_IDS` (62 to
   64) with the same cascade as #1795.
4. **Binding (§1.2).** The G4 Crystal binding generator emits an `EXACT` `ots/item_server_id`
   binding for each of the two ids from the A12 §4.2 evidence: the Crystal map-appearance record
   and identity-projection continuity. The palette then resolves both ids through its unchanged
   binding step.
   - No allowlist and no new converter or validator step. Numeric equality alone never maps a
     source id.
   - Every other unbound id is unchanged, including every id held as `CONFLICT`, `AMBIGUOUS` or
     `NO_MATCH`.
   - Palette indices are kept, so no region file changes.
5. **Bundle.** The two chests are then compiled into `WorldBase`. Under §1.4 the two claims
   bind with no change to the binding code. The packet re-pins the bundle.
6. **CHEST-APPEARANCE-ADMIT-1 (impl worker, §2.1).** Allocate it after #1830 and #1805 merge,
   with one writer at a time on `content/world/pins/`.

## 1. Rulings

### 1.1 The two chests are admitted as appearance-only Items

A12 §4.1 makes a key the rule applied to an id in the admitted CipSoft set. 28827 and 28828 are
in that set, so `oteryn:item.tibia.i28827` and `oteryn:item.tibia.i28828` are their only possible
Item keys, and admitting them mints nothing.

Neither id is in Crystal `items.xml`, so they are appearance-only Items, like the 62 ids of
`APPEARANCE_ONLY_ITEM_IDS`. They take no TibiaWiki stats and no behaviour. The chest behaviour
stays the RewardClaim placement binding (ADR-0021 §4.5, D39).

### 1.2 Generator-emitted `EXACT` bindings map the two ids to their Item keys

The palette resolves a map server id through its `ots/item_server_id` binding. An appearance-only
Item has no binding, because the binding generator reads Crystal `items.xml`, so admitting the
Item alone leaves the palette entry provisional.

A Crystal map id is a source id. Under A12 §4.2 and G4 rules 3 and 6, a source id maps to an Item
only through an `EXACT` or `ACCEPTED_ALIAS` crosswalk row, and that row emits the binding. An id
held as `CONFLICT`, `AMBIGUOUS` or `NO_MATCH` stays unbound, and the same number is never enough
by itself. So the two ids get real `EXACT` bindings from committed A12 §4.2 evidence, and nothing
in the palette bypasses the binding.

The evidence for each id, all already committed:

| Id | Source appearance reference | Identity-projection continuity |
|---|---|---|
| 28827 | `imports/crystalserver/summer-update/map-content-linking/graphics/map-appearance-links.json` record `donor:crystalserver@00ce02a5:item/28827`: `source_appearance_id` 28827, `appearance_asset` the admitted 15.30 file (`2dfa943b`), palette slot 17073 | projection `31cfaf1c…1130a1` in the source's pinned file (`crystal-donor-00ce02a5`, Crystal `00ce02a5` `data/items/appearances.dat`) equals the projection in the newest admitted file (`client-15.30`); the records differ, so the row is `EVOLVED` |
| 28828 | same file, record `donor:crystalserver@00ce02a5:item/28828`: `source_appearance_id` 28828, same `appearance_asset`, palette slot 17074 | the same projection `31cfaf1c…1130a1` in both files; `EVOLVED` |

Projections and membership are read through `tools/content-schema/item-authoring/appearance_membership.py`
`load_admitted()`. Neither id is in Crystal `items.xml`, in the donor census or in the alias
crosswalk, so no crosswalk row exists for either in any other disposition.

`tools/content-census/g4_item_crystal_binding_generator.py` gains a map-appearance crosswalk
source, limited to these two ids. For each id it derives, from the committed inputs only:

1. the map-appearance record above, whose `source_appearance_id` equals the id and whose
   `appearance_asset` is the newest admitted file;
2. continuity: the identity projection of the id in the `crystal-donor-00ce02a5` file equals
   the one in `client-15.30`.

It then writes one crosswalk evidence row per id, with the same evidence fields as the
`content/items/aliases.json` donor entries, and emits an `EXACT` binding:

- `identity_namespace` `ots/item_server_id`, `external_id` the id;
- `source_key` `oteryn:source.crystalserver`, `source_revision`
  `00ce02a57ca5a12e48f32a3476e37471167e4c3f`;
- target `{Item, oteryn:item.tibia.i<id>, definition-r1}`.

The bound count goes from 33,971 to 33,973.

The new source fails closed and emits no binding when, for either id:

- the map-appearance record is missing, or names another appearance id or another file;
- either manifest lacks the id;
- the two projections differ. That is `CONFLICT`: the generator reports
  `ARCHITECTURE_ESCALATION_REQUIRED`, and the worker returns a BLOCKER;
- the id already has a crosswalk row or binding in any other disposition;
- the target Item record does not exist.

The palette order stays exactly as in ADR-0021 §4.5:

1. the binding's Item key, when it has an Item record. The two ids now resolve here;
2. the Terrain catalogue key, then the WorldObject catalogue key;
3. the provisional donor key.

`convert_world_base.py` and `validate_world_base.py` `check_palette` do not change, because
both already resolve Item keys through the binding map. The append-only palette keeps every
index and refreshes only the key. Its existing check that no two ids share a key still fails
closed.

The map-appearance source covers only these two ids. Any further donor map id needs its Item
record, its own decision and an independent identity review before the source covers it.

### 1.3 Checklist

1. Amendments go in the owning contract: the palette rule is in ADR-0021 §4.5. The binding rule
   (`ARCH-WORLD-CONTENT-SERVE` §1.4) already covers the result and is unchanged.
2. Concurrency is serialized: one writer per shared file.
   - #1830 rewrites the same content index, lock and WorldProject files.
   - #1805 owns `content/world/pins/`.
   - SPAWN-ADMIT-1 also re-pins.
   So the packet depends on #1830 and #1805, and it and SPAWN-ADMIT-1 never hold the pin at the
   same time.
3. Durable state is restart-sufficient: no durable state changes. The bundle and pins are
   content.
4. References are typed: the palette entry carries the A12 Item key through a typed G4
   binding. The donor key is not aliased.
5. Older peers are gated: a production pin still refuses any provisional key (ADR-0021 §4.5).
   This removes two such keys and adds none.
6. Split work is one unit: the admission, the palette and the re-pin land in one PR. A palette
   key with no Item record fails compilation, so they cannot land apart.

## 2. Packet

### 2.1 CHEST-APPEARANCE-ADMIT-1 (impl worker)

```yaml
task_id: OTV2-20261005-chest-appearance-admit-1
decision: this decision §1.1-§1.2; ADR-0021 §4.5; A12 §4.1-§4.2; G4 rules 3 and 6; ARCH-WORLD-CONTENT-SERVE §1.4
worker: oteryn-impl-worker
review: >-
  independent identity review (Codex, final frozen head), as A12 requires for Item-key
  admission; it covers the two Item admissions, the map-appearance crosswalk rows and the two
  EXACT bindings
branch: claude/chest-appearance-admit-1-20261005
base: main
depends_on: ["#1830 merged", "#1805 merged", "no open SPAWN-ADMIT-1 head holding content/world/pins/"]
migration_lease: none
owned_paths:
  - apps/game-server/src/content/item_identity.rs        # APPEARANCE_ONLY_ITEM_IDS 62 -> 64, digest pins
  - apps/game-server/src/content/cw2_b1_import.rs        # bound count 33,971 -> 33,973
  - apps/game-server/tests/content_item_identity.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/items/definitions/                           # the regenerated shards only
  - content/items/index.json
  - content/items/taxonomy/items.json
  - content/abilities/definitions/index.json
  - content/abilities/effects/index.json
  - content/abilities/formulas/index.json
  - content/behaviors/index.json
  - content/creatures/definitions/index.json
  - content/loot/index.json
  - content/presentations/definitions/index.json
  - content/manifest.json
  - content/content.lock.json
  - content/world/content.lock.json
  - content/world/definitions/reference.json
  - content/world/manifest.json
  - content/world/project.json
  - content/world/placements/index.json                  # palette keys of 28827 and 28828 only
  - content/world/pins/                                  # the bundle re-pin
  - content/world/provenance/imports.json                 # bindings digest
  - imports/crystalserver/bindings/items.json            # generated: the two EXACT bindings
  - tools/content-census/g4_item_crystal_binding_generator.py
  - tools/content-census/g4_item_crystal_binding_generator_self_test.py
  - tools/content-schema/world-authoring/README.md      # palette counts
  - tools/content-schema/item-authoring/samples/candy-decay-completion-20261002.json  # bindings digest
  - tools/content-schema/native-gameplay/build_spell_item_identities.py              # bindings digest
  - imports/ots-source-evidence/item-fx-audio295/         # bindings digest
  - imports/tibiawiki/facts/items-external-family18-20261001.json                   # bindings digest
  - docs/reference/spells/r21-local-candidate/active-artifact/source-closure-report.json  # bindings digest
  - tools/content-migration/                             # count and digest pins the cascade moves, as in #1795
  - imports/tibiawiki/facts/items-bounded7-navigation-20261002.json   # cascade output, as in #1795
  - imports/tibiawiki/facts/items-family-alias26-20261001.json        # cascade output, as in #1795
  - tools/content-schema/world-object-authoring/official_corpses.py   # qualification pins, as in #1795
  - tools/content-schema/world-object-authoring/qualified_world.py
  - tools/content-schema/world-object-authoring/samples/              # census and qualified samples
  - docs/agents/evidence/OTV2-20261005-chest-appearance-admit-repin-receipt-v1.json
  - docs/agents/evidence/OTV2-20261005-chest-appearance-admit-map-crosswalk-v1.json   # the two crosswalk rows
  - docs/agents/tasks/archive/OTV2-20261005-chest-appearance-admit-1.md
validation:
  - python tools/content-census/item_key_references.py
  - python tools/content-census/g4_item_crystal_binding_generator.py --check
  - python tools/content-census/g4_item_crystal_binding_generator_self_test.py
  - python tools/content-schema/world-authoring/test_world_base.py
  - python tools/content-schema/world-authoring/validate_world_base.py
  - the regeneration and checks #1795 ran (regenerate_content.py)
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server --quiet
  - cargo test --locked -p oteryn-world-bundle-compiler
  - python tools/agents/validate_governance.py
  - python tools/repository/validate_repository_policy.py
  - git diff --check
```

Builds:

- 28827 and 28828 in `APPEARANCE_ONLY_ITEM_IDS`, with the #1795 cascade and its re-pin receipt.
  Cite the 15.30 appearance membership `2dfa943b` as evidence.
- The §1.2 map-appearance crosswalk source in the binding generator, limited to 28827 and 28828,
  with its evidence file and the two `EXACT` bindings in `imports/crystalserver/bindings/items.json`.
  `EXPECTED_BOUND`, the self-test pin and the `cw2_b1_import.rs` count go from 33,971 to 33,973.
  Self-test cases:
  - each id emits exactly one `EXACT` binding at `00ce02a5` to `oteryn:item.tibia.i<id>`;
  - a missing or mismatched map-appearance record emits nothing and fails;
  - a projection that differs between the two files is `CONFLICT` with
    `ARCHITECTURE_ESCALATION_REQUIRED`, and emits nothing;
  - an id with an existing crosswalk row or binding in any other disposition fails;
  - an id with no Item record fails;
  - an id outside the two is never read by the source;
  - `--check` reproduces the committed output byte for byte.
- No converter or validator change. `python validate_world_base.py` passes on the regenerated
  palette through the unchanged binding step.
- The new bindings digest in each live pin listed in `owned_paths`. The worker updates a pin only
  when a check reads the current bindings file. A historical receipt under
  `docs/agents/evidence/` that recorded an earlier input is not rewritten.
- `world-authoring/README.md`: the palette counts are those of the regenerated capture summary.
  Appearance-only ids without a binding stay provisional, as today.
- The regenerated palette. Exactly two entries change, 28827 and 28828, and every index is kept.
  The worker's report states the before and after provisional counts.
- The bundle re-pin under the #1805 pin procedure.
- If the cascade moves a file outside `owned_paths`, the worker returns a BLOCKER naming it. It
  does not edit the file.

Acceptance: on the re-pinned non-production bundle, the two claims are bound
(`oteryn:reward-claim.quest.u15_24.targuna.mana_potions_chest`,
`oteryn:reward-claim.quest.u11_80.the_secret_library.small_islands.parchment`) and the
`NO_ENTRY` count falls by two. The worker reports the served-claim counts before and after. If
either claim stays unbound for any other §1.4 reason, the worker returns a BLOCKER with that
reason.

## 3. Rejected options

- **Hand-edit the two palette keys.** The next conversion would revert them, and the converter
  would then disagree with the committed palette.
- **A reviewed per-id allowlist in the converter (`APPEARANCE_PALETTE_IDS`).** It would map
  source ids to Items outside the binding, so the A12 §4.2 crosswalk row and its binding would
  not exist, and the converter and the validator would each need a second resolution path.
  (#1834 review on `1f4f6656`.)
- **Map any unbound id to the Item of the same number.** Numeric equality is not identity
  evidence (G4 rule 3), and it would promote ids held as `CONFLICT`, `AMBIGUOUS` or `NO_MATCH`.
- **Hand-add the two binding rows.** A row outside the generator has no reproducible evidence,
  and the next `--check` would reject it.
- **Map the chests as WorldObjects.** Every other served chest is an Item key. A second family for
  the same kind of entry would split the §1.4 rule for no gain.
- **Lift the provisional skip for these keys.** ADR-0021 §4.5 keeps provisional keys out of every
  build that could reach production. Admission is the path that section names.

## 4. Decision test

`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`:

1. **Must decide now? YES.**
   - `ARCH-WORLD-CONTENT-SERVE` §1.4 names CHEST-APPEARANCE-ADMIT-1, but no packet body exists.
     The control plane cannot allocate it.
   - Admitting the Items alone leaves the two palette entries provisional (§1.2). So the
     binding route must be fixed before the packet can make the two claims bind.
2. **What is blocked?**
   - CHEST-APPEARANCE-ADMIT-1.
   - Binding the two served RewardClaims, which stay `NO_ENTRY` until then.
   - Removing their two provisional keys from the path to a production pin.
3. **What gets harder later?**
   - Each further donor map id needs its Item record, its own decision and an independent
     identity review before the map-appearance source covers it. This is deliberate: numeric
     equality is not binding evidence (§1.2).
   - The two bindings are identity records. Undoing one follows the A12 retirement rules, not
     a plain revert. Palette indices never move, so no region file changes either way, and no
     wire or durable state is involved.
4. **What would justify superseding it?**
   - Evidence that a map record's placed object is not the CipSoft appearance it names, or a
     later admitted file whose projection breaks continuity. The id then becomes `CONFLICT`
     and its binding follows A12.
   - A general map-appearance crosswalk accepted for all donor map ids. The two-id limit would
     then be lifted by that decision.
   - A WorldObject or other family accepted for chests instead of Items (§3).
5. **What is deliberately not decided?**
   - Admission or binding of any other unbound or provisional palette id. Each needs its own
     Item record, decision and identity review.
   - Stats, behaviour or TibiaWiki facts for the two Items.
   - SPAWN-ADMIT-1 and the order of other re-pins, beyond one writer on `content/world/pins/`.
   - Whether production pins accept these two claims. That stays with the existing production
     pin procedure.

The rule holds when:

- a map id reaches the bundle as an Item only through an `EXACT` or `ACCEPTED_ALIAS` binding
  emitted from A12 §4.2 evidence;
- the palette keeps every index;
- the two claims bind under the unchanged §1.4 rule.
