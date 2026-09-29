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
| `entrance` | CrystalServer `entryx/y/z` | engine entry tile; the client does not ship it |
| `map_marker` | client f6 | staged as `entrance` by HOUSES-1, but it sits at or next to the footprint centre (328 exact, rest ±1 tile), so it is not the door |
| `size_sqm`, `beds`, `rent_gold` | client f7, f5, f4 | official values win over the engine |
| `entry_restriction` | client f3 | structured form of the only observed text, "Only Sorcerers can enter." (3 houses) |
| `footprint` | client staticmapdata | bounding box: `origin` (minimum x/y/z) + `width`/`height`/`floors` |
| `provenance` | both | client id + record digests, verbatim name/restriction text, engine House id (the id on House tiles in the engine map) |

## Cross-source check (CrystalServer `summer-update` @ `00ce02a5`, `data-global/world/world-house.xml`)

Joined 1:1 on engine `clientid` == client house id: 995/995, none unmatched.
`rent`, `beds`, `guildhall` agree for every house. `samples/conversion-report.json`
records the divergences: `size` differs for 812 houses (the engine counts tiles
differently; the official value is kept), one name (`Harbour Street 4` official vs
`Harbour Place 4`), and 10 engine entries outside the client footprint. Engine
`townid`s map 1:1 to client town names.

TibiaWiki BR (`Todas_as_casas`) is not cross-checked yet: it is behind a Cloudflare
challenge from agent containers, so it needs a capture workflow like
`npc-tibiawiki-br-capture.yml`.

## Files

| File | Purpose |
|---|---|
| `house.schema.json` | JSON Schema 2020-12, closed shapes |
| `validate_houses.py` | schema plus semantic checks: key family, unique key/source id/engine id/name, marker inside footprint, name normalization, restriction text ↔ structured form, shop naming |
| `verify_formal_schema.py` | positive/negative cases; regenerates `synthetic-valid-house.json` |
| `convert_houses.py` | `extract-crystal`: pinned `world-house.xml` → `samples/crystal-world-house-00ce02a5.json`; `convert`: joins it with `imports/cipsoft-staticdata/houses/` (digest-checked), validates all 995 houses, writes `samples/conversion-report.json` |

```text
pip install -r requirements.txt -r requirements-dev.txt
python verify_formal_schema.py && git diff --exit-code -- .
python validate_houses.py synthetic-valid-house.json
python convert_houses.py convert --check            # all 995 houses validate; report unchanged
python convert_houses.py convert --out /tmp/houses.json
# needs a crystalserver@00ce02a5 checkout (local only):
python convert_houses.py extract-crystal --xml <crystal>/data-global/world/world-house.xml --check
```

## Open decisions before acceptance

1. House key: slug of the official name (current) or of the client id.
2. Town: reference a city `Area` (current, keys not yet defined) or a separate Town identity.
3. `entrance`: engine entry tile (current) until the client door position is derived from `staticmapdata`/map tiles.
4. Tile membership: next step is the cell order → x/y/z mapping (HOUSES-1 UNKNOWN), or engine `world.otbm` House tiles, into `content/world/placements/`.
