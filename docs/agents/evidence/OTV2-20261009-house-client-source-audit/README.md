# House client source audit: 15.30 to 15.33

Task: `OTV2-20261009-house-client-source-audit`. Owner requested retention of this audit in Oteryn/Oteryn-Game on 2026-10-09.
Scope: this evidence directory only. Single writer: Codex chat `01a0f366-2aba-7352-ae39-21eba2889b57`, branch `codex/house-client-audit-20261009`.

## Result and authority

**PROVEN:** all 995 source IDs in the installed client `15.33.b21348` match the historical 15.30 House catalogue.
Every one of the 995 raw staticdata House records has the same SHA-256 as the source-record digest retained in that catalogue.

The comparison baseline is immutable Game commit `4cf5ff2298666069fdcf2cb821611e0d6de079ad`, used by the earlier audit on 2026-09-30.
The publication base is `1e77ca22b4fcb7869f8f1d169f4e12bc749b295d`; this evidence does not claim to re-audit runtime at that later revision.
The source observation is the owner-provided running client in `otclient-track-a-kasmvnc`, read directly through Remote Desktop Commander on 2026-10-09.

| Comparison | Result |
| --- | --- |
| House IDs | 995 old, 995 new; no additions or removals |
| Ordinary houses | 878 |
| Guildhalls | 66 |
| Shops | 51 |
| Rent, size, maximum beds | Identical for all 995 |
| Town, restrictions, guildhall/shop flags | Identical for all 995 |
| Client map markers | Identical for all 995; these are not verified physical door entrances |
| Raw staticdata House record hashes | 995 identical |
| Raw staticmapdata House layout record hashes | 994 identical, 1 changed |
| Source towns | 19 |

The complete source ID set and per-record hashes are retained in `client-observations.json`.
Whole-file SHA-256 pins for both 15.33 inputs are retained in `audit-report.json`.
Original client binaries, sprites, layout image payloads, credentials, account details, tenants, paid-until dates and screenshots are not redistributed in this packet.
Static facts and bounded appearance-ID comparisons are reference evidence, not an accepted ruleset update or an asset redistribution grant.

## Catalogue name normalization

`audit-report.json.field_changes` compares current raw source text to the normalized authoring names, not old raw source text to new raw source text.
The only two authoring/text differences are already-existing whitespace normalization:

- ID 101: source `Targuna Cottage  1` has two spaces; authoring uses `Targuna Cottage 1`.
- ID 30602: source `Tunnel Gardens 2 ` has a trailing space; authoring uses `Tunnel Gardens 2`.

**PROVEN:** the raw source records themselves are unchanged between versions, including those two names.

## Lakeside Mansion

**PROVEN:** source ID `55015`, Lakeside Mansion, Svargrond, is the only changed layout record.
Rent remains 300,000 gold, size 149 sqm, maximum beds 5, client map marker `(32326,31147,6)`.

Old and new layout origin are `(32322,31142,4)`, dimensions `10 x 18 x 4`, and encoded cell count 287.
The 287 cells describe the encoded preview, not the declared 149-sqm rentable area.
Decoded positions use ascending z, then x, then y; skip counts advance after the current cell.

| Client coordinate x,y,z | Old appearance-ID stack | New appearance-ID stack |
| --- | --- | --- |
| 32326,31154,7 | 799,6629,6757,4982 | 4600,6675,4982 |
| 32326,31155,7 | 4600,4733,1755 | 4600,1755 |
| 32327,31154,7 | 4599,6682,6675 | 4599,6675 |
| 32329,31150,7 | 4598,4736,6677 | 4598,6677 |

**UNKNOWN:** appearance semantics, collision, walkability and door/bed effects of these changes were not qualified.
Unchanged origin, dimensions and encoded positions do not prove unchanged server geometry or collision.
Both raw layout record hashes and all four changed stacks are retained in `lakeside-layout-diff.json`.

## Method and reproducibility

The strict protobuf reader was taken from
`tools/content-census/stage_staticdata_houses_achievements.py` at the immutable baseline commit.
Its `stage_houses(staticdata, staticmapdata)` function was called directly with the installed 15.33 bytes.
The 15.30 generator CLI was not run and its digest pins were not relaxed.

The parser checks wire types, exact record shapes, UTF-8, booleans, unique IDs, joined House ID sets and the layout cell-plus-skip count.
For each source ID, source-record SHA-256 was compared to
`content/houses/houses-*.json.houses[].provenance.staticdata_sha256` at the baseline.
Layout record SHA-256 was compared to `provenance.staticmapdata_sha256`.
The four differing cells were compared to the full retained staged source record in
`imports/cipsoft-staticdata/houses/houses-00500-00749.json` at that same commit.

The packet retains extracted facts and hashes to verify the ID set, kind counts, scalar comparisons and the reported layout hash counts independently against the baseline.
Replaying byte parsing additionally requires the exact owner-provided 15.33 files named and hashed in the report.
A later live client update must not replace this capture or its pins.

## Earlier schema and integration audit

The following findings belong to the earlier audit at the immutable baseline, not a claim about current protected main:

- **PROVEN:** combined House/Area validation passed for 995 Houses and 465 Areas; House formal checks passed 28/28, Area formal checks 14/14.
- **PROVEN:** the historical compiler admitted House engine-ID membership only. Full authoring identity, revision, tiles and ACL-qualified doors were not represented by a complete admitted House runtime family.
- **PROVEN:** Area generator checks used Windows path separators against forward-slash output keys; single-file Area CLI validation also rejected city parents supplied in other shards. These were tooling findings, not proof of corrupt combined data.
- **PROVEN:** House rebuilds could preserve an old identity revision after a material scalar edit; the validator could admit overlapping interiors with distinct doors. These were future-update gate gaps, not detected corruption of the existing corpus.
- **PROVEN historical closeout:** PR #1366 retained the official bed count and engine entrance, with 46 entrance and 84 bed-placement divergences routed to the base-map lane. This source comparison does not close those checks.

## Limits and next disposition

This confirms completeness and compatibility of static House source facts, including all 66 guildhalls.
It does not qualify Oteryn ownership, guild authority, auctions, rent collection, ACL, eviction, persistence, restart recovery, physical entrances or playable runtime.
Dynamic world rental status belongs to runtime state and must not be copied into static definitions.

If the accepted map/content baseline remains 15.30, retain this as comparison evidence.
If a separately authorized versioned update adopts 15.33, qualify the four Lakeside Mansion cell changes with the map owner before promotion.
No existing 15.30 source files, catalogue records, contracts, rules, validators or runtime were changed by this retention task.
