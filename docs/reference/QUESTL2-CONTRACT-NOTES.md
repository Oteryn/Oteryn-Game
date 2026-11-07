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
