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

## questpc6 split continuation
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs`: expose the existing test-only `phase_a_cohort` to sibling auto-mounted lane cases; one visibility change, no production or assertion changes.

## questpc6 qualification cohorts
The only shared-file edit remains `phase_a_cohort` visibility at
`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs:366`.
New lane cases reuse its exact placement, inventory, replay and refusal assertions;
no shared assertion, fixture, registration, production API or migration changed.
The auto-mounted `questpc6_access.rs` contains all new tests.

## questpc6 crate formatting prerequisite
- `crates/gameplay/dispatch/src/gameplay_transport/npc_command/catalogue.rs:322`:
  split `).with_wheel(...)` over two lines exactly as rustfmt requested.
  The package fmt check failed solely on this inherited line. This is a
  formatting-only prerequisite for the required changed-crate fmt gate;
  no expression, caller, authority or NPC behavior changed.

## questpc6 inherited Clippy prerequisites
The first dispatch Clippy run failed in its durability dependency before reaching
the lane tests (clippy-red.log). Two minimal, behavior-neutral repairs:
- `crates/gameplay/durability/src/durability/quest_state/npc_program/fathers_burden.rs:9`:
  remove the unused `serde_json::json` import; keep `Value` and all logic.
- `crates/gameplay/durability/src/durability/character_revision_sequencer/npc_program.rs:5`:
  document and allow only `too_many_arguments` on the existing forwarding method,
  matching adjacent revision-sequencer adapters. Its eight existing arguments
  carry the fenced transaction context; no signature, argument or behavior changed.
No durability mechanic, test assertion, fence, SQL or migration changed.

## Current pressure-plate mechanism
- `world/src/map/facts/quest_callbacks.rs`: shared single-item volatile callback removal, retaining pinned views; called by the production committed movement path.
- `world/src/map/facts.rs`: serve unplaced native sewer435; focused removal/copy-on-write unit test.
- `dispatch/.../paced_movement/special_movement.rs`: source uid25030 creates/removes435 at32482/32170/-14, using existing map mutation owners.
- Existing quest callback scenario/world builders get isolated named Devil Helmet fixture branches only; prior cases and assertions remain unchanged.
- Actual PG RED41.72s at first framed STEP, before production change. GREEN and full checks pending.

Final pressure slice proof: PG3/3,82.83s with3positive markers; unit1+8; BOTH-crate check/Clippy-Dwarnings and workspace/scopedfmt PASS. Evidence:pressure-result-summary.json. Recursive formatting of four untouched sibling modules was restored; no old scenario/assertion changed. Whole Devil Helmet quest remains UNFINISHED, ownerquestpc6.
