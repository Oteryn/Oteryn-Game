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

Existing NPC trade rejects charged offers before commit; preserving count in the
quest catalogue does not implement charged minting. Requested the generic trade
feature from qengine2. The journey's access purchase uses an ordinary source
offer (Yaman dwarven ring 3097 / Haroun axe ring 3092), keeping the unsupported
charged rows faithful rather than relabelling them as ordinary items.

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
