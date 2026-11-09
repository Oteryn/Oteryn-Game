# M38 Anna and Targuna source witnesses

Anna's pinned donor is [Crystal 201189f, anna.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/anna.lua#L67).
The captured file SHA256 is `5ad524861ac070192466d751215e9be859f6cea67a3d8fa2612e43a12b4fdd2e`.
Lines67–68 specify landing `(31938,31928,7)` and minimum level8. Lines98–107
teleport, reset the three Newhaven flags, and select the named Targuna town.

The existing repository witness
`tools/content-schema/area-authoring/samples/crystal-world-towns-00ce02a5.json:1`
has SHA256 `99cd4af290d68279c0b417ad85f0f6c9c1b9835413ea0558442df958a71f0c64`.
It captures Crystal `00ce02a57ca5a12e48f32a3476e37471167e4c3f`,
`data-global/world/world.otbm`, SHA256
`dcb735549bd11de526c4bd441bbf62e4490efbb60ff7aa334530f1692345d8d7`.
Its Targuna record is town34, temple `(31934,31925,7)`; Newhaven remains town33.
The existing 15.30 Area source
`content/world/areas/regions/areas-00000-00442.json:1`, SHA256
`7c41518413b7280a5c72691f03d1aa75b8e4c992bb56089e2304651514cc0059`,
identifies Targuna as area526 under Thais.
These witnesses have different pinned revisions and are joined explicitly by name/id;
the historical files are unchanged.

The first native slice retains both reviewed tutorial towns in the compiled catalogue,
reusing Newhaven's existing source-town adapter. It keeps the current entry/default,
ordinary hometowns, masks and movement safety rules. Duplicate names or a relabelled
town id refuse compilation. This is Oteryn native admission from the engine-town
witness, not a claim that the original client Area classified Targuna as a hometown.

Anna's existing Oteryn Thais default remains unchanged. Targuna departure is not enabled
by catalogue admission alone: its confirmed route, actual placement, citizenship and
tutorial reset must qualify together, including reconnect/replay after pending arrival.
The existing HomeTownRelocation owner already composes arrival consumption, position
and citizenship in one fenced transaction; reuse it for that continuation.

Native landing investigation after c95897fdb: the repository B3 decoder (`tools/content-schema/world-authoring/world_region_codec.py:370`) finds `(31938,31928,z7)` with flags1, house0, and one top-level palette94 entry. The pinned placement index resolves palette94 to `oteryn:item.tibia.i231`; Terrain `oteryn:terrain.tibia.i231` declares ground, walkable=true and speed160. This is a raw-input fact, not a substitute for the compiler/runtime landing gate. The canonical Anna placement is `(32552,32497,z7)`, with Lua definition SHA2565ad524861ac070192466d751215e9be859f6cea67a3d8fa2612e43a12b4fdd2e. No NPC placement was found on the Targuna landing itself. Input SHA256s:

- `content/world/placements/region-z07-x124-y124.b3`: `d4354c21e1d3342f9ef92b9f7f249ef3c484d4da3d67d88bbff1a7da19deff93`
- `content/world/placements/index.json`: `f770da7cceec09faf63e6086fa099efa001db57045d4e9906d0057ff42c90af6`
- `content/world/terrain/terrain-00000-00499.json`: `d54273ee59a6703392a27ecc0ea0778961b66a2abdcba52167beb7548badfd21`
- `content/world/npc-placements/placements-00000-00055.json`: `d5d0a7bd0a5d382364fc79cf7ea943ac4cd5cc2efd9577afbaa609ac5c4ba993`

Static geometry closure at these three anchors also requires appearance-only IDs
20661, 34434 and 51921. Source: CrystalServer revision
`00ce02a57ca5a12e48f32a3476e37471167e4c3f`,
[appearances.dat](https://github.com/zimbadev/crystalserver/blob/00ce02a57ca5a12e48f32a3476e37471167e4c3f/data/items/appearances.dat)
(SHA256 `17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1`)
and [items.xml](https://github.com/zimbadev/crystalserver/blob/00ce02a57ca5a12e48f32a3476e37471167e4c3f/data/items/items.xml)
(SHA256 `13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece`).
The exact normalized flags are retained in
`tools/content-schema/world-object-authoring/samples/npcfix-travel-source-flags.json:1`
(SHA256 `e4a8aad823a249072f8050c4ae101f6e941ae7be430ee31f1e8512e408af458f`)
and reproduced by `npcfix_travel_static.py --donor-dat ... --donor-xml ...`.
All three are absent from that XML and the Item binding catalogue. The admitted
15.30 appearances artifact independently has the identical flags. In particular
20661 is a clipping, unpassable border; qualification must preserve its collision.
This bounded Terrain corpus does not admit Item identities or interactive hooks,
and does not enable Anna's Targuna offer.
