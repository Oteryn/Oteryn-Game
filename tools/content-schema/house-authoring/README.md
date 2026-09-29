# House authoring schema candidate v1

Static House definitions for `content/houses/` (owner `House/Area` in
[`OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md`](../../../docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md)).
CANDIDATE only: not a WorldProject/v2 contract, not runtime activation, and
`content/houses/` stays `READY_UNPOPULATED` until the schema is accepted.

Runtime House state (owner, rent payment, ACL/access lists, auction, custody) is
World/persistence state by the tree contract and is **not** modeled here. Rent
period, payment and eviction rules belong to `rulesets/economy/`. Tile membership,
doors and bed placements belong to `content/world/placements/`.

## Record

| Field | Source | Note |
|---|---|---|
| `identity` | derived | `oteryn:content.house.<slug of name>`, `definition-r1`; shared shape with `../monster-authoring/monster.schema.json` |
| `name` | client staticdata f2 | whitespace-normalized; verbatim in `provenance.source_name` |
| `kind` | client f8 / f10 | `private_house` \| `guildhall` \| `shop`; exclusive in all 995 houses |
| `town` | client f9 | `Area` ref `oteryn:content.area.city.<slug>`; city Areas are not populated yet |
| `entrance` | CrystalServer `entryx/y/z` | engine entry tile in front of the door (next to a House tile for 975 houses); the client does not ship it |
| `map_marker` | client f6 | staged as `entrance` by HOUSES-1, but it sits at or next to the footprint centre (328 exact, rest ±1 tile), so it is not the door |
| `size_sqm`, `beds`, `rent_gold` | client f7, f5, f4 | official values win over the engine |
| `entry_restriction` | client f3 | structured form of the only observed text, "Only Sorcerers can enter." (3 houses) |
| `footprint` | client staticmapdata | bounding box: `origin` (minimum x/y/z) + `width`/`height`/`floors` |
| `tiles` | client staticmapdata | House area: every position with a non-empty layout cell, `[x, y, z]`, sorted |
| `provenance` | both | client id + record digests, verbatim name/restriction text, engine House id (the id on House tiles in the engine map) |

## Cross-source check (CrystalServer `summer-update` @ `00ce02a5`, `data-global/world/world-house.xml`)

Joined 1:1 on engine `clientid` == client house id: 995/995, none unmatched.
`rent`, `beds`, `guildhall` agree for every house. `samples/conversion-report.json`
records the divergences: `size` differs for 812 houses (the engine counts tiles
differently; the official value is kept), one name (`Harbour Street 4` official vs
`Harbour Place 4`), and 10 engine entries outside the client footprint. Engine
`townid`s map 1:1 to client town names.

## House tiles and the staticmapdata cell order

Cell order is floors with ascending z, then x, then y; a cell's `skip` counts the empty
positions **after** it. `otbm_tile_check.py` derived this by scoring all 24 candidate
orders against the House tiles of the pinned CrystalServer `world.otbm` (gzip,
sha256 `dcb73554...d8d7`): the selected order matches 104,748 ground items, the next best
96,981. Client tiles cover 108,034 of the 109,744 engine House tiles (98.4%) and 442
houses have identical tile sets; the houses below 90% coverage are listed in
`samples/otbm-tile-check.json` (engine map drift). Doors and beds on these tiles are world
placement content (`content/world/placements/`), not modeled here.

## TibiaWiki BR

`wiki_br_houses.py` captures `Todas_as_casas` and every page it links to in the
`house-tibiawiki-br-capture.yml` workflow (the site challenges agent containers). It
discovers the infobox parameters instead of assuming them, joins by the parameter whose
values are client house ids, and reports agreement per field plus disagreement examples.
A local run against four Fandom pages joined on `houseid` and agreed on rent, size and
beds (4/4), which supports the official `size_sqm` over the engine value. The snapshot
stays a CI artifact; the facts file may be committed from it.

First runner capture (head `a6900ccb`, artifact
`house-tibiawiki-br-snapshot-a6900ccbed77168436e8c2d6b86350c8e8f20c5f`): 963 pages with an
infobox; no House id parameter, so 916 joined by name. Agreement with the official values:
rent 898/916, `size_sqm` 885/916 (vs 183/995 for the engine), beds 889/916, town
(`payrent`) 915/915. Most disagreements and the 47 unjoined pages (Lower/Upper Barracks,
Outlaw Camp, Tunnel Gardens 10-12, ...) are Kazordoon and Thais/Carlin flats that the 15.30
client lays out differently; the official client values are kept.

Owner in-game check (2026-09-29) agrees with the 15.30 client, not the wiki: The Lair 166 sqm,
3 beds; East Lane 2 108 sqm, 2 beds; Lower Barracks 1 25 sqm, 2 beds; Lower Barracks 11 does
not exist.

## Files

| File | Purpose |
|---|---|
| `house.schema.json` | JSON Schema 2020-12, closed shapes |
| `validate_houses.py` | schema plus semantic checks: key family, unique key/source id/engine id/name, marker inside footprint, name normalization, restriction text ↔ structured form, shop naming |
| `verify_formal_schema.py` | positive/negative cases; regenerates `synthetic-valid-house.json` |
| `otbm_tile_check.py` | local-only: pinned `world.otbm` House tiles → `samples/otbm-tile-check.json` (cell order evidence) |
| `wiki_br_houses.py` | TibiaWiki BR `fetch` / `facts` / `compare` / `self-test` |
| `convert_houses.py` | `extract-crystal`: pinned `world-house.xml` → `samples/crystal-world-house-00ce02a5.json`; `convert`: joins it with `imports/cipsoft-staticdata/houses/` (digest-checked), validates all 995 houses, writes `samples/conversion-report.json` |

```text
pip install -r requirements.txt -r requirements-dev.txt
python verify_formal_schema.py && git diff --exit-code -- .
python validate_houses.py synthetic-valid-house.json
python convert_houses.py convert --check            # all 995 houses validate; report unchanged
python convert_houses.py convert --out /tmp/houses.json
# needs a crystalserver@00ce02a5 checkout (local only):
python convert_houses.py extract-crystal --xml <crystal>/data-global/world/world-house.xml --check
python otbm_tile_check.py --otbm <crystal>/data-global/world/world.otbm --check
python wiki_br_houses.py self-test
```

## Owner decisions (2026-09-29)

1. House key: slug of the official name. 2. Town: city `Area` ref. 3. `entrance`: engine
entry tile. 4. Next: House tiles from the official layout, verified against the engine map
(done here), then the TibiaWiki BR capture (workflow here; facts to commit after its first run).
