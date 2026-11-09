# M20 crafting qualification — PARTIAL

Base: `b7da07f2b0d38836b60018a2ecd20c4149fb7ff9`.
Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`
(`OTS_HYPOTHESIS_ONLY`, selected by the task owner).

The base already contains the first Cursed Crystal carried recipe in
`content/quests/carried-recipes.json`, the final recipe's two operand orders in
`content/quests/npc-callbacks/m20-cursed-crystal-recipes.json`, and the existing
captured-ingredient craft receipt extension in migration 2030. These definitions
and the receipt are retained. No second crafting system or migration is added.

Molten Wax uses the existing carried quest-tool receipt and exact self-target
proof. Item 9106 consumes one unit, raises Oneeyedjoe to 2 only from values below
2, and requires the user's own current entity inside x=31938..31946,
y=32932..32940, floor 10. This is the strict distance-under-five predicate around
(31942,32936,10) in donor
`data-global/scripts/quests/the_cursed_crystal/actions_MedusaOil.lua:29-43`.

## Donor blocker

The complete donor Git tree is untruncated (9,718 entries). Its extracted Lua
sources contain no Nutrient Solution, Bottled Yolk, Fresh Hydra Herbs,
Anti-Mutagen, conservation-jar, fish-flake blender or goldfish-feeding action for
the two requested quests. The only Cradle quest script is
`data-global/scripts/quests/cradle_of_monsters/the_monster_fight.lua`; it defines
the boss encounter (`:1-39`), not the laboratory recipes. Scatterbrained's sole
quest-specific declaration is the empty
`TheScatterbrainedSorcerer = {}` in `data-global/lib/core/storages.lua:1587`.
Fish flakes are item 9109 (`data/items/items.xml:24458`), but item 9109 has no
reference in any donor Lua file; the missing blend/feed logic is not merely
hidden behind numeric item IDs.

Consequently, exact donor parity cannot establish the missing recipes' item
identities, quantities, station selectors or collection consequences. Cradle
s8/s9, Scatterbrained s3/s4, multi-output recipe qualification and their container
stage consequences remain blocked until a pinned implementation source is
provided or another behavior source is authorized. No substitute data is authored.

## Native Item admission blocker

The final Cursed Crystal mix's native route also remains closed. Its source
crystal dust (21507) and output Medusa's Ointment (21506) have no native Item
profiles in `content/items/definitions/spell-native-profiles.json` or its Item
profile supplements. Only the first mixture (21505) is admitted there. The
attempted real final-mix caller reaches `Unavailable(InvalidStoredState)` when
the existing reward MINT owner resolves the unadmitted output. No admission is
fabricated and no Item guard is weakened. Item profiles and their registration
are outside this lane's owned paths; the temporary caller opening was removed.

The source's Molten Wax message and yellow-smoke effect (`actions_MedusaOil.lua:40-41`)
are not emitted by the existing carried callback adapter. This delivery proves
the item debit, self identity, area and source storage consequence, not full
visual feedback parity or a complete quest route.

## Acceptance and local qualification

| Criterion | Result and evidence |
| --- | --- |
| 1 | PARTIAL: the existing first Cursed Crystal recipe debits both carried inputs and grants its output atomically, refuses a missing carried target without changing item snapshots, and replays one craft receipt. `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/m20_cursed_crystal.rs:203`. Multi-output crafting remains unqualified; the existing qmech first-recipe implementation was not changed. |
| 2 | BLOCKED: no pinned Cradle brew or Scatterbrained blend action exists in the donor audit above. No substitute world-tool recipe or dispatch stub is added. |
| 3 | BLOCKED: the conservation-jar and goldfish-feed consequences have no implementation in the pinned donor. |
| 4 | PASS for value/state semantics: `content/quests/npc-callbacks/m20-cursed-crystal-wax.json:6`, registered in `crates/gameplay/quests/src/quest/reward_callbacks.rs:343`; unit boundary/stage proof in `reward_callbacks/recipe_tests.rs:4`; real self-use, stale-entity rejection, one-item debit, replay and later-command rejection in `postgres_tests/m20_cursed_crystal.rs:9`. Visual feedback remains outside this qualification. |
| 5 | PASS: the mounted PostgreSQL cases return `Ok(())` without the admin URL and ran against the retained PostgreSQL 18.6 cluster. Wax: 60.683295796 s; existing first-mixture atomic debit/grant: 61.763047947 s. Both passed with real owner calls and no ignored tests. |
| 6 | PARTIAL: Cursed Crystal self-use is wired. Cradle and Scatterbrained content remain blocked. Authoring checks/unit tests and content-tree validation pass; full regeneration hits the baseline failure below. |

Local results on the published source tree:

- `cargo test -p oteryn-gameplay-quests --lib m20_`: 4 passed. The new wax test was first red with `missing Molten Wax callback`.
- `cargo test -p oteryn-gameplay-dispatch --lib m20_ -- --nocapture`: 2 PostgreSQL cases passed. The first recipe is existing qmech behavior, verified without changing its implementation or receipt.
- `run_checks.py --part checks`: passed.
- `run_checks.py --part unit-tests --jobs 2`: 114 modules, 1,146 cases, no failed modules; 8 existing skips.
- `validate_world_project_v2_to_tree.py`: passed both independently and in regeneration.
- `cargo check --workspace --tests`: passed, 8m 13s.
- Clippy for quests and dispatch, all targets/features with `-D warnings`: passed, 5m 43s.
- Cargo formatting and the mounted PostgreSQL file's rustfmt check: passed.
- World `pin-check`: passed, unchanged world digest `7b680a6007b27dff037ef35cfdb4bd97deec63b32e47c88cc597a603cd28a41c`. No pin refresh is required.
- Duplicate migration-number check: clean. No new migrations or receipt system.

Full `regenerate_content.py --resolve --no-stage` materialized the content and
passed content-tree validation, then failed in the existing migration-tool test:

```text
FAIL: test_registration_replays_all_current_outputs
File "tools/content-migration/test_monster_seven_spell_overlay.py", line 54
self.assertEqual(len(keys),63)
AssertionError: 64 != 63
```

The task base already contains 64 QENGINE reward Item profiles. Regeneration also
updates the same stale expected count at
`crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs:364,430`.
That automatic edit was restored because existing equipment tests and migration
tools are outside this lane's ownership. No failed check was weakened or skipped,
and no unrelated generated changes are included.
