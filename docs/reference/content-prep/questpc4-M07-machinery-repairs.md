# M07 Machinery: six atomic repair sites

The selected recipe `tools/content-schema/quest-authoring/samples/completion242/recipes.json`, identity `oteryn:quest.machinery_of_war_quest`, s2/s3, requires44 Bricklayers' Kits for four outposts and two wall runs. It selects level50 and free-account eligibility. These callbacks implement those six physical occurrences, not Baxter's commission, war plans, finale or completion.

The captured B3 map contains exactly44 broken-brick segments in these six clusters:

| site | inclusive x/y bounds | kits | native control (x,y,ordinal,appearance) |
| --- | --- | --- | --- |
| outpost01 |32793..32798 /31731..31737|7|32797,31731,2,6280|
| outpost02 |32771..32776 /31774..31780|7|32775,31774,3,6280|
| outpost03 |32791..32796 /31812..31818|7|32795,31812,3,6280|
| outpost04 |32852..32857 /31816..31822|7|32853,31816,3,6280|
| wall01 |32798..32805 /31769..31778|8|32798,31769,1,6282|
| wall02 |32808..32815 /31789..31798|8|32808,31789,1,6282|

All are source floor7. Evidence: `content/world/placements/region-z07-x128-y123.b3`, SHA256 `865ee3aa9e0c4d592571b40b72c137a8b05a19f850e0b570974276ebbeb9a22d`, and `region-z07-x128-y124.b3`, SHA256 `bdcf1412c9d0ea16670cd16ed415740da23293bc12f30cfc16cbe1bd04194f66`. The authoring test decodes both and verifies all44 distinct segments, control ordinals and exact budget7+7+7+7+8+8=44. No placements change.

`imports/canary/items-xml/items.xml:3351-3353,17243-17245,21014-21017`, revision `47dfd51f45280a59a1d3e50ba7edd573d7234446`, identifies1270..1280 as brick walls,6280..6283 as broken brick walls, and7785 as a kit whose description says to repair destroyed brick walls. File SHA256 `1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`. Source URL: https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml . The checked Crystal `actions/tools/bricklayer_kit.lua` only implements a bridge; its action-id is not assigned to these objects.

Chosen Oteryn adaptation: one use of a site's declared native control atomically repairs all its broken segments, consuming exactly one plain carried7785 per segment. Four outposts cost7 each; two wall runs cost8 each. Each site has a personal once flag, a bounded six-site counter and a bounded44-kit counter. Physical preparation may precede the commission, but never starts or completes the canonical quest. Existing non-stackable instances are consumed separately inside one transaction; no impossible seven-item stack is manufactured.

The visual mapping is authored: horizontal6280/6282 becomes plain brick wall1271; vertical6281/6283 becomes1277. Surviving nearby walls already use these plain brick appearances. This is not a recovered donor transform table or AID. The full set, including the clicked object, explicitly projects solid collision. These volatile world projections last until another authorized world projection or channel restart; personal credits persist. No original dynamic war schedule is inferred.

`replacement_passable:false` is an opt-in repair capability. Admission requires an exact native player USE, irreversible personal writes, a bounded consumed input, exact immutable appearance predicates for every segment, matching source transform, no conditional missing pieces or timers. Legacy replacement passages retain their old passability default. Occupied source or remote cells refuse before durable input reservation. The repair input resolver locks up to64 positive-quantity exact-definition instances and their plain custody locations, resolves the entire required amount before writes, and reuses existing Quest consumption proofs/BURN writes. Retained tools, substitutions and the Rotten Blood sacrifice path remain unchanged.

The native PG story uses unchanged captured tiles and real kit MINT/custody transfers. It checks six-kit refusal, occupied source and remote refusal, mismatched segment refusal, seven separate kits consumed in one transaction, solid collision for every repaired segment, exact personal/counter writes, receipt replay, and no further charge after an external damage projection with a fresh seven-kit budget.
