# Client-local Items data and bounded comparison evidence

This snapshot makes prepared local-data evidence reviewable:6,470 Cyclopedia-bearing
client appearance records and exact item names representing6,228 client types;
Description hash/count/state custody;157 alias/taxonomy groups;15,350 raw NPC tuple
occurrences;115 candidate comparison rows;9 correction records/8groups; static-data
readiness and a12-field weapon comparison supplement. Complete available-field
comparison tables cover the whole6,470 appearance cohort. Thirteen data files
total5,082,415B; this is not full item-stat or runtime parity.
It changes no runtime or source definitions.

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
- `item-names-description-custody.json`: all6,470 exact UTF8 item names and their
  byte/codepoint/hash receipts. Description literals are excluded:3,697 fields
  absent,2,771 UTF8 qualified,2 encoding UNKNOWN. Absent/unknown text counts stay
  null, never zero; no encoding is guessed.
- `alias-taxonomy-matrix.json`:157 groups preserve every separate appearance and
  compare name/Description custody/market/NPC-sequence variants.1,056 types wholly
  lack a market category and40 have mixed presence. These are client taxonomy
  facts, not missing game items, ID bindings or permission to merge records.
- `static-dataset-readiness.json`: raw structural counts845 creatures,21 bestiary
  classes,453 bosses,104 quest lines,378 achievements and995 houses; map and444
  proficiency-record metadata. Current raw assets fail old15.30 profile gates;
  gates remain unchanged, runtime is not admitted, no UI tab is declared complete.
- `weapon-comparison-supplement.json`: four Creature Products cards113/116/725/729,
 12 observed Attack/Defence/Weapon Type fields. Canonical has8 literal matches/4
  unqualified label differences; selected has6/3 plus3 NOT_SELECTED comparisons.
  Source axe/sword versus retail cutting/stabbing labels remain candidate-only.
  Secondary UNKNOWN fields retain raw state even when the formatter uses a fallback.
- `whole-cohort-name-candidates.json`: all6,470 appearances, including all1,288
  lacking market category, with complete independently indexed candidate keysets.
  Canonical:4,876 unique candidates/1,179 ambiguous/415 none; selected source view:
  3,244/327/2,899. All157 aliases and exact-name references stay separate.
- `whole-cohort-description-status.json` and its summary: all6,470 client states
  and17,373 ordered candidate evaluations. Arrays align every candidate in the
  name matrix; no first/best candidate is selected. Comparison uses exact UTF8
  custody versus source text hashes, normalization NONE; Description text stays
  private. Source KNOWN_EMPTY, UNKNOWN/N/A and ancestor ABSENT remain distinct.
- `whole-cohort-npc-axes.json` and its summary:14,235 tuple occurrences with both
  raw sale/buy axes per layer (28,470), including12,432 known-zero axes. Numeric
  equality is candidate-only; quantity, currency, residence, direction, eligibility
  and executable offers are not established by that equality.

The five thin table/summary blobs retain their original custody filenames inside
metadata; `validate.py` resolves the explicit filename aliases through their fixed
SHA256 pins. All candidate sets, array positions, price axes and source states
are preserved, not merged. Fresh main3e168ce6 has72 item-source blobs and four
selection-route files byte-equivalent to5efd67da. The6,090-record selected layer
is the provider/update/disjoint-starter comparison view, not deployed admission.

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
from the selected bulk comparison. Augments, Imbuement Effect, instance expiry/charges
and NPC runtime offers remain unqualified. Only the supplement's four bounded melee
cases have a source display projection; no general weapon/runtime parity is claimed.
Speed/ratio/sign/zero-display formatter differences need their own qualification.

Original assets, screenshots, raw private exports, Description narratives, quest
labels, absolute private paths and secrets stay outside Git. Header provenance
uses relative symbolic keys; original private headers remain hash-sealed. Private
source readers independently reconstructed the two compact tables exactly before
transfer. Repository validation checks the pinned prepared projection, structure,
counts and privacy boundaries; it does not claim fresh private screenshot access.

Run `python3 validate.py` and `python3 -m unittest discover -s . -p test_validate.py`.
Governance checks are recorded in the active task. Draft stays PARTIAL; exact-head CI
and formal independent review are separate qualification. Embedded historical PR/head
strings in source packets are custody snapshots, never current lifecycle authority.
No merge/deploy or review dispatch is authorized. ZERO client clicks/keys/focus/scroll,
modal handling, anti-logout or collector/supervisor restarts remains binding.

Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6849833911
Material append/reopen: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6850459977
Final current-batch reopen: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6851110135
