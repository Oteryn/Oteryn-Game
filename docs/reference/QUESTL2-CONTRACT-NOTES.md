# L2 integration decisions

2026-11-07: Consume main's QENGINE2/QBASE and QWORLD migrations directly.
QuestTransition and world callback fixture literals explicitly set the new
optional world_change field to None; no new world-change behavior is authored.

Central generated completion reports distinguish native chest progress bindings
from donor mission coverage. Validated native claims remain in quest-state.json;
they are excluded only from the missing-trigger work queue. A native-only claim
owner does not become a source mission overlay. Exact catalogue checks remain
310 completion candidates, 6 source-lowered owners, 121 unbound transitions,
and 382 unsupported effects. A mismatched or undeclared claim fails closed.
Earlier lane source receipts and mechanics decisions are preserved in
QENGINE2-CONTRACT-NOTES.md following main's rename of the former root file.

QNPCFX integration: main migration 1315 now supplies the Father item/cape
proof formerly preserved at the end of lane migration 1219. Drop that duplicate
section; canonical text differs only by renaming expected_xp to required_xp.
Retain 1219's distinct fluid, charge, Corruptor and Viola owners.
Canonical NPC item exchange, calendar and conversation mechanisms are reused.
L2 exact legacy-default snapshots and sixteen-effect QuestState transaction cap remain;
canonical NPC planning keeps its eight-effect cap with the exact eleven-track
Father acceptance exception. Both cap and atomic acceptance assertions are retained;
canonical admission predecessor support is kept alongside those snapshots.

The canonical write-bounds adapter keeps the Father quest-log flag at its
strict 0..1 domain; the earlier blanket NPC-source widening is superseded.
Canonical NPC registration adds Marlin Trophy, Beregar and Tibia Tales to the
six prior source owners: exact tests now require eleven programmes and nine
source legacy owners, with all predecessor/default/replay assertions retained.
Draconia's genuine earlier L2 native-callback hashes go through main's existing
predecessor map; a newly activated donor NPC programme never acquires invented
source history. Unknown hashes remain refused. The full quest crate passes
303 tests, with no ignored or filtered cases (20.67 seconds).

Cake investigation: Biff's donor script uses the shared200-substance stage,
not a personal20-item exchange (Crystal9f5a72c,
data-global/npc/biff_the_baker.lua:50-99, SHA256
a590a49ebec09956779a7ca9776c449dc4e6d849e9821c5ddcd6df9da0b36a9d).
The accepted chosen completion recipe explicitly replaces world totals with a
personal20-substance campaign, while retaining extraction and cake-bite steps.
Reuse SEASON16cba2868's existing Cake custody/extraction/bite implementation;
request the quest-progress bridge from qengine2. A20-item hand-in alone cannot
qualify the complete recipe. No new world-event engine is authored in L2.

Post-merge PG regression: four real journeys pass on4528570a6 (Father's eight
hand-ins86.680607778s, Father atomic acceptance83.099677349s, Tomes123.258276695s,
Sam125.095789195s). The generic atomic-eleven case fails43.885595413s with
game_character_quest_receipt_effects_shape: main1314 replaces the wider1210
constraint with8, while Rust's catalogue still accepts16. The exact Father
exception remains green. Request canonical reconciliation from qengine2; do not
weaken the generic regression or change main's historical migration.

Djinn source review: Crystal9f5a72c quest_system2.lua:2-9,30-38,361-475
(SHA25604990d261d85479bf9a75de040f6c158bc6eb557018a7ad359374400c2a49635)
binds literal map-USE of UID2285/3024 to one carried lamp3231, a fresh output
3243 and Mission03 1->2. Reuse existing world callback carried inputs and
item_reward rather than inventing USE_WITH or a second lamp engine. Bo'Ques
requires cookbook3234 and awards three sapphires3029; Rata'Mari consumes
cheese3607 and issues report3232. Both initial PG binding probes are RED
with actual18.6 server markers and five-second durations before authoring.

### Djinn faction binding slice (2026-11-07, in progress)

Reuse the existing NPC reward, paid exchange, conditional shop and literal map-USE
owners. Source UID 2285 (Efreet) and UID 3024 (Marid) exchange a carried 3231 lamp
for a 3243 sleeping lamp and advance mission 3 atomically:
`data-global/scripts/actions/other/others/quest_system2.lua:2-38,361-484`,
sha256 `04990d261d85479bf9a75de040f6c158bc6eb557018a7ad359374400c2a49635`.
The fountain AID 12105 issues tear 3233 only at Efreet mission 2 stage 1:
`data-global/scripts/quests/the_djinn_war_quest/action_water_fountain.lua:3-13`,
sha256 `c259ac14a543bd5cc1fd3c84c2540b4b09c2fcd4055fe588a9abc85cacbbbc4c`.
Oteryn delivery adaptation: the tear is minted directly into carried inventory,
rather than placed at the player's feet, using the existing atomic reward owner.
The Orc King's global ReceivedLamp once flag is projected onto the active faction's
DoorToLamp flag: faction enlistment atomically forbids the other faction, so the
active faction is the only possible issuer. Generated callback sources retain
full file, line and SHA witnesses for every dialogue transition.

Shop menus preserve source count/subType fields; a new 21-line projection helper
keeps charged Yaman offers at their source count rather than silently rewriting
them to one. Its unit test was red before the adapter and green after. Neither
Djinn quest is qualified until its complete native PostgreSQL journey is green;
this binding slice does not substitute data-presence checks for the journey.

Collector main 78294c081 supplies NPC-TRADE-3 full-charge commits (1347).
L2 now reuses that owner: each source Lua `count` must equal the exact KNOWN
native charge count, and is projected as one item with that charge subtype.
The positive Efreet access purchase is Yaman's 200-charge necklace; Marid uses
Haroun's axe ring. The source mismatch regression refuses 199 in place of 200.
Main's separately branded NPC input authority also supersedes the earlier L2
carried-debit helper; the carried-only location guard remains in its caller.

### Koshei source decision and shared seams (2026-11-07)

The pinned donor is explicit: A Sweaty Cyclops consumes parts 7528–7531 and
5000 gold, waits 24 hours, then issues amulet 7532 once. Witness:
`data-global/npc/a_sweaty_cyclops.lua:251-337`, sha256
`bc694bf7d62f6a0daae1eda9f6542c6bd79fb749805fe58c23275189ce1e5d0a`.
Oteryn will apply the fee and four parts atomically, retaining them when any
input is absent; the donor's fee-before-parts bug is not copied.

The final use selects carried amulet 7532 on corpse appearance 7535, consumes
one and grants RewardDoor=1:
`data-global/scripts/quests/koshei_the_deathless_quest/action_boss.lua:1-14`,
sha256 `1452b73d3922672bd6a2f5173cbcad2d52e2cb747a5d2978d1292b6e93033fd0`.
The coffin switch UID3070 checks all five statue positions and transforms
162→167 for 120 seconds; its source is
`data-global/scripts/quests/koshei_the_deathless_quest/action_switch.lua:1-46`,
sha256 `73c7e2f3c6b1b5ef582b9e09e068d829fd127ec5553358d443b7c25873b26855`.

Existing bounded reward state operations already provide SetNow and
ElapsedAtLeast for assembly. Production duration remains 86400 seconds.
Requested qengine2's selected-corpse item-use route and a deterministic durable
quest-clock test seam; tests must not shorten that timer or seed completion.
QENGINE2 was fetched again and remains e5850715e. No competing implementation
was found in its current head or the available corpse-callback branch history.
This is source investigation, not a claim of Koshei qualification.

Djinn lamp identity decision: use the existing one-item replacement owner for
3231→3243, preserving the carried instance while changing its definition in the
same transaction as mission progress. The donor instead mints a new instance
and removes the old one. Contents, quantities, access and replay behavior are
preserved; no unsupported combination of world item-cost and fresh reward is
authored. Both donor Ally achievements are part of the final NPC transition.

The post-main boss fixture also exposed stale generated loot facts: the active
profiles already admit corpses 5973/5995/6006, while this branch's loot table
still marked them unmaterializable. Regenerate with
`build_full_spell_artifact.py --refresh-loot-tables` before registering native
providers. Keep all five pre-existing shared-body admission expectations intact.


Collector round 1 handoff (main 78294c081, merge 568541212): preserve main's
NPC input authority, world-observation guards and NPC-TRADE-3 full-charge owner
alongside L2's read-only gates, exact clicked objects and quest-shop bindings.
The lane migrations remain 1210-1219, with no duplicate numbers or QBASE copies.
Canonical regeneration retains 336 pilot claims plus Sam's single armor claim;
registration emits 46 outputs including both L2 source-loot artifacts.

The merged PostgreSQL run executed seven cases in 385.55 seconds: both real
Father journeys, Sam and Troll passed. The synthetic generic eleven-track
transition remains red against main 1349's eight-track cap; its exact error is
`Unavailable(Database(Database(None)))`. The shared engine owner must compose
that capacity after migration 1349, preserving the authentic Father exception.
Neither the published world-change migration nor this regression was weakened.
Both Djinn journeys independently exposed `SharedCell`: the fixture placed all
twelve actors on one cell. They now occupy twelve distinct reachable cells,
leaving UID/AID cells clear. Their fresh PostgreSQL verification is pending;
neither Djinn quest is marked QUALIFIED. The additional repository integration
build was stopped to prioritize mandatory pre-push checks and remains pending.
PREPUSH-2 permits a compiling checkpoint when the changed-crate unit run exceeds
the cadence budget; the status JSON records the outstanding checks explicitly.
After the handoff push this branch is frozen; follow-up work belongs on questl2b.


Continuation questl2b starts at a16e1e9b0; the collector branch remains frozen.
The paid-travel correction is a 48-line local routing helper, not a new reward
or trigger capability. It shares the native quote predicate and allows only an
existing yes confirmation with the same NPC, conversation, item fence and
unexpired 60-second quote. Existing missing-item and stale-confirmation refusals
remain owned by their original handlers. White Raven reproduced the missing
travel receipt on real PostgreSQL 18.6 in 93.92 seconds before this correction.

The Djinn fixture needs twelve distinct actor cells and exactly thirteen slots
including the player. Its previous five-slot default caused both real PG cases
to fail before mission execution (105.77s/106.12s); production limits are intact.
Neither faction is qualified until its complete post-correction journey passes.

Main's itemhold_charge_precedence.py:12-15 selects the full 150-charge Item3083
for UID6063, preserving the donor raw argument1 as a witness (file sha256
7706d1fb2a71e0918c52d0ee835bb4fa1e0ee9f2a6f318de18cb7067c893e730). Preserve that canonical native default and reject the stale L2
partial-charge override, including forged source-choice tags. The dedicated
regression now checks that choice and all schema/hash and charge tampering.
No other quest's authored reward data was changed.

Equipment audit decision: reuse qengine2 dd35529b6's source-ID audit generator.
The immutable collector baseline has 3706 composed profiles: 3668 base
profiles plus 13 starting-loadout and 25 additional item-update identities.
L2 adds 185 identities, for 3891 composed profiles; overrides do not add identities.
The separate137-Item reward source packet overlaps existing cohorts; Item3079
has captured speed20. Preserve every raw-ability and unknown-speed assertion,
and additionally pin the exact sorted identity-set SHA through the new audit
generator. Both audit generators are registered in the repository regeneration
workflow. This replaces an incorrect disjoint-count assumption with an exact
source identity check; no cardinality is edited by hand.

Custody evidence must hash the final Creature provider after Prey embeds its
rules, rather than the intermediate provider. Finalize that source binding in
the native generator, retaining the exact same gameplay profiles and loot.

Sea Serpent source lookup: Haba's canonical donor filename is
`data-global/npc/captain_haba.lua`, not `haba.lua`. At crystal revision
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, SHA256
`8a2aba893a3611c7ab1741b24f8922b1fcf5b6c837b4702a673dd4e4e6317909`,
lines 99–133 require five each of fish 3578, northern pike 3580, green perch
7159 and rainbow trout 7158, in order. Lines 95–112 gate the explicit hunt
confirmation and destination 31942/31047/6 on completion. Reuse these exact
ordered hand-ins and the existing travel owner; no missing NPC assumption.

Conditional merchant admission: the generated source greeting and the validated
conditional-shop identity must both exist. The 24-line local NPC helper grants
conversation access only; native trade retains every quest predicate. Fresh PG
runs pass White Raven (92.508357247 s scenario, 98.002 s test) and paid travel
(91.501 s). The equipment audit passes in 51.288 s, including exact unknown-speed
membership after captured profiles qualify 3212/31266/31268 with zero speed.
Both Djinn journeys progress past merchant lookup but remain red at actual
fountain/lamp use (`NothingToUse`). The current Quest library loads 386 callbacks,
including all three Djinn callbacks; inspect fixture target resolution next.
QENGINE2 7019e60a8 retains the callback-only greeting guard; no existing merchant
helper was available to reuse. No large shared engine feature was added.
