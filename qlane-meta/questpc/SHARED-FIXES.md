# QUESTPC shared fixes retained at restart

No new shared fixture or migration fix was made by the restart's cohort split.
The following inherited repairs remain in the current uncommitted tree; keep
these bounded hunks when integrating. Historical logs prove only their snapshots.

| File | Reason | Focused proof |
| --- | --- | --- |
| `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_fixture/fresh_session.rs` | Observe Platform security/signing after runtime publication, preserving the actual older runtime observation and existing five-second deadline. | `quest_world_callbacks_stag_fixture_refreshes_security_after_runtime_publication`; run8 central repaired log has `STAG_FRESH_SESSION POSTGRES_CASE_COMPLETED`, 3.351517416s. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/classic_quests.rs` | Bind the fixture actor to the admitted character lease before placing source NPCs. | Current Phase A cohorts exercise their own exact lease/fence path; NPC cases still require fresh qualification. |
| `apps/game-server/migrations/1002_questpc_viola.sql` | PostgreSQL TRUNCATE triggers must use `FOR EACH STATEMENT`; both immutable receipt and state guards retain rejection. | Run8 central repaired tests progressed through migration application after the exact row-trigger DDL failure. Current Phase A cohorts will apply the same migrations once database creation succeeds. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/viola_book.rs` | Cold framed reconnect starts at the original command sequence/continuity rather than inventing a cursor. | `viola_book_postgres_real_use_repeat_replay_refusal_and_fences`; run8 NPC/Viola repaired log has completion marker, 41.489111269s. |

## Unfinished inherited cases

- Father fresh/legacy and scoped NPC dispatch tests last failed at `2504: hi` /
  `2600: hi` (`Rejected` / `NoConversation`). No current qualification claimed.
- Viola hand-in tests last failed at `11: mission` (`Rejected`). Current source
  contains the `0::smallint` stage-query repair but still needs real PG proof.
- Corruptor all-vocations failed its Brass Armor ordinary+guarantee count
  assertion (`left: 1`, `right: 2`); low-stamina scenario returned `Malformed`.
  These are Phase C follow-ups, not chest batch blockers.

- `crates/gameplay/quests/src/quest/loader.rs`, source-binding unit fixture:
  `a_lowered_npc_transition_keeps_its_source_and_dialogue_bindings` asserted raw
  SOURCE bounds `(initial,min,max)=(0,0,1)` against the now extended runtime
  catalogue `(0,-1,2147483647)`. Actual restart unit run was RED, 254/255.
  The test now keeps every original exact binding/effect/evaluation assertion
  for BOTH raw source and runtime, and additionally asserts each exact domain
  and the preserved initial default. No
  source/runtime constraint or production logic changed. Fresh rerun PASS: 255/255 quest units, 13.51s execution / 22.840s wall; .questpc/restart-quest-units-final.log.

## RESTART2 LostWisdom fixture
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/viola.rs`: `relocate` reused admission-only position initialization on an already-positioned actor; both actual PG cases failed `PositionAlreadyInitialized`. It now commits cardinal steps through the existing exact-snapshot position owner, retaining actor/pin/revision and source targets. Proof filter:`viola_mission_yes`; baseline restart2-lost-wisdom-pg-before.*, repaired result pending. This is fixture setup between source selectors, not an access-route certificate.
- Same test adds a contended spell-state real mission call with bounded rejection and zero receipts. Actual RED:`Viola suspended while holding the runtime lock`,1failed/0passed,95.82s execution/214.662s total; restart2-lost-wisdom-contention-red.*. Narrow production fix is `npc_command/viola.rs`:try_lock rejection instead of suspending while holding runtime. Both original strict exchange scenarios retained; repaired PG pending.

- `postgres_tests/viola.rs` and `postgres_tests/npc_program.rs`: after injected unknown COMMIT, call existing production `FreshAdmissionAuthority::resume_npc_bank` rather than redispatching `npc_talk` around the connection recovery preflight. Actual Viola repair-round1 reached deferred rollback then failed `!reconciled.unknown`; the nested case passed with POSTGRES_CASE_COMPLETED119.676870263s. The Father sibling ignored the same retry outcome before command2504. Both now require a present, known, rejected reconciliation. Father additionally proves the final wood-track trigger ran via a nontransactional sequence; no assertion was weakened. Final focused proof pending.

LostWisdom final proof: normal rollback/recovery PASS, marker141.53087234s /wall278.533s, restart2-lost-wisdom-rollback-pg-final.*. Nested capacity case PASS marker119.676870263s from repair-round1; its branch is unchanged by the final recovery-only fixture edit. Both run new contention rejection before any receipt. Production lock regression RED→GREEN; fixture relocation and real recovery resume proved. Father sibling proof remains pending.

Father shared recovery/witness repair is now proved by BOTH actual fresh+legacy PG passes:93.515309753s and89.460936624s, restart2-npc-starts-pg-before.*.
Hero fixture: quest_world_callbacks/scenario.rs adds only a named fixture branch, reusing existing production NPC setup. Existing mixed-case assertions remain reachable through false hero_only; focused true branch additionally asserts one exact receipt after replay. New test proof pending under restart2-hero-greetings-pg-repaired.*.

`postgres_tests/npc_program.rs::exercise_scoped`: actual follow-up reached `AlreadyPlaced` when a second source NPC called the static once-per-Channel placement owner. Place the same five source NPCs in one table once, retaining all existing actor refs and all original assertions. No dynamic/hireling API, production placement rule or other lane helper was changed. Actual baseline/repaired evidence:restart2-hero-greetings-pg-repaired.*; final focused/mixed rerun restart2-hero-scoped-fixture-pg-final.*.

`postgres_tests/quest_world_callbacks/scenario.rs`: only namedHero/scoped fixture capacity is6, exactly one player+five source NPCs. Actual batched5NPC baseline correctly hit Carrier(CapacityExceeded) because legacy classic fixture allowed5total slots. Production capacity, other scenarios and all NPC-placement guards are unchanged. Proof restart2-hero-scoped-six-slots-pg-final.* pending.

FinalHero/sourcefixture proof: focused PASS marker81.139063654s on6slots; originalmixed nowpasses Hero+Palimuth thenhitsactualunownedCalendar gate atJeanPierre2603hi Rejected. restart2-hero-scoped-six-slots-pg-final.*. Static placement and exact fixture slot repairs proved; production capacity and all later assertions remain unchanged.

## RESTART3 Corruptor fixture repair (qualification pending)
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/newhaven_corruptor.rs`: actual filtered baseline reproduced ordinaryBrassArmor1!=2 and malformedQuickLoot. The fixture initializes the hunting clock at0 and kills after60s, so the production firstXP spends2minutes before the ordinary-loot snapshot. Seed843 now asserts exactlive841 after the injected failedMINT, then retains the existing841->840 retry and two-independent-BrassArmor assertion. The first842attempt was correctly rejected by this new strictassert (actual840), log restart3-corruptor-stamina.log; no production stamina/loot rule changed.
- Same file: use canonical Click QuickLoot payload (`fallback_to_main=false`); true is legal only on Assign and caused encoderMalformed before dispatch. Existing exactcorpse/itemUUID/quantity/equipment/replay assertions remain. Focused proof pending: restart3-corruptor-repaired.*. No new command/capability/API/migration or otherlane edits.

RESTART3 source-admission follow-up: `tools/content-migration/quest_finish_newhaven_items.py` now preserves exact noncontainer classification and complete named equipment requirements for the selected native overlay. The existing runtime rejects Unknown level/vocation and container facts (actualSlotMismatch3359/50195), so no runtime guard was relaxed. Cached Crystal corpus declares the missing unrestricted moveevents; pinned currentclient proves noncontainers; earlier accepted currentWiki Monklevel0 retained. `test_quest_finish_newhaven_items.py` adds RED10subcase→GREEN complete-gear regression; full10unitmodule PASS2.751s. Nativegenerator/check PASS43.462/56.877s and Grizzlydependenthash regenerated. Real PG proof pending restart3-corruptor-equipment.*. Source record:restart3-equipment-source-decisions.md.

RESTART3 strict stamina witness: `.load_stamina(840)` after XP intentionally preserves existing live841; new focusedPG assertion reproduced RED841!=840 in66.75stest/125.847swall. The fixture now calls existing `hunting_experience` at one second beyond the first award's120s deadline; exact841before/840after are both asserted. GREEN focusedPG1/1 marker56.066936905s,107.273swall. All original ordinary+guaranteed output, rollback, sameUUIDtake/equip, same-death replay assertions remain. This fixes the test witness only, without changing the source stamina owner or adding API. Final fullCorruptor+Viola/book rerun is queued after serialfmt/Clippy/units in restart3-final-checks.py.

## RESTART3 final current disposition
Corruptor fixture and native-item repairs above are now proved on finalsource/nativeprofile hashes: all5vocations kill→framedQuickLoot→equip, ordinary+guaranteed independentcauses after actual841->840 change, zero-stamina/200%event, replay, corpse transforms/retirement and fresh durabilityprocess recovery. Final4CorruptorPGcasesPASS;8positive markers in restart3-corruptor-qualified.json. BothViola handins+repeatbookPG alsoPASS onthisnativecontent (full9selected/7actualPG/11markers). All original count/Malformed/SlotMismatch/no-op-witness errors listed earlier are superseded by these actualpasses. Father/Hero earlier boundedpasses remainasrecorded; originalmixedJeanPierre calendar testcase remainsblocked. No runtime stamina/equipment guard changed by this continuation; nativefacts are filled from cachedsources.

MintwallinKey3620 is an evidence-only follow-up, no newfixturefix: its existing standaloneUSE/replay/refusal test PASS1/1 afterqualificationpromotion, marker5.542130326s. Strictledgerremaining207->206 and qualified127->128; only thisclaim'squalification/derivedtotals change. Sourceassociation/recipe/worldbinding unchanged.

## RESTART11 authoring fixture
`tools/content-migration/test_quest_callback_profiles.py`: mount the pinned content/assets directory in the existing temporary-root fixture, so the new eight-material native admission verifier can read its actual client source. No admission guard relaxed. Full test_quest_callback_profiles.py 15/15PASS91.789s, restart11-item-units.log. Productiongenerator/nativeprofiles add only i11545..i11552 andi11701; all pre-existingnative records unchanged.

## RESTART11 Father trigger repair
`apps/game-server/migrations/1004_fathers_burden_items.sql`: rename the localPL/pgSQLamountvariable torequired_xp, retaining exact2500/8000andatomicXPobligationguards. RealPGRED42702columnreferenceamountisambiguous inrestart11-father-4.log; GREEN fathers_burden_eight_handins_cape_experience_and_replay marker56.384858737s inrestart11-father-qualified.log. No trigger, refusal or provenance check weakened.1004is the owner-allocatedunpublishedFathermigration. AlltemporaryDBG-father/SETCONSTRAINTSdiagnosticsremoved.

## RESTART11 Alawar source fixture (GREEN pending)
`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks.rs`: do not append a second synthetic passage Item when that passage is exactly the already placed clicked root. Alawar removes its own lever. The old fixture created two2772candidates atthatposition; productionexecute.rs:115–122 correctly rejectedambiguoussource. RED alawars_vault_portals_lever_and_all_claims atscenario.rs:505, Rejected!=Committed, afterbothportalscompleted,95.29s inrestart11-alawar-1.log. Narrowfixturefixretainsoneoriginalroot/UID andeveryremoval; no productionguard/assertionrelaxed. Currentfocusedrerun:restart11-alawar-2.log.

Alawar duplicate-root fix proved: focused second run PASS1/1, completion143.953528621s, `restart11-alawar-2.log`. Both portals, lever/self-removal and every claim completed. Later static source-closure checks and the additive stateless-USE harness extension are being covered by `restart11-alawar-fibula-3.log`; qualification/status promotion waits for that run.

Final shared-fixture proof: `alawars_vault_portals_lever_and_all_claims` and `deeper_fibula_well_and_all_claims` PASS2/2,193.79s in restart11-alawar-fibula-3.log. Alawar retains both real STEP destinations, strict unchanged quest state and its root-removal guard; Fibula adds USE with exact AID15005, exact destination, unchanged tracks/receipts, all6native claims. `stateless_scenario` now explicitly preserves Trigger::StepIn for every existing caller, permitting Use only for the named new Fibula case. No source coordinate or existing assertion was relaxed. Root fixture formatting used rustfmt skip_children=true to keep other lane modules untouched.

## RESTART12: exact position+appearance stateless USE fixture
- File: `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs`. The old helper treated every callback except Fibula as STEP. Between the Lines uses eight exact position+appearance USE roots without an AID/UID. A named family now verifies Trigger::Use plus exact position and nonempty appearance; old callers keep their original Trigger/selector assertions.
- RED: `.questpc/restart12-books-red.log`, Use!=StepIn before DB. GREEN: `between_the_lines_all_eight_book_portals_preserve_quest_state`, eight actual PG cases, completion234.753650472s in `.questpc/restart12-books-green-1.log`. Every destination, nonempty control tracks and receipt count checked. Production selectors were not changed.

## RESTART12: shared source-claim fixture admits the native no-UID nest
- `quest_catalogue_chests.rs`: the source placement tuple now retains `Option<u16>`; every inherited UID stays `Some(original)`. Only the Hydra native case adds its exact32954,32695,z8/5676 selector with `None`, so no source UID0 is emitted.
- `quest_container_chests.rs`: the source namespace assertion includes the same Content-owned fixed Hydra binder called by production USE. Existing legacy source expectations and every refusal/replay/content assertion stay intact.
- RED: `hydra_egg_native_nest_once_replay_and_refusal`,32.10s actualPG, source binder false!=true. Final GREEN still pending; do not claim fixture qualification until its completion marker.

## RESTART12: common NPC debit and generated room proof
- `durability/quest_state/npc_program/fathers_burden.rs`: extracted the already audited bounded inventory read and exact one-item debit for Pemaret reuse; Father's fixed eight IDs/XP/cape predicates stay in its owner. Father regression passed63.573143655s and Marlin passed57.546346838s including post-mint rollback/recovery/replay.
- `content/quests/grizzly-boss-rooms.json`: regenerated after two added native Item profiles; only the native-profile input digest changes, all30room bounds/qualification facts unchanged. Existing `test_grizzly_rooms`1/1PASS4.026s.

Hydra final GREEN: `hydra_egg_native_nest_once_replay_and_refusal` actualPG28.547676038s in restart12-hydra-green-2.log. The sparse BundleFacts callback cache excludes unbound roots; `RewardClaimTile::palette_id` (all2implementations updated) lets this exact Content binder read the current immutable base entry. No UID was fabricated. Clippy content/world/dispatch alltargets/allfeatures clean; pureHydra and source-mapunits pass.

## RESTART12: four plain native source chests
- `quest_catalogue_chests.rs`: only BeregarBPTheUndeathStare6283 and GiantSmithHammer6077/6078/6079 now omit script UIDs in their fixture, matching the four unchanged native cells. Source identity assertions and complete content/replay/capacity checks stay strict.
- `quest_container_chests.rs`: includes the production Content `bind_standalone_chests` result in its source namespace assertion. Production USE uses that same fixed-source binder and existing generation admission, receipt and mint owners.
- Native RED37.28s: source binder false!=true. Native GREEN49.247318891s (`restart12-braindeath-giant-native-green.log`); later reward cohort passed44.385346426s in `restart12-braindeath-giant-native-final.log` while the separate lever case failed. These are reward proofs; Giant whole status still requires native lever proof.

## RESTART12: native Giant lever fixture
- `quest_world_callbacks.rs`: only the source Giant callback drops UID30024; its exact native lever at32778,32232,z8 has no attributes. `giant_smithhammer.rs` now uses that position. The source wall remains32780,32231,z8/389.
- RED37.00s (`restart12-giant-native-lever-red.log`): first USE NothingToUse!=Committed. The fixed-position production alias then opened the wall but return USE was NothingToUse atscenario.rs760 (`restart12-braindeath-giant-native-final.log`). Current repair recognizes the committed2773 member of the exact source2772/2773 pair, while preserving the original UID selector and source effects. GREEN pending in `restart12-giant-native-lever-green.log`.

Native Giant lever final GREEN: `giant_smithhammer_source_lever_opens_and_restores_the_wall` POSTGRES_CASE_COMPLETED35.867644046s in `restart12-giant-native-lever-green.log`; first open and reverse close both execute through the real dispatch.

Four-chest fixture physical correction: the three Giant source2472 objects now use movable `Family::Item` facts with `pickupable=true` and a real Item definition provider; other source fixtures retain their existing family. The pinned client confirms2472container/take=true/unmove=false, while1983coffin is immovable. This stronger fixture reproduced RED48.72s (`restart12-giant-movable-chest-red.log`), exposing an overrestrictive pickupability predicate. The fixed4binder now accepts its exact immutable base source regardless of pickupability; Hydra still requires its immovable nest. Player-added containers never bind to these base keys. Final GREEN56.019167108s (`restart12-giant-movable-chest-green-2.log`) covers all4claims, replay, full backpack/capacity refusal and complete contents. Content1/1, source2/2, pinned mounted rustfmt and content/world/dispatch alltargets/allfeatures clippy-Dwarnings passed.

- RESTART12 Jorge: `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks.rs::input_item` now uses the active runtime's actual map/content/frame placement evidence for setup mints, and reports the item, ordinal and failing phase. The old `content-1`/`[1]` placement conflicted with a legitimately dropped replica on the same cell; `spell_item_transaction.rs::locked_tile_rows_for_scope` correctly refused the next mint as an unreconciled opaque/incompatible placement. No production guard changed. RED `restart12-jorge-dispatch-green-4.log`: fixture mint commit `Unavailable(InvalidStoredState)`,43.28s. GREEN `raid_tokens_jorge_repeatable_exchange_and_replay` POSTGRES_CASE_COMPLETED53.059713253s, `restart12-jorge-dispatch-green-5.log`, all37 source exchanges/repeat/replay/two rollback stages/split-stack and backpack capacity.
- `content/quests/grizzly-boss-rooms.json` regenerated after native item-profile input changed. All30 room facts exactly equal initial28e4313e1; only `inputs_sha256` changed. `restart12-jorge-grizzly-semantic.json`; Python `test_grizzly_rooms.py`1/1,2.919s. No room, geometry or unrelated gameplay change.
- `apps/game-server/src/node/serve/bundle_npcs.rs`, `classic_quests.rs` fixture and `crates/gameplay/npc/src/npc_talk.rs`: admit only Jorge's inherited default NpcHandler greeting through the existing closed native caller gate. His source declares no greeting override; adding him to the caller list alone still reproduced `NoConversation`49.13s. The final exchange PG witness above reaches his actual production greeting and talk handlers.
