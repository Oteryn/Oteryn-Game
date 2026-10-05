# ARCH-CHEST-APPEARANCE-ADMIT-0: admitting the two provisional chest appearances

- Decision: `ARCH-CHEST-APPEARANCE-ADMIT-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings, the ADR-0021 §4.5 amendment and
  the packet. They implement accepted semantics only: A12 §4.1 (D146),
  `ARCH-WORLD-CONTENT-SERVE` §1.4, and the appearance-only Item admission of ITEM-KEY-I40450-1
  (#1795).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane's readiness request for CHEST-APPEARANCE-ADMIT-1. That request
  found that `ARCH-WORLD-CONTENT-SERVE` §1.4 names the packet but no packet body exists.
- Amends: ADR-0021 §4.5 (palette keys), in this PR.
- Content, runtime and production authority: NONE. The packet needs its #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **This PR** adds one palette-key rule to ADR-0021 §4.5 and this packet. No code.
2. **Findings on `main` (`9cb66e38`):**
   - Two ready RewardClaims sit on appearances 28827 and 28828. Their palette entries in
     `content/world/placements/index.json` are provisional donor keys.
   - Neither id is in Crystal `items.xml`. Neither has an `ots/item_server_id` binding in
     `imports/crystalserver/bindings/items.json` or an Item record.
   - Both ids are in the admitted 15.30 appearance set (A12 §4.1).
   - Every other chest appearance of a served claim is an `oteryn:item.tibia.i<id>` Item key.
3. **Admission (§1.1).** The packet adds 28827 and 28828 to `APPEARANCE_ONLY_ITEM_IDS` (62 to
   64) with the same cascade as #1795.
4. **Palette rule (§1.2).** The world-base converter gives an id with no binding its A12 Item
   key, when that Item record exists, before the Terrain, WorldObject and donor fallbacks.
   - On `main` no provisional palette id has an Item record, so the rule changes exactly the
     two admitted ids.
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

### 1.2 An id without a binding takes its A12 Item key when the Item exists

The palette resolves a map server id through its `ots/item_server_id` binding. An appearance-only
Item has no binding, because the binding generator reads Crystal `items.xml`, so admitting the
Item alone leaves the palette entry provisional.

The converter therefore resolves in this order:

1. the binding's Item key, when it has an Item record (unchanged);
2. new: `oteryn:item.tibia.i<id>`, when that Item record exists;
3. the Terrain catalogue key, then the WorldObject catalogue key (unchanged);
4. the provisional donor key (unchanged).

Step 2 is sound because in the 15.x Crystal corpus the server id is the client appearance id
(`tools/content-census/g4_item_crystal_binding_generator.py`). It applies only to ids with no
binding, so it never overrides a binding. The append-only palette keeps every index and refreshes
only the key. Its existing check that no two ids share a key still fails closed.

The validator applies the same order. Today `validate_world_base.py` `check_palette` accepts a
non-provisional Item key only through the binding map, so it would reject the two new entries.
It gains step 2 from the same `defined` Item-key set: an unbound id whose
`oteryn:item.tibia.i<id>` Item record exists must use that key, and may no longer be
provisional. The converter and the validator change in the same PR, so neither accepts a palette
the other rejects.

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
4. References are typed: the palette entry carries the A12 Item key. The donor key is not
   aliased.
5. Older peers are gated: a production pin still refuses any provisional key (ADR-0021 §4.5).
   This removes two such keys and adds none.
6. Split work is one unit: the admission, the palette and the re-pin land in one PR. A palette
   key with no Item record fails compilation, so they cannot land apart.

## 2. Packet

### 2.1 CHEST-APPEARANCE-ADMIT-1 (impl worker)

```yaml
task_id: OTV2-20261005-chest-appearance-admit-1
decision: this decision §1.1-§1.2; ADR-0021 §4.5; A12 §4.1; ARCH-WORLD-CONTENT-SERVE §1.4
worker: oteryn-impl-worker
review: ordinary review on the final frozen head
branch: claude/chest-appearance-admit-1-20261005
base: main
depends_on: ["#1830 merged", "#1805 merged", "no open SPAWN-ADMIT-1 head holding content/world/pins/"]
migration_lease: none
owned_paths:
  - apps/game-server/src/content/item_identity.rs        # APPEARANCE_ONLY_ITEM_IDS 62 -> 64, digest pins
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
  - tools/content-schema/world-authoring/convert_world_base.py
  - tools/content-schema/world-authoring/validate_world_base.py   # check_palette step 2
  - tools/content-schema/world-authoring/test_world_base.py
  - tools/content-schema/world-authoring/README.md      # resolution order, validator order, counts
  - tools/content-migration/                             # count and digest pins the cascade moves, as in #1795
  - imports/tibiawiki/facts/items-bounded7-navigation-20261002.json   # cascade output, as in #1795
  - imports/tibiawiki/facts/items-family-alias26-20261001.json        # cascade output, as in #1795
  - tools/content-schema/world-object-authoring/official_corpses.py   # qualification pins, as in #1795
  - tools/content-schema/world-object-authoring/qualified_world.py
  - tools/content-schema/world-object-authoring/samples/              # census and qualified samples
  - docs/agents/evidence/OTV2-20261005-chest-appearance-admit-repin-receipt-v1.json
  - docs/agents/tasks/archive/OTV2-20261005-chest-appearance-admit-1.md
validation:
  - python tools/content-census/item_key_references.py
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
- §1.2 step 2 in `palette_entry`, fed from the same `defined` Item-key set the converter already
  builds. Tests:
  - an unbound id with an Item record takes its Item key;
  - an unbound id without one stays provisional;
  - a bound id is unchanged;
  - a key shared with a bound id fails.
- §1.2 step 2 in `validate_world_base.py` `check_palette`. Its `defined` set is the one the
  validator already builds for the binding map. Tests in `test_world_base.py`:
  - an unbound id with an Item record and its Item key passes;
  - the same id with a provisional, Terrain or WorldObject key fails;
  - an unbound id with no Item record and a provisional key passes, as today;
  - a bound id is still checked against its binding only.
  `python validate_world_base.py` passes on the regenerated palette.
- `world-authoring/README.md` matches the new rule:
  - the resolution order and the validator order name step 2: an unbound id with an
    `oteryn:item.tibia.i<id>` Item record takes that key;
  - appearance-only ids stay provisional only when they have no Item record;
  - the palette counts are those of the regenerated capture summary.
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
- **Add `ots/item_server_id` bindings for the two ids.** The bindings are generated from Crystal
  `items.xml`, where these ids do not exist. A hand-added row would be outside the generator's
  evidence.
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
     converter rule must be fixed before the packet can make the two claims bind.
2. **What is blocked?**
   - CHEST-APPEARANCE-ADMIT-1.
   - Binding the two served RewardClaims, which stay `NO_ENTRY` until then.
   - Removing their two provisional keys from the path to a production pin.
3. **What gets harder later?**
   - Every future appearance-only Item with a map placement takes its Item key through step 2.
     That couples the palette to the A12 key rule for unbound ids, which is the coupling A12
     already fixes.
   - Undoing it means one converter change and a re-pin. Palette indices never move, so no
     region file changes either way. No wire, durable or identity state is involved.
4. **What would justify superseding it?**
   - A Crystal or CipSoft corpus where a server id differs from its client appearance id. Step
     2 would then need a binding source, as `ots/item_server_id` is for bound ids.
   - A binding generator that reads a source beyond Crystal `items.xml` and covers these ids.
     Step 2 would then be redundant.
   - A WorldObject or other family accepted for chests instead of Items (§3).
5. **What is deliberately not decided?**
   - Admission of any other unbound or provisional palette id. Each needs its own admission.
     Step 2 only makes an admitted id reach the palette.
   - Stats, behaviour or TibiaWiki facts for the two Items.
   - SPAWN-ADMIT-1 and the order of other re-pins, beyond one writer on `content/world/pins/`.
   - Whether production pins accept these two claims. That stays with the existing production
     pin procedure.

The rule holds when:

- an unbound map id reaches the bundle only through an Item record that A12 §4.1 already makes
  unique;
- the palette keeps every index;
- the two claims bind under the unchanged §1.4 rule.
