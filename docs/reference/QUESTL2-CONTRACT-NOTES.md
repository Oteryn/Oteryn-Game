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
