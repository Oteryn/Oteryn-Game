# Tibia 15.33 Item evidence: source census and Basic110

This packet changes no Item definition, pin, protocol, runtime or workflow.
Source: Oteryn/Oteryn-Game@83be72ad403ebc221dadba4b161c85fa1d47219a.
Observed product: Tibia Test Server Client 15.33.dd0955.
Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6837923337

## Frozen client scope

33 previous full-pass records plus Basic24–110:87 Basic records,
77 new manual transcriptions and 10 exact-identity reuses.
The union contains110 distinct name-signature observation groups, all Armors.
This is an incomplete category inventory. Basic111+ is excluded.
Raw OCR is not reference truth. No numeric Item ID was read from the UI:
repository keys are derived candidate full-name correspondences.

client-basic110.json stores verified names/properties and capture references.
Narrative descriptions retain hashes and lengths, not copied game text.
All screenshots/APNG and original text remain on the owner workstation.
client-proof-receipts.json identifies the retained bytes. No original assets
or copied source trees are redistributed.

The collector name signature is not a unique Item identity. A later targeted
Amulets search showed two separate rows named wolf tooth chain. Name-based
deduplication can merge variants. The frozen count describes observed cards/name
signatures, not all Item types or instances. Unique repository name matching is
not proof of a client numeric ID; ambiguous names require separate evidence.

## Entire enumerated source corpus

| Collection | Count / interpretation |
|---|---|
| Canonical Item identities |34111 unique keys |
| Canonical semantics / known-name records |12616 /12169 (6974 distinct names) |
| Native base / updates / starter |6077 /76 /10 |
| Offline merged membership |6090:73 replacements,3 additions,10 starters |
| Merged materializable flags |6014;not actual runtime admission |
| Materializable plusknown nonempty name |5975;still source gates only |
| Supplemental profiles |35 files,4108 occurrences,3581 explicit keys |
| Taxonomy / relations |12252 /880 records,1093 relation edges |
| Market metadata |5095 rows;appearance baseline 15.30 |
| Trade |324 services,12234 declared offers,2076 item keys |
| Held trade facts |457 in60 services;not executable offers |

Every record in 69 canonical shards and all 6077/76/10 inputs was traversed.
field-coverage.csv contains 1878 normalized paths across 20 complete enumerated
collections. This supersedes the frozen PR3148 census (1770 paths): domain
UNKNOWN objects had hidden 106 held-fact paths, wrapper metadata hid two taxonomy
paths, and array ancestry omitted its immediate collection parent. The same
source records and immutable receipt inputs are retained. `census.py` reproduces
the CSV from verified Git blobs; `test_census.py` guards both defect classes, and
`census-repair-receipts.json` records every changed row.

KNOWN, UNKNOWN, NOT_APPLICABLE, absence, empty, zero and null remain
distinct. Parent KNOWN never means complete children. These are structural
counts, not manual retail validation of every value.

Alias/binding targets all exist canonically. 39 supplemental keys and 1,963
market keys lie outside selected 6090. Selection gaps do not automatically
justify recollecting source values from the UI. Appearance objects also include
non-Items;membership counts are not missing-Item counts.

## Qualified source differences

| Item/field | Client 15.33 | Canonical | Merged6090 |
|---|---|---|---|
| Goo shell i19372 Weight |58.00oz |48.00oz |58.00oz |
| Death oyoroi i50260 Mantra |16 |15 |Record not selected |

Canary XML also declares Mantra 16, but remains OTS_HYPOTHESIS_ONLY evidence.
These are source/version conflicts, not proof of wrong runtime behaviour.
See qualified-differences.json for exact paths, captures and scope.
The names bearskin/bear skin and fireheart cuirass/Fireheart cuirass have
explicit bindings. Earlier case-sensitive fireheart-unmatched status is withdrawn.

## Different views and consumers

Static definition description, generated Look, type BasicDetails and dynamic
instance data are separate. field-catalogue.json records53 view/field definitions,
202 source-schema paths,32 modifier kinds andsource→consumer/wire routes.

Look uses semantics, count and distance through a separate UTF-8 result codec.
Its near-only description/weight and definition maxima are not remaining charges,
tier, occupied imbuements or decay. Type Catalogue requests lack instance metadata.
The formatter does not infer articles/plurals or render every richer Basic stat.

Identity key, source namespace,appearance ID and native definition_ref differ.
Numeric mapping or membership does not prove spritesheet, frames, timing, alpha, scale
or 15.33 pixel identity. A three-second preview does not prove a full cycle.
Trade directions, currency, conditions and residence are separate facts:
100 Silver Token is not 100 gp;held facts are not executable offers.

## Validation and boundary

From repository root:
- python docs/agents/evidence/items-15-33/validate.py
- python -m unittest discover -s docs/agents/evidence/items-15-33 -p test_census.py
- python docs/agents/evidence/items-15-33/validate.py --source-blobs
- python docs/agents/evidence/items-15-33/validate.py --evidence-root OWNER_WORKSPACE
- python tools/agents/validate_governance.py
- python -m unittest discover -s tools/agents/tests
- git diff --check

The source audit is complete for enumerated collections/routes; it does not
claim complete 15.33 inventory, every-value parity, active deployment,actual Look/
instance verification or graphical/gameplay1:1. No host, NAS or production access
is required to close this source-only slice. Later evidence andimplementation
belong to separate updates/branches. CI/review/merge status is not asserted.
