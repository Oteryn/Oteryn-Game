# TibiaWiki BR Creature source catalog

This import preserves **2,149 direct Creature pages** by MediaWiki page ID.
Each record carries the observed title, exact page revision, UTC revision
timestamp, raw UTF-8 SHA-256, source category observations, and short scalar
infobox observations. Categories are retained verbatim as Wiki evidence, not
silently interpreted as Oteryn runtime types. These
values are strings as published by the source; for example, `mitigation` is
not silently converted to a gameplay percentage, and `pushable` retains the
original Portuguese `sim`/`não` value. Missing fields are absent, never zero.

The page list is the pinned G3 direct-family classification. The G4 source
capture independently pins the exact page ID, title, revision and raw digest
for every record. A previously superseded G4 extraction supplied the scalar
values. `tools/content-census/stage_tibiawiki_creatures.py` verifies all 2,149
extracted rows against both archives before retaining them; the old artifact
is **not** treated as a qualified crosswalk. Page `63947` changed between G3
and G4, so its retained fields refer to the exact later G4 revision.

The import is a complete catalog of the **directly classified Creature source
pages**, with the bounded infobox fields available from the preserved
extraction. It does not contain all Wiki article fields, attacks, loot,
resistance tables, Bestiary overlays, or appearance assets. It does not create
native Oteryn Creature identities or turn source values into runtime
definitions. `content/creatures/definitions/` has no accepted authoring schema
yet; the subsequent source comparison and explicit identity binding can use
these exact page/revision tuples without re-matching names.

Reproduce from GitHub Actions artifacts `10801778929` (G3), `10831362943`
(G4 raw digest capture) and `10847703280` (scalar extraction) with:

```sh
python tools/content-census/stage_tibiawiki_creatures.py \
  --g3 /path/to/source-family-classification.zip \
  --g4 /path/to/g4-nonitem-source-capture.zip \
  --fields /path/to/superseded-g4-bulk.zip \
  --output imports/tibiawiki/facts/creatures
```

The generator checks the three fixed archive digests, the 2,149 direct
family decisions, every G4 revision/digest tuple, source shape and field
allowlist. `manifest.json` records the exact outputs and field coverage.
