# Tibia worldmap reconstruction tool

This tool is the proprietary-data-free normalization and comparison layer harvested from `blakinio/Oteryn-Platform` PR #1006 into the canonical native game/client repository.

It does **not** contain or extract proprietary Tibia assets by itself. Runtime/asset evidence is transformed outside Git into normalized IDs and classifications; only safe normalized data may be passed to this tool.

## Contract

Input format is `oteryn-worldmap-normalized-v1`. Every tile has an explicit `(x,y,z)`, `observed` state, monotonic `sequence`, ordered contents, semantic class, client appearance ID and explicit mapping state.

Never use an empty tile to mean “not seen”. A tile that has not been received/observed must carry `observed: false` or be absent and is handled separately by comparison.

Semantic classes:

- `ground`
- `ground_border`
- `static_item`
- `dynamic_item`
- `creature`
- `npc`
- `effect_or_ephemeral`
- `unknown`

Mapping states:

- `MAPPED` — `server_otb_id` is proven and required;
- `UNMAPPED` — no server ID is guessed;
- `NOT_APPLICABLE` — the content is intentionally outside static OTBM item translation.

Creatures/NPCs belong in the document `entities` stream and are excluded from static OTBM comparison unless separately classified as static content. One sighting must never be converted into a proven spawn definition.

## CLI

```bash
PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 tools/tibia-worldmap-reconstruction/run.py validate map.json

PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 tools/tibia-worldmap-reconstruction/run.py compare reconstructed.json reference.json --output diff.json

PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 tools/tibia-worldmap-reconstruction/run.py merge accumulated.json next-capture.json --output merged.json

PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 tools/tibia-worldmap-reconstruction/run.py otbm-plan reconstructed.json --output export-plan.json
```

`otbm-plan` is intentionally **not** a binary OTBM writer. It exits with status `2` while any observed tile has an unproven mapping or no proven ground. A real OTBM writer is allowed only after client/object ID -> server/OTB ID translation is proven.

## Reference comparison

The same normalized representation is used for reference inputs. Candidate reference adapters include CrystalServer/OTBM content, Renemap and TibiaMaps geographic/coverage evidence. Those sources are evidence/reference inputs, not canonical runtime authority.

Comparison is coordinate/structure based, never OCR/visual matching. Current result statuses are `MATCH`, `MISSING_IN_REFERENCE`, `MISSING_IN_RECONSTRUCTION`, `GROUND_MISMATCH`, `ITEM_MISMATCH`, `STACK_ORDER_MISMATCH`, `UNMAPPED_ID`, `REFERENCE_CONFLICT`, and `NOT_OBSERVED`.

## Provenance

Source repository: `blakinio/Oteryn-Platform`.
Source PR: `#1006`.
Source branch head audited for harvest: `97f8df9e64e1e4f0520440073e497f24dad929ef`.
The executable source files are copied from that research branch without proprietary assets. Repository-specific research workflows, credentials, screenshots and live-client control scaffolding are intentionally excluded.

## Validation

```bash
python3 -m compileall -q tools/tibia-worldmap-reconstruction
PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 -m unittest discover -s tools/tibia-worldmap-reconstruction/tests -v
PYTHONPATH=tools/tibia-worldmap-reconstruction \
  python3 tools/tibia-worldmap-reconstruction/run.py validate \
  tools/tibia-worldmap-reconstruction/examples/synthetic-capture.json
```
