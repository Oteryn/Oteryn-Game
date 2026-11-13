# Client-local static evidence

Five unchanged, prepared whitelists make the available offline static audit
reviewable. They total550,926B and compare against Game main
`3e168ce6afe374485da20ee0ac13d3749558e2f8`. This auxiliary evidence changes no
content definitions, client profiles, source contracts or runtime admission.
The original full static audit remains **PARTIAL**.

| File | Available evidence | Qualification held |
|---|---|---|
| `achievements378-metadata.json` |378 client records;368 exact-name source candidates;367 Description custody matches and1 difference |Raw field4 meaning, grade, points and flags are unqualified;203 unmatched secret source records do not prove client omission |
| `creatures845-classes21-bosses453-summary.json` |845 creatures,21 classes,453 bosses; name-candidate population and raw-field histograms |Raw numeric meanings, look rendering, numeric class links and runtime admission remain unknown |
| `quests104-title-custody.json` |104 records;102 exact old raw-record matches and2 new client IDs192/195 |Client quest IDs are not runtime Quest/storage/progress identities; title and mission narratives are excluded |
| `houses995-source-metadata-columns.json` |995 explicit provenance joins;995 raw static-record and qualified metadata matches;994 layout-record matches,1 difference at55015 |Map marker is not engine entrance; layout semantics, collision, doors, ownership, ACL, auctions and rent cadence remain unqualified |
| `maps473-structure-summary.json` |473 areas;463 exact old raw records,2 changed and8 new;212 mask references,1297 counted markers |No marker semantics, image pixels, rendered map, canonical area identity or world admission |

These populations are separate; their totals are not a single count of game
objects. Creature unique/ambiguous/no-name candidate counts are798/21/26;
boss counts386/8/59. All21 class labels match as sets, without proving the
numeric creature-to-class relationship.

## Reading the column tables

Achievement rows retain exact static name labels and raw encoded IDs. Their
Description fields contain only hash/length/state custody. All source-key
relationships in this member remain exact-name **candidates**, never ID bindings.
Raw field4 varints1/2/3 retain their literal histograms without becoming grades.

Quest title custody retains UTF8 hash/byte/codepoint counts, not title text.
New client records192/195 have no old raw-record hash and a false same-old-title
flag. Neither old102 nor current104 constitute runtime profile admission.

House rows use the owner contract's explicit
`cipsoft/staticdata/house_id` to existing catalogue `provenance.source_id` join.
The full existing source key is `catalogue_key_prefix + source_gamekey_suffix`;
this is a qualified provenance relationship, not a name-only guess and not a
runtime World House identity. SHA256 dictionary entries encode32 digest bytes
as43 base64url characters without padding. Repeated digest references preserve
both current and main raw-record hashes. Names and restrictions remain hash
custody only; town labels are static metadata. Map marker is presentation data.
No engine entrance coordinates were selected or compared. Six known-zero bed
counts remain0, not absent or unknown. Layout55015 differs only at the raw-record
level here; it needs the map owner's semantic qualification.

Map structure is a bounded summary, not a full image/mask export. The one
multiply-parented child remains a count, never a first-parent identity rule.

## Provenance and validation

`provenance.json` pins all five payload bytes and their original private prepared
manifest hash. Source comparisons, parser pins and custody hashes are retained
inside the approved payloads. Historical classification strings such as
PREPARED_PRIVATE_NOT_PUBLISHED describe the immutable preparation snapshot;
the live PR controls publication lifecycle. Original bytes were not rewritten to
update historical labels.

Run from repository root:

```sh
python3 docs/agents/evidence/static-local-data-20261113/validate.py
python3 -m unittest discover -s docs/agents/evidence/static-local-data-20261113 -p test_validate.py
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
git diff --check
```

The standalone validator verifies exact payload hashes, coverage, row/hash
references, candidate states, known zeros, namespace/admission holds and privacy
structure. Negative cases reject substituted raw hashes, lost zero beds, hidden
layout differences, runtime promotion, narrative/capture paths and merged map
provenance populations. This does not claim fresh access to private originals
or an independent native re-read in Cloud.

Original assets, screenshots, private filesystem paths, name/restriction/title
narratives and account/character state stay outside Git. No GUI input, anti-logout,
network collection, builders or game compilation occurred for this publication.
Items/proficiency data already published in3170/3173 are not duplicated or changed.
Formal independent review and exact-head CI are separate, pending qualification;
this draft does not authorize merge, deployment or source admission.

Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6853408185
