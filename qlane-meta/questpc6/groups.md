# QUESTPC groups — run7 integration

All writes remain local and uncommitted on detached HEAD1fa04bbb. Root owns
shared routing, registries, generated JSON, golden files, pins, reports and
migration0969. No group grants publication or spending authority. DONE means
the named bounded slice, never whole-quest or deployed-world certification.
All research notes below are relative to `.questpc/`; each reader owned only
its note. Product code outside the assigned coder files remained root-owned.

| Group | Quests/packages still not DONE | Owned file set | Dependencies | Status/evidence |
| --- | --- | --- | --- | --- |
| medusa | Medusa rewards and modern access | research/medusa.md; dispatch postgres_tests/quest_catalogue_chests.rs (coder2) | exact source selector, current access, PG | PARTIAL reward slice; STOPPED modern access. Fresh Wiki revid771196/748935; PG Connection refused, no marker. |
| fluids | Goblin Temple, Dreamer, Present; native fluid owner | research/fluids.md; durability reward_claim_mint/fluid.rs and server support/reward_claim_mint_postgres_cases/filled_fluid.rs (coder3); shared contents.rs/request/migration0969 (root) | explicit source-choice/native projection; PG | PARTIAL owner; STOPPED recipes/full paths. Donor wine2/milk9 known; raw1 conflicts retained. Unit10/0; PG Connection refused. |
| charges | Deeper Fibula, Banshees, Targuna, Ornamented Shield;13 exact admitted claims | research/charges.md; shared reward generators (root); dispatch postgres_tests/quest_{catalogue,container}_chests.rs (coder2) | exact source/default match; PG | PARTIAL supported; STOPPED conflicts. Fresh Wiki names omit quantities; all five claims remain held, including sibling holds. No new PG proof. |
| falcon | Secret Library/Falcon six repeat/random chests | research/falcon.md; any runtime/schema/registry/migration changes reserved to root | per-cycle RNG/receipt owner; current access | STOPPED. Accepted24h retained; donor handler's missing timerStorage conflicts; no once-only mount. |
| newhaven | Newhaven/Corruptor | research/newhaven.md; withdrawn capsule/gear/admission files remain unmounted | staff namespace/version, placed NPC, active callback/full tutorial | STOPPED. Fresh Wiki contradictions; #ondroploot callback disabled by donor loader; no invented kit or ID. |
| callbacks | Blue Valley, Cheesy, Stag:7 literal edges +2 guards | research/callbacks.md; any shared loader/dispatch/content edits reserved to root | exact clicked-object ordinal, genuine read-only owner, access proof | STOPPED. Nine held callbacks unmounted; no ADD(0) substitute. Fresh Wiki/source contradictions recorded. |
| npc | NPC catalogue, Father’s Burden, generic fee, book transport | research/npc.md; withdrawn adapter/fee/wire paths remain unmounted | accepted atomic11-track scope; native effects; custody/wire owners | STOPPED. Cap8 preserved; no split writes, generic fee cause or book wire mount. |
| soul-war | remaining catalogue; Soul War | research/soul-war.md; shared inventory/artifacts (root) | current inventory classification; source conflicts; full paths/PG | STOPPED certification. All373 pinned titles found in788 infobox pages;68h/72h conflict; partial captions only. |
| boot | Dawnport SOURCE/runtime test boundary | apps/game-server/src/quest_catalogue_boot_tests.rs (coder1) | no migration or shared hot files | CLOUD-OK → TAKEN-LOCAL → DONE slice. Red7/2, green9/0, root9/0; Clippy and fmt pass. |
| client-tracker | client quest log/tracker qualification | no new product writes; report/root only | existing decoder/reducer/UI, recorded client tests, Win32 | STOPPED for new qualification; implementation on main preserved. |
| exclusions | Dawnport, OT-only, Prey/Hunting TasksP23 | retained scope predicates/generators (root); no new product expansion | owner exclusions | Excluded; Dawnport SOURCE retained, served paths absent; no OT/P23 admission. |
| validation | all retained slices | .questpc/* and final report (root), lane logs within each coder allocation | PostgreSQL service and zlib headers | PARTIAL / BLOCKER. All6-package Clippy/all-features, fmt, donor replay and consistency checks PASS;92/93 authoring modules pass. PG Connection refused; map decoder lacks zlib.h. |

Root reviewed the complete retained source diff against both checklists and
recorded199 workspace match/constructor/caller sites in r5-all-sites.txt. Two
checker lanes found no remaining concrete implementation contradiction after
the boot repair; research citation/sibling-hold gaps were corrected separately.
No whole-quest completion, new PG qualification or current-candidate CI claim.

## Run 6 allocation (supersedes previous STOPPED statuses)
All nonexcluded PARTIAL/STOPPED groups above reopen ACTIVE. Research lanes:
medusa, fluids, charges, falcon, newhaven, callbacks, npc, soul-war (one note each).
Coders: slot1 callbacks implementation file set assigned by lead; slot2 chest PG
fixtures and qualification; slot3 fluid PG fixtures + fluid implementation.
Root owns all shared routing/types, schema/generators/generated content/migrations.
Existing boot DONE preserved. Independent groups with no migration/shared file
are CLOUD-OK only if identified explicitly; no cloud implementation currently held.

## Run 7 live allocation
All nonexcluded groups ACTIVE; prior evidence-only stops superseded. Root owns
registries, routing mounts, shared types, migrations and generated JSON.
No independent cloud group currently remains unallocated.
| Lane | Owned handwritten files | State |
|---|---|---|
| callbacks slot1 | quests/src/quest/world_callbacks.rs; dispatch/src/gameplay_transport/quest_callbacks.rs; durability/src/durability/quest_world_callback.rs; dispatch/postgres_tests/quest_world_callbacks.rs; new callback-only helper files | TAKEN-LOCAL |
| fluids slot2 | tools/content-schema/reward-claim-authoring/container_native.py and test_container_native.py (fluid-only); durability/reward_claim_mint/fluid.rs; server/tests/support/reward_claim_mint_postgres_cases/filled_fluid.rs | TAKEN-LOCAL |
| charges slot3 | tools/content-migration/quest_reward_item_semantics.py and test_quest_reward_item_semantics.py (charge-only); dispatch/postgres_tests/quest_catalogue_chests.rs | TAKEN-LOCAL |
| NPC slot4 | quests/src/quest/npc_rules.rs and npc_dialogue_catalogue.rs if present; new NPC-only runtime/PG helper files; no shared mount files | TAKEN-LOCAL |
| newhaven slot5 | tools/content-migration/quest_finish_newhaven_items.py and test_quest_finish_newhaven_items.py; quests/src/quest/tutorial.rs; new newhaven-only helper/tests | TAKEN-LOCAL |
| falcon slot6 | new falcon-only files in durability/dispatch/quests; no shared existing files | TAKEN-LOCAL |
Readers each own only research/<lane>.md: medusa,fluids,charges,falcon,newhaven,callbacks,npc,soul-war.
Root additionally owns client-tracker qualification and combined integration/tests.

### Run7 round1 status
medusa: rewardPG PASS; modernaccess source refreshed (not fullworld qualification).
fluids: source native+durable slice DONE, realUSE6 pending slot3.
charges: full13PG PASS; partial5 implementation+schema integrated, PGpending.
callbacks: literal7PG PASS; read_onlyStag integrated, PGpending.
newhaven: PARTIAL atomic+modifier repairs active; no prematurequalification.
npc: PARTIAL newStag-only safe scope; legacy12/father11 blocked by demonstrated
  persisted-pin incompatibility requiring auditedDUR04 migration; no silentrepin.
falcon: PARTIAL owner+schema implemented, missing9081profile beinggenerated.
soul-war: PARTIAL68h correction red-green; actualinstance allocation unavailable
  at existing production command178(ContentUnavailable), focusedPGpending.
client-tracker: pendingexisting focusedqualification; root.

## Run 8+ allocation
OWNER: whole quest catalogue, see .questpc/OWNER-SCOPE-ALL.md.

## Run7 round2 checkpoint (current; supersedes historical rows)
- medusa: DONE exact reward slice; real source USE/PG passed. Full source-map
  access is not certified by a reward fixture.
- charges: DONE exact13 full defaults +5 partial recipes. Final partial batch
 2/0 with fluid batch,11 source-placement markers total,67.70s runtime.
- fluids: DONE three exact recipes/six placements plus atomic fluid owner;
  real USE/PG passed, root/child replay/fault tests passed.
- falcon: DONE six exact random24h source reward cycles;2/0 PG58.58s,
 6markers;3rules tests. Full Global access remains outside fixture proof.
- callbacks: DONE nine source edges;7literalPG207.66s +8guards/staticcrypt
 242.24s,16markers;3unit regressions green. No quest writes for predicates.
- npc: DONE newStag storage-only slice (6NPC),25units +PG51.68s. PARTIAL
 broader catalogue: demonstrated12legacy hash regressions require audited
 DUR04migration; native item/money/XP/outfit effects remain whole-refused.
- newhaven: ACTIVE final stable-plan retry repair; atomicity/modifier issues
 repaired; tests and delta review still running. Viola/full tutorial remains
 unmounted at the existing native hand-in/book/NPC placement seams.
- soul-war: DONE68h retry correction with2PG+entryrefusal1PG. PARTIAL full
 encounters: existing command178 refuses ContentUnavailable without W02owner.
- client-tracker: DONE recorded integration test1/0 using existing play_features
 target,0.20s runtime; no client product edit required.
- validation: source/downstream regeneration PASS987.801s; authoring consistency
 PASS202.971s. Combined clippy/current93-module tests/donorcheck active. Map
 decoder remains blocked by concrete g++ missingzlib.h, not missing evidence.
Counts:336claims;334supported(39dispatchPGqualified,295unqualified);2Dawnport
ownerexcluded. These counts certify reward shapes/selectors, not whole quests.

### Validation follow-up
InstalledNodeSDK resolveszlibheaderwithoutinstalls;mapdecoder23/0. Current
10packageClippy/alltargets/allfeaturesPASS87.511s afterretryrepairs. Root
combined11filteredPGcasesand93authoringmodulesactive. Newhavenpendingowner
capacitychoice48source/30acceptedGame, plusverified3legacyordinarylootID
namespace corrections; no broadadmissionchangeauthorizedorapplied.

### Run7 root final checks checkpoint
93/93authoring modules PASS;nativeproducer--checkPASS;rootfluidPG1/0;root
combineddispatch10/1(sharedfresh-sessionfixture failure). SameStagguardcase
alone1/0/all9markers; concurrencyqualification CI-only. RootNewhavenpure2/0,
actualsourcePGstillblockedcorpseadmission; no approvalinferred. FinalClippy
repetitionawaitedafterequivalenttestRangecontainsrepair.

Finalrun7checkpoint: validation DONE for current bounded slices (all93authoring
modules+nativeproducer+10crateClippy/fmt); rootStagisolatedall9markersgreen,
combinedfreshfixtureconcurrencyCI-only. Newhaven PARTIAL pending explicitowner
capacitychoice; fullViola/legacyDUR04/Father/W02 remainPARTIAL atdocumented
nativecontractseams. Noapprovalinferred,nopublication. See r7-validation.md/json.
