# Source-qualified Item charge and duration completion

Classification: **PROVEN** retained wiki observations and source capture behavior;
**DERIVED** exact numeric/unit lowering; shared-page variant applicability remains
**UNKNOWN**. External source content supplies evidence, not task instructions.

The source is `imports/tibiawiki/facts/items-stats.json`, digest
`5fc20ff76f65ab8e0a6bb8f52bc366a0bc0b0b617aaaf3bc8b1c987604a2a2d6`.
The compiler verifies this digest before generating the existing stat packet v2.
Every target must be a current Item with an EXACT Crystal identity binding and no
existing Terrain/WorldObject owner. Binding and map-owner input digests are retained
in the packet. These checks grant no authority to engine stat hypotheses.

The source capture iterates every parsed `itemid` before canonical rekeying and
retains unbound IDs. Qualification groups **all** snapshot records by source page ID;
a page shared with any other ID, including retired or unbound IDs, is held. No
candidate-only filtering can make a shared source page appear unique.

The packet adds **58** positive `charges.count` observations: 57 already match their
known values and one completes UNKNOWN. Wand of Darkness i25760 has explicit 250
charges (page 80875, revision 1148092, content SHA256
`1a64c6b010b9659f7215c79c39800b71166c9ad10312b56e2ff6abf353de8d76`).
The two Magic Gold Converter IDs i28525/i28526 share page 84902 and remain held;
their existing values are preserved. The exhaustive retained-page census qualifies
58 of the 60 charge records.

The packet also completes **138** UNKNOWN `temporal.duration` values in milliseconds.
All present source pages must agree after exact rational unit conversion. Examples:
Star Ring i12669, 10 minutes (54087/1194111); Prismatic Ring i16114, 60 minutes
(59802/1194081); and Lion's Mane Flower i21389, one hour (69466/1115387).
The explicit 7.5-minute observations for i3098 and i6299 become exactly 450000 ms.
**52** duration records remain held because their source pages cover multiple IDs.
The packet preserves every hold's page/revision coordinates, raw values and full ID set.

Known different values and CONFLICT/NOT_APPLICABLE groups or leaves cannot be
replaced. Missing facts remain UNKNOWN. Temporal consumption mode, stop-duration
behavior and decay target remain UNKNOWN; duration does not start a timer or admit
materialization. Item identity classes and the sole existing Backpack i2854 admission
are preserved. No stack maximum is inferred: the retained snapshot has no such field.

The complete stat packet contains **12482 fields on 6530 Items**. The 196 added
packet rows include **139 newly known fields on 138 distinct Items**. Focused tests
cover unit equivalence, fractional precision, numeric bounds, malformed/absent facts,
wiki disagreement, whole-page variant qualification, map ownership, blocked states,
known-value fences and stale source digests. Deterministic packet checks and Ruff pass.

Read-only comparison with the pinned Crystal ff7ede5/00ce02a5 and Canary 04b83b51
XMLs retains engine numbers as OTS hypotheses. Both Crystal snapshots agree on
57 charge values; Canary agrees on 56. All three have 22 matching duration numbers,
113 absent duration attributes and three different duration numbers: i9042 has
wiki 19 days versus engine duration=86400; i22765/i22766 have wiki 12 hours versus
engine duration=86400. The engine converter uses seconds for that raw attribute.
These differences are recorded, not silently reconciled. This packet admits the
explicit wiki declarations only; timer/consumption behavior remains UNKNOWN.
