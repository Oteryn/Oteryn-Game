# Chakoya Iceberg: fixed personal first survey

The authored recipe explicitly says “Make the iceberg route a fixed personal
expedition rather than dependent on a mini-world spawn.” This resolver selects
only its first objective, “Reach the iceberg north of Port Hope”. It does not
activate or qualify the global miniature World event, infer its spawn chance,
credit Chakoya kills, inspect the hidden chamber or grant a reward.

TIBIAWIKI_STRUCTURED references retained by the recipe:

- https://tibia.fandom.com/index.php?curid=76212&oldid=847008
- https://tibia.fandom.com/index.php?curid=76213&oldid=887139

The exact Area source is independently retained at:
https://tibia.fandom.com/index.php?curid=75507&oldid=1117033
Its marker is (32623,32584,7), in coordinate frame global-target-2026-09-27.
No raw wiki body was reacquired for this slice. Recorded revisions/hashes and
canonical recipe intent are checked locally; the fixed personal expedition is
an Oteryn choice, not a claim about the original World event's availability.

The existing `region-z07-x127-y127.b3` retains marker stack [799,7152] at that
point, with a clear snow approach at (32622,32585,-7), ground6585. The resolver
checks the complete landmark stack, exact Area identity/frame/position and pinned
source. House-owned or changed stacks are rejected. All native objects remain.
The adjacent clear start is (32622,32586,-7), ground6582, for a STEP North test.

OTS_HYPOTHESIS_ONLY item-name reference, previously fetched at this exact revision:
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml
The native snow/ice footprint itself is inspected in the retained B3 data.

Premium remains required by the canonical recipe. The resolver refuses other
quests/stages and changed personal-route intent, repeat mode, objective or spoiler
provenance. The default global World owner remains HELD and is not aliased.
The first-survey callback is active. Real native dispatch PostgreSQL case
`m01b_chakoya_native_personal_iceberg_entry_requires_current_premium` passed
in78.48657683s (0ignored): original landmark stack preserved; missing Current
Premium refuses credit; gaining Premium cannot retroactively credit a replay;
fresh entry credits once and revisits do not farm it. Kill/chamber track rows,
whole-quest completion and global iceberg event state remain zero.
This qualifies only the first survey, not the remaining expedition or its reward.

## Selected later igloo inspection

The authored s3 objective is “Inspect the hidden igloo chamber”. Its exact Native
binding selects the existing northern ice-walled alcove at (32621,32555,-6),
ground6585, with clear south start (32621,32556,-6), ground799. This is a documented
Oteryn room selection for the authored fixed expedition, not a claim that a wiki
body supplied these precise coordinates or an original secret-trigger algorithm.

The retained `region-z06-x127-y127.b3` has a frozen chair at (32620,32554), stack
[799,7107,7163], enclosed by checked north/west/east wall stacks at
(32620,32553)[6869,6729,6952],(32618,32555)[6869,6728], and
(32622,32555)[6591,7110,6728]. The scoped resolver verifies this complete footprint
and the fixed personal first-route contract, then retains both prior guards:
s1=1 and s2=3, plus Current Premium. Other stages/objectives cannot borrow it.
No item is spawned or changed; the global iceberg owner remains HELD.

The dedicated native PG case passed in66.819570055s. It earned s1 through real
STEP, arranged only the separate KILL owner's three-kill condition, refused stale
replay and missing Premium, credited a fresh room entry once and left whole
completion zero. The first-visit regression also passed in64.969116186s after
sharing its unchanged NativeCrop. Combined:2passed0failed0ignored,131.79s.

The first chamber PG run failed in58.114248801s because half-open crop maxima
excluded the east wall and south start:4 of6 required tiles. Expanding the crop
included both while retaining all six source assertions; both cases then passed.
