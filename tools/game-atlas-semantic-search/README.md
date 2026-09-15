# Game-owned Atlas semantic search source

This tool produces the normalized public-safe `semantic-search-source-v1` snapshot consumed by Oteryn Atlas.

## Authority

`Oteryn/Oteryn-Game` remains World/Content and public-export authority. Legacy OTBM/Crystal data is accepted only at the offline migration/import boundary. Atlas and browser runtime must never parse those sources or use them as fallback authority.

The exporter combines:

- the already accepted Game `static-creatures-v1` export for NPC/monster placements;
- Game/import-side Town and Waypoint records decoded through the pinned legacy semantic parser;
- conservative public NPC service summaries derived from static definitions.

No AID/UID, legacy numeric town identifier or browser-generated canonical identity is published.

## Record model

Every record exposes:

```json
{
  "kind": "npc",
  "id": "producer-owned exported record id",
  "label": "Sam",
  "aliases": [],
  "position": {"x": 32361, "y": 32198, "floor": -7},
  "bounds": null,
  "provenance": {},
  "capabilities": ["shop", "static-placement"]
}
```

Town/Waypoint identity derived from migration evidence remains explicitly unresolved in provenance; their `id` is a deterministic Game export-record identity, not a promoted legacy ID.

## Coordinate input aliases

The snapshot explicitly publishes the accepted legacy import aliases `z=0..15 -> native floor=-z`. Atlas may therefore accept a user coordinate such as `32369 32220 7` while navigating internally to native floor `-7` without inferring legacy floor semantics itself.

## Invocation

Use a validated Game `static-creatures-v1` JSON plus either a Game-normalized navigation JSON or the exact pinned legacy checkout/map:

```text
python tools/game-atlas-semantic-search/export.py \
  --creatures /path/static-creatures.json \
  --npc-root /path/crystalserver/data-global/npc \
  --legacy-root /path/Otheryn \
  --map-path /path/Otheryn/vendor/map-analysis/crystalserver/data-global/world/world.otbm \
  --output /tmp/semantic-search-source.json
```

The pinned map SHA-256 is `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`; other map bytes fail closed.

Run `python tools/game-atlas-semantic-search/self_test.py` for deterministic/negative fixture coverage.
