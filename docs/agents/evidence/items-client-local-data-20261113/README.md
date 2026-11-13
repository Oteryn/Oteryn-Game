# Client-local Items data and bounded comparison evidence

This snapshot makes prepared local-data evidence reviewable:6,470 Cyclopedia-bearing
client appearance records representing6,228 client types;15,350 raw NPC tuple
occurrences from2,679 appearances;115 candidate comparison rows; and9 qualified
evidence correction records in8 groups. It changes no runtime or source definitions.

These populations differ: the Cyclopedia-bearing subset contains14,235 NPC tuples
from2,571 appearances. Another1,115 of the global15,350 tuples occur outside that
subset. Membership uses the shared client appearance namespace only, never serverID.

The repository authoring base is3e168ce6afe374485da20ee0ac13d3749558e2f8. Comparison
data retains the older5efd67da164ac9dc33c8c066434b228cebb86c2b source snapshot:
34,111 canonical definitions and6,090 selected profiles remain separate. This is
not a claim that the old data equals current main or the running client.

## Reading the files

- `client-cyclopedia-coverage.json`: explicit column tables, including157 alias
  groups. Rows preserve client appearance/type namespaces and raw-record hashes.
- `client-npc-tuples.json`: lossless column encoding. Dictionaries intern repeated
  strings/identical field objects; they never deduplicate rows. Numeric columns
  remain literal. Run `validate.py` to decode all15,350 ordered occurrences.
- `field-comparison-candidates.json`:115 selected rows/116 field instances that
  differ as candidates or retain KNOWN_ZERO. Complete20-field aggregate counts
  describe2,623 observations across10 input categories; this is not all Items.
- `qualified-correction-deltas.json`:9 records/8 groups. Historical published
  evidence stays unchanged; these are explicit deltas awaiting the owning lane.
  Decoration116 remains310 gp: the30 gp candidate was rejected. Every empty
  quote array, including Food117 Sell To, means OBSERVED_INITIAL_PANEL_ONLY;
  it never establishes global absence of NPC offers.

Names, images, appearance IDs, Cyclopedia types, source candidate keys and server
IDs are distinct namespaces. No name/icon/ordinal equality promotes identity.
All comparison differences remain candidate evidence, not verified gameplay bugs.
Canonical/selected equality is also only candidate equality.

Six NPC field-presence bits remain independent of their raw numeric values.
Sale-price0 and buy-price0 are explicit known zeros (6,688 and6,859 occurrences).
Missing currency components remain FIELD_ABSENT; present client appearance
references retain UNKNOWN display/server currency binding. Never infer gp or a
trade direction beyond the recorded raw field names. Quest label literals remain
private; only their custody hashes/counts are included.

## Limits and custody

Overall static audit: **PARTIAL**. Potions1..36 remain partial with UNKNOWN total;
Others547 retains an unresolved description glyph. Armors/Amulets are excluded
from the selected bulk comparison. Augments, weapon statistics, Imbuement Effect,
instance expiry/charges and NPC runtime offers are not qualified by this comparator.
Speed/ratio/sign/zero-display formatter differences need their own qualification.

Original assets, screenshots, raw private exports, Description narratives, quest
labels, absolute private paths and secrets stay outside Git. Header provenance
uses relative symbolic keys; original private headers remain hash-sealed. Private
source readers independently reconstructed the two compact tables exactly before
transfer. Repository validation checks the pinned prepared projection, structure,
counts and privacy boundaries; it does not claim fresh private screenshot access.

Run `python3 validate.py` and `python3 -m unittest discover -s . -p test_validate.py`.
Governance checks are recorded in the active task. AUTHORING draft; CI and formal
independent review remain pending. No merge/deploy or review dispatch is authorized.

Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6849833911
