# questpc5 final local evidence

AUTHORING on detached local base `28e4313e1`; no frozen candidate or publication. All 33 allocated titles have final ledger rows:3 QUALIFIED original-source contracts and30 BLOCKED on concrete shared engine pieces. The separately authored approximation stage graphs and a deployed World are not certified.

## Changed product files

- `apps/game-server/migrations/1004_fathers_burden_items.sql`
- `content/content.lock.json`
- `content/items/definitions/spell-native-profiles.json`
- `content/items/index.json`
- `content/manifest.json`
- `content/spells.manifest.json`
- `crates/gameplay/dispatch/src/gameplay_transport/npc_command/catalogue.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc5_reward_paths.rs`
- `crates/gameplay/durability/src/durability/character_revision_sequencer/npc_program.rs`
- `crates/gameplay/durability/src/durability/quest_state/npc_program/fathers_burden.rs`
- `tools/content-migration/questpc5_father_item_profiles.py`
- `tools/content-migration/register_spell_families.py`
- `tools/content-migration/test_questpc5_father_item_profiles.py`

Lane records: QUEST-STATUS.json, RESUME.md, SHARED-FIXES.md, REVIEW.md, final-status-audit.json and the named logs/results.

## PostgreSQL evidence

- QUALIFIED: Alawar's Vault Quest — `questpc5_alawars_vault_lever_both_portals_and_all_claims`, `POSTGRES_CASE_COMPLETED questpc5_alawars_vault_lever_both_portals_and_all_claims duration=123.598361507s`; `.questpc5/reward-paths-green.log`.
  Scope: Original source lever/portal/reward contract; authored approximation stage graph and deployed World are not certified.
- QUALIFIED: Behemoth Quest — `questpc5_behemoth_lever_and_all_four_rewards`, `POSTGRES_CASE_COMPLETED questpc5_behemoth_lever_and_all_four_rewards duration=72.085102686s`; `.questpc5/reward-paths-green.log`.
  Scope: Original source lever/portal/reward contract; authored approximation stage graph and deployed World are not certified.
- QUALIFIED: Braindeath Quest — `questpc5_braindeath_coffin_exact_backpack_contents_replay_and_refusal`, `POSTGRES_CASE_COMPLETED questpc5_braindeath_coffin_exact_backpack_contents_replay_and_refusal duration=30.111469623s`; `.questpc5/braindeath-initial.log`.
  Scope: Original standalone Braindeath coffin source contract (Beregar BPTheUndeathStare alias), including all exact nested rewards. The separate authored approximation kill counter and deployed World are not certified.
- Partial Father hand-in proof: `fathers_burden_eight_handins_cape_experience_and_replay`, `POSTGRES_CASE_COMPLETED fathers_burden_eight_handins_cape_experience_and_replay duration=69.439690489s`; `.questpc5/father-guard-green.log`. Eight exact item debits, progress, 28,000 XP, cape and replay pass. The 8,000 gold and dungeon acquisition paths are not certified; the complete-quest row remains BLOCKED.

Every PostgreSQL command used CARGO_BUILD_JOBS=3, the absolute queued wrapper, --locked/--offline, and a mechanic filter. No whole PG suite ran. Exact executed filters:

```sh
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib questpc5_ -- --nocapture --test-threads=1
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib questpc5_braindeath_ -- --nocapture --test-threads=1
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib fathers_burden_eight_handins_cape_experience_and_replay -- --nocapture --test-threads=1
```

The first filter selected the two lever cohorts when run; Braindeath was added afterward and ran with its own filter. All 3 source contracts have positive markers. Father RED at2704yes was traced to the 1004 PL/pgSQL local/column amount ambiguity and repaired by a local-variable rename, with no guard weakened.

## Other validation

- Dispatch and durability Clippy, --all-targets --all-features -- -D warnings: PASS, final-clippy-repaired.log. The inherited eight-argument NPC bridge required its narrow documented allowance; no signature or behavior changed.
- Explicit rustfmt check of all changed auto-mounted dispatch files and the sequencer bridge: PASS, final-mounted-fmt.log.
- Native profile source-closure Python unit: 1 passed; generator register_spell_families.py --check:PASS. Exactly 9 new native profiles,0 existing profile changes.
- Content native units: 32 passed, 0 failed, 1 pre-existing ignored full-source-manifest test; no ignore was added or changed.
- Server integration_rest content_: 123 passed, 0 failed.
- `server-clippy`: exit 0, 125.701s wall; `.questpc5/final-server-clippy.log`.
- `content-native-units`: exit 0, 274.389s wall; `.questpc5/final-content-native-units.log`.
- `server-content`: exit 0, 130.677s wall; `.questpc5/final-server-content.log`.
- `world-project`: exit 0, 96.137s wall; `.questpc5/final-world-project.log`.
- `fmt`: exit 0, 8.810s wall; `.questpc5/final-fmt.log`.

Exact remaining cargo commands (CARGO_BUILD_JOBS=3):

```sh
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-game-server --all-targets --all-features --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-content --lib content::native_gameplay::tests:: -- --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test integration_rest content_ -- --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test content_world_project_repository -- --test-threads=1
/home/mole/oteryn-cache/codex-bin/cargo fmt --all --check
```

- World project repository: 5 passed, 0 failed.

## Unfinished

30 complete quest paths remain BLOCKED. These are engine dependencies with exact file:line references and attempts in QUEST-STATUS.json; none is blocked for missing tests.26 depend on qengine2 (NPC item/value composition or outfit/addon grants), 4 on the allocated qengine encounter/reset owner. All 139 blocker references resolve.

- A Father's Burden Quest: The pinned complete reward includes8000gold in addition to Old Cape and28000XP. The current Tereban completion plan gives only one cape plus8000terminalXP; reward_claim_mint::fathers_burden::claim encodes one noncontainer root with no children. The shared atomic NPC multi-output inventory/reward composition is QENGINE2-owned; a second independent gold write would not share the completion receipt/rollback boundary, and a second durable owner is prohibited.
- A Piece of Cake: QENGINE2-owned atomic NPC inventory/quest-progress composition for Biff the Baker: 20 Sweet and Sugary Substance and Cake Backpack. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- A Pirate's Death to Me: QENGINE2-owned atomic NPC inventory/quest-progress composition for Smaralda: Scum Bag custody and Pirate Bag/Platinum Coin/Pirates Surprise rewards. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- A Pirate's Tail Quest: QENGINE2-owned durable NPC outfit/addon grant: Rascoohan Outfits. Production evaluate_source passes an empty outfit snapshot and uses plan_storage, whose effect whitelist rejects Outfit/OutfitAddon. This shared owner is explicitly allocated elsewhere; implementing a second durable grant path in this lane is forbidden.
- Adventures of Galthen Quest: QENGINE2-owned durable NPC outfit/addon grant: Royal Bounacean Advisor Outfits. Production evaluate_source passes an empty outfit snapshot and uses plan_storage, whose effect whitelist rejects Outfit/OutfitAddon. This shared owner is explicitly allocated elsewhere; implementing a second durable grant path in this lane is forbidden.
- Afflicted Outfits Quest: QENGINE2-owned durable NPC outfit/addon grant and atomic hand-in composition. The production source writer supplies an empty outfit snapshot and rejects Outfit/OutfitAddon effects; a lane-local duplicate would violate the explicit shared-engine allocation.
- Against the Spider Cult Quest: QENGINE2-owned atomic NPC inventory/quest-progress composition for Tibia Tales contacts: the accepted anthology material hand-ins and value-bearing completion. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- An Interest In Botany Quest: QENGINE2-owned atomic NPC inventory/quest-progress composition for Rabaz: Giant Dreadcoil/Giant Verminous samples and Sedge Hat/1000 gold reward. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- An Uneasy Alliance Quest: QENGINE2-owned atomic NPC inventory/quest-progress composition for Curos: milk/steak/flute obligations and the two Tome of Knowledge reward. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Ancient Aucar Outfits Quest: QENGINE2-owned durable NPC outfit/addon grant and atomic hand-in composition. The production source writer supplies an empty outfit snapshot and rejects Outfit/OutfitAddon effects; a lane-local duplicate would violate the explicit shared-engine allocation.
- Annual Autumn Vintage: QENGINE2-owned atomic NPC inventory/quest-progress composition for Emilie/Gruffy: winterberry/wine custody and the six potion outputs. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Arito's Task Quest: QENGINE2-owned atomic NPC inventory/quest-progress composition for Arito/Muhad: nomad parchment/treaty hand-in and access progress. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Assassin Outfits Quest: QENGINE2-owned durable NPC outfit/addon grant and atomic hand-in composition. The production source writer supplies an empty outfit snapshot and rejects Outfit/OutfitAddon effects; a lane-local duplicate would violate the explicit shared-engine allocation.
- Asura Palace Quest: Personal encounter admission and kill-to-door completion for the Destroyer/Greater Elemental groups and Flaming Orchid. Callback supports USE/STEP, map effects and character storage, but carries no participant-bound encounter token or eligible-kill completion callback. This requires the shared encounter/reset owner allocated to QENGINE (NOTE-coordination.md:16), including shared callback/capture integration; it cannot be supplied by an isolated quest module or by test-only progress injection without duplicating that owner.
- Awash World Change: Reset/pickup and encounter access for three water pumps and the personal Sunken Mines steamship route. The callback owner has no pump encounter/reset lifecycle or character-qualified travel-route publication operation; a generic teleport is not proof that the mine/scout/pump obligation unlocked the route. This requires the shared encounter/reset owner allocated to QENGINE (NOTE-coordination.md:16), including shared callback/capture integration; it cannot be supplied by an isolated quest module or by test-only progress injection without duplicating that owner.
- Bank Robbery Mini World Change: QENGINE2-owned atomic NPC inventory/quest-progress composition for Thais banker: Bag With Stolen Gold debit and ten platinum coins/Honest Finder completion. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Barbarian Arena Quest: Participant-bound arena encounter admission and per-room kill completion across Greenhorn/Scrapper/Warlord. Existing source reward claims grant the nine reward choices, but neither Callback nor the generic NPC storage writer owns arena occupancy, combatant identity or exit/completion eligibility. This requires the shared encounter/reset owner allocated to QENGINE (NOTE-coordination.md:16), including shared callback/capture integration; it cannot be supplied by an isolated quest module or by test-only progress injection without duplicating that owner.
- Barbarian Outfits Quest: QENGINE2-owned durable NPC outfit/addon grant and atomic hand-in composition. The production source writer supplies an empty outfit snapshot and rejects Outfit/OutfitAddon effects; a lane-local duplicate would violate the explicit shared-engine allocation.
- Barbarian Test Quest: QENGINE2-owned atomic NPC inventory/quest-progress composition for Sven: Mead Horn reward and source item-bearing test progress. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Battle Mage Outfits Quest: QENGINE2-owned durable NPC outfit/addon grant and atomic hand-in composition. The production source writer supplies an empty outfit snapshot and rejects Outfit/OutfitAddon effects; a lane-local duplicate would violate the explicit shared-engine allocation.
- Beggar Outfits Quest: QENGINE2-owned durable NPC outfit/addon grant and atomic hand-in composition. The production source writer supplies an empty outfit snapshot and rejects Outfit/OutfitAddon effects; a lane-local duplicate would violate the explicit shared-engine allocation.
- Between the Lines Quest: QENGINE2-owned durable NPC outfit/addon grant: Illuminator Outfits. Production evaluate_source passes an empty outfit snapshot and uses plan_storage, whose effect whitelist rejects Outfit/OutfitAddon. This shared owner is explicitly allocated elsewhere; implementing a second durable grant path in this lane is forbidden.
- Bewitched: Cauldron encounter admission/reset and Bane Bringer kill-to-completion eligibility for the fixed daily contribution. Callback has no encounter lifecycle, and reusing a plain XP-bearing USE would grant1000XP before the required boss kill. This requires the shared encounter/reset owner allocated to QENGINE (NOTE-coordination.md:16), including shared callback/capture integration; it cannot be supplied by an isolated quest module or by test-only progress injection without duplicating that owner.
- Bigfoot's Burden Quest: QENGINE2-owned atomic NPC inventory/quest-progress composition for Gnome NPCs: item-bearing recruitment/repair/task hand-ins and supply package. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Blood Brothers Quest: QENGINE2-owned atomic NPC inventory/quest-progress composition for Julius: garlic-bread/task items and Blood Goblet completion. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Bloody Tusks Quest: QENGINE2-owned atomic NPC inventory/quest-progress composition for Anaztassja Moroia: 17 Norcferatu Talisman hand-in and terminal Gloom Maw/Bat Person grants. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Bored Mini World Change: QENGINE2-owned atomic NPC inventory/quest-progress composition for Wyda: Blood Herb debit and Torn Teddy/Someone's Bored grants. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Brotherhood Outfits Quest: QENGINE2-owned durable NPC outfit/addon grant and atomic hand-in composition. The production source writer supplies an empty outfit snapshot and rejects Outfit/OutfitAddon effects; a lane-local duplicate would violate the explicit shared-engine allocation.
- Cartography 101 Quest: QENGINE2-owned atomic NPC inventory/quest-progress composition for Rotem Valos: survey-map custody and terminal Cartography 101 grant. The only inventory-backed source NPC branch in this snapshot is the fixed Tereban path; other NPC plans run with empty inventory and plan_storage rejects TakeItem/GiveItem/GiveMoney/Experience/Achievement. A second transaction after a storage commit would lose atomicity and a lane-local duplicate violates the shared-engine allocation.
- Cave Explorer Outfits Quest: QENGINE2-owned durable NPC outfit/addon grant and atomic hand-in composition. The production source writer supplies an empty outfit snapshot and rejects Outfit/OutfitAddon effects; a lane-local duplicate would violate the explicit shared-engine allocation.

Independent exact-candidate review, final integration/CI and publication remain coordinator work. No Git/index, credential, Docker, protected routing, external posting or dependency-download mutation was performed.
