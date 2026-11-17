# Nomads Land window evidence

Public source checked 2026-11-08: https://www.tibia-wiki.net/index.php?title=Nomads_Land_Quest/Spoiler&oldid=322146
HTML SHA-256: `b5909d2cab61c88d58487803785f30a34d4684cdaa96d79fa0c8f8ee47633239`. This independent Polish guide is available; the earlier Fandom/BR fetch failures do not establish absence of game data.

The window instruction specifies a crate with a Large Amphore placed on top. Its annotated route is https://tibia-wiki.net/images/b/b4/Droga.PNG (SHA-256 `bdb754c2860e0c9fe27a8df2d83904e09c663bc5435040763c5f2e26400ccfb7`), marker 5. The temple-pillar entrance link identifies surface position `(33185,32844,7)`; that is not the dungeon window coordinate.

The M02 engine fixtures deliberately use fixture coordinates/definitions and a three-root arrangement to exercise ordering and duplicate-definition custody. They do not assert that three roots are the Nomads source solution. This guide supports a two-root authored requirement; exact native window/cabinet placements and canonical Item mappings still need qualification before enabling the production stage.

## Native-map cross-check

The annotated floor plan has a unique local alignment at source origin `(33183,32831,9)` in `content/world/placements/region-z09-x129-y128.b3`: 1463 of 1571 sampled floor/wall cells agree (the next candidate agrees in 1166 cells; portable props/text account for some differences). This is candidate identification, not a substitute for a registered selector/proof.

The captured room contains drawers `2434` at `(33220,32858,9)`, a large amphora `2904` at `(33218,32853,9)`, and three crate `2471` roots at `(33218,32854,9)`. The drawer is root ordinal 1 above ground appearance 7358. The default three-crate pile and the guide's singular crate wording must not silently become a claimed source quantity rule.

Existing bindings identify crate `oteryn:item.tibia.i2471`; its hireling profile is pickupable, 8000 weight and container capacity 15. The house profile identifies large amphora `oteryn:item.tibia.i2904` as movable, non-pickupable and Ground-only. Enabling a consumer therefore also requires checking the native Ground-to-Ground move path, rather than treating the amphora as a carried bag Item.

Current region SHA-256: `5f1c4081bc48d6e2481d0e29f37fed6b6823d6e749d1d9e75ecbfba15706d3bc`.
