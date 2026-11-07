# QUESTPC shared fixes retained at restart

No new shared fixture or migration fix was made by the restart's cohort split.
The following inherited repairs remain in the current uncommitted tree; keep
these bounded hunks when integrating. Historical logs prove only their snapshots.

| File | Reason | Focused proof |
| --- | --- | --- |
| `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_fixture/fresh_session.rs` | Observe Platform security/signing after runtime publication, preserving the actual older runtime observation and existing five-second deadline. | `quest_world_callbacks_stag_fixture_refreshes_security_after_runtime_publication`; run8 central repaired log has `STAG_FRESH_SESSION POSTGRES_CASE_COMPLETED`, 3.351517416s. |
| `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/classic_quests.rs` | Bind the fixture actor to the admitted character lease before placing source NPCs. | Current Phase A cohorts exercise their own exact lease/fence path; NPC cases still require fresh qualification. |
| `apps/game-server/migrations/1283_quest_viola_rewards.sql` | PostgreSQL TRUNCATE triggers must use `FOR EACH STATEMENT`; both immutable receipt and state guards retain rejection. | Run8 central repaired tests progressed through migration application after the exact row-trigger DDL failure. Current Phase A cohorts will apply the same migrations once database creation succeeds. |
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

## questpc7 source reward cohort reuse

- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs`:
  change only the existing `cases` and `execute` functions' visibility to
  `pub(super)`, allowing the auto-mounted questpc7 tests to select this lane's
  exact claims without copying or relaxing the production-dispatch harness.
  No production API, assertion, fixture or source reward changed.

- `crates/gameplay/dispatch/src/gameplay_transport/npc_command/catalogue.rs:322`:
  required workspace `cargo fmt --all -- --check` found a pre-existing chained
  `.with_wheel(...)` on the closing constructor line. Applied rustfmt's exact
  one-line split only; no NPC behavior or other formatting changed. This is
  outside `npc_command.rs`, and adds no routing hook or shared engine copy.
  Baseline failure: `lane-fmt.log` / `lane-fmt.result.json`.

## questpc7 inherited Clippy blockers

- `crates/gameplay/durability/src/durability/quest_state/npc_program/fathers_burden.rs:9`: remove the unused `json` macro import only. Actual `lane-clippy-final` failed under `-D warnings`; `Value` remains used at the two deserialization sites. No behavior changes.
- `crates/gameplay/durability/src/durability/character_revision_sequencer/npc_program.rs:5`: add a function-scoped `too_many_arguments` allowance with its forwarding reason, matching the existing revision-sequencer convention. The inherited writer gained the required item-content argument and Clippy now counts eight. Do not bundle or discard authority/fence/content inputs merely to alter that count. No signature, caller, operation or test changes. Both findings are recorded in `lane-clippy-final.log`; repaired all-target/all-feature dispatch+durability proof PASS (161.564s), with final workspace fmt PASS (10.216s). Exact commands/results: VALIDATION.json.

## Current continuation: shared passage-state predicate

`world_callbacks::Passage.require_state` is optional/default false and omitted
when false, preserving previous callback serialization and receipt inputs.
Only the source Giant Smithhammer callback opts in. The executor checks every
configured predicate under the existing Channel owner lock before a quest or
map write, so a discordant remote rock cannot toggle its lever. Rust and Python
reject the flag for replacement passages, one-way/non-levers and STEP callbacks.
No schema CHECK, cause, migration, public function, wire code or authority owner
was introduced. Source: Crystal201189f giant_smithhammer/lever.lua:4-18.

The existing real-dispatch callback scenario retains all original assertions;
its Giant Smithhammer branch additionally exercises both discordant states,
asserts unchanged lever revisions and remote visibility, and then proves the
next legitimate restoration/collision change. The new auto-mounted test uses
the existing 64+MiB scenario-thread runner. Initial actual RED29.78s;
GREEN29.695032877s. Final evidence is asymmetric-validation.json.

The callback proof and quest-tree provenance were refreshed by
regenerate_content.refresh_quest_callback_proof and quest_tree_authoring.py;
only generated hashes change in the four quest-tree files. No reward contents,
recipe, stage order, requirement, NPC or inherited assertion was changed.

### Continuation final validation

All9Rust/content checks pass on regenerated runtime inputs;14callback units,
2GiantSmithhammer-only PG tests (4positive case markers;44.91s execution),
123server content tests and5world-project tests. Final marker35.916384503s.
Exact commands/results: CONTINUATION-VALIDATION.json and
asymmetric-rechecked-validation.json. Full authoring consistency and strict
Python3.12 donor replay also pass. The donor NPC decompressed diagnostic proved
exactly4quest-input hash updates; the normal generator refreshed their dependent
captures. Recursive audit of31JSON/gzip files found no non-hash changes except
the new guard flag. No checker, source fact, test assertion or admission was
weakened. All37product files are listed in CHANGED-FILES.txt. This qualifies
one shared mechanism;0whole quests and0quest batches were completed.
