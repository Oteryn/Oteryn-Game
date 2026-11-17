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
