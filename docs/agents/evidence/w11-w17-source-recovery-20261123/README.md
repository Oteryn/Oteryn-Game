# W11 / W15b / W16 / W17 source recovery

Research date: 2026-11-23. Base: `staging@4f13511ca9ec3fb152f11299bb5cf7c71de16c41`.
This checkpoint continues draft PR #3459 after `f3ca41df2264414a2d64673b2735cc2d38ee0e0f`.
The live PR records each published head and its exact validation.

## Published findings

| Area | Verified result | Remaining work |
| --- | --- | --- |
| W11 Soul War | Crystal and the inspected original-author revision retain seven Phantom coordinates followed by bare `Pos`. Alternate generic spawn lists do not establish Phantom slot eight. | The eighth initial coordinate and intended Global cardinality remain UNKNOWN. Preserve `unresolved_phantom: Some(8)`. |
| W15b Goblin Temple | Existing authoring selects UID 6119, item 2874, quantity 1, milk subtype 9. Crystal's tuple supplies subtype 1, which means water. | Preserve the existing reconciliation. No duplicate override is introduced. |
| W16 Mimar Haffar | The evidence packet recovers the two-cannon loop and separates the standalone fight from the later Phosphorus rematch. | Complete entry, actor schedules and the precise vulnerability timer remain unqualified. A bounded crystal-lifetime implementation is under separate validation and is not included in this checkpoint. |
| W17 Tibiadrome | Five appearance mappings and the 252-cell Thais battle mask are recovered. The reproducible source importer is included in this checkpoint. Existing owners already contain substantial wave policy. | Executable creature profiles, attack clocks/masks, native admission and exact historical wave-skip thresholds remain incomplete. Public admission stays disabled. |

Detailed provenance and limitations:

- [W11 Phantom and W15b milk](w11-w15b.md).
- [W16 admission and phases](w16.md).
- [W17 sources, wave policy and native integration](w17.md).

## W17 source importer

[tibiadrome_source_profile.py](../../../../tools/content-migration/tibiadrome_source_profile.py)
provides `capture` and read-only `check` commands for
[tibiadrome-source-profile.json](../../../../tools/content-schema/native-gameplay/tibiadrome-source-profile.json).
It validates the pinned appearance manifest/catalog, selected outfit records,
six sprite sheets and eight minimap PNGs. Changed bytes, duplicate/unknown
fields and invented gameplay defaults are refused.

| Creature | Qualified look type |
| --- | --- |
| Murmillion | 1422 |
| Scissorion | 1423 |
| Hoodinion | 1424 |
| Mearidion | 1425 |
| Domestikion | 1426 |

The source-derived Thais mask is `x=32283..32298, y=32179..32194, z=12`,
excluding four corners: 252 cells. The lever is at `32289,32198,12`; the five
waiting pads are `x=32289, y=32199..32203, z=12`. These coordinates are derived
from retained minimap data and original footage, not a complete terrain/item
stack import. One observed historical solo arrival is documented separately
and is not promoted to a general party-admission rule.

The retained packet has SHA-256
`42760285635e2d4f51a3960e5ddf19762dba66fae244e50c5ee97164959bbcbb`
and keeps `runtime_admission: false`. Original footage shows skips of three,
two and one waves; it does not establish exact speed thresholds, scoring or
continuity to the target 15.30 build. Existing accepted Oteryn defaults remain
explicitly separate from measured source facts.

## Validation of this checkpoint

- Eight source-importer Python tests passed, including CLI idempotency,
  read-only checking, input drift refusal, pool/geometry and historical skips.
- The importer successfully checked the actual retained appearance/catalog,
  six sprite sheets and eight minimap PNGs.
- Twenty-three existing Goblin reward-authoring tests passed; no new
  PostgreSQL chest journey is claimed.
- Governance checks passed (22 required documents, 9 lanes; 135 tests).
- The lead repeats bounded checks on the returned published head. Exact-head
  CI/review state is recorded in the PR; this packet does not imply CI success.

No mechanics ledger is marked DONE. This checkpoint does not enable a public
Mimar or Tibiadrome entry, guess the Phantom coordinate, or qualify a complete
boss encounter. Runtime work is published in a later verified commit.
