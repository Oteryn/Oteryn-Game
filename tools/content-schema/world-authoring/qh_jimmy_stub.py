"""Open the dead end south of Jimmy's Yalahar passage (Top of the City Quest).

Our B3 map closes the passage x32856..32859 at y31381 and has no tile below it,
while Tibia global continues it: tibiamaps floor 6 (map and path images) is
walkable at x32857..32858 from y31370 through y31383, and TibiaWiki and the
Polish TibiaWiki both mark Jimmy at 32857,31383,6. This overlay opens the
closing wall row and extends the passage with the existing wall and floor
appearances, then closes it one row further south. Region bytes, counts and the
capture summary are always generated here; nothing is hand-edited.
"""

import argparse
import hashlib
import json
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
REGION = "content/world/placements/region-z06-x128-y122.b3"
SOURCE = "qh_jimmy_stub"
FLOOR = 6
XS = (32856, 32857, 32858, 32859)
CLOSING = (450, 440, 440, 449)
PASSAGE = (441, 408, 408, 442)
REPLACED_ROW = 31381
ADDED_ROWS = (31382, 31383, 31384)
NEW_CLOSING_ROW = 31385
EVIDENCE = {
    "schema": "OTERYN_AUTHORED_WORLD_STUB/v1",
    "basis": "CHOSEN_OTERYN_APPROXIMATION",
    "quest": "oteryn:quest.authored.top_of_the_city_quest",
    "evidence": [
        "https://github.com/tibiamaps/tibia-map-data floor-06-map.png and "
        "floor-06-path.png (main; frame xMin 31744, yMin 30976): walkable "
        "x32857..32858 from y31370 through y31383",
        "TibiaWiki Jimmy NPC map mark 32857,31383,6 (owner browser evidence 2026-11-17)",
        "Polish TibiaWiki Jimmy map mark 32857,31383,6 (owner browser evidence 2026-11-17)",
    ],
    "replaced_row": REPLACED_ROW,
    "added_rows": list(ADDED_ROWS) + [NEW_CLOSING_ROW],
    "columns": list(XS),
    "passage": list(PASSAGE),
    "closing": list(CLOSING),
}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def layout():
    """Every tile this overlay owns after it has run, as source appearance ids."""
    tiles = {}
    for y in (REPLACED_ROW, *ADDED_ROWS):
        tiles.update({(x, y): [item] for x, item in zip(XS, PASSAGE)})
    tiles.update({(x, NEW_CLOSING_ROW): [item] for x, item in zip(XS, CLOSING)})
    return tiles


def _read(outputs, root, path):
    return outputs[path] if path in outputs else (root / path).read_bytes()


def _plain(tile):
    _x, _y, flags, house, zones, items = tile
    return (
        flags == 0
        and not house
        and not zones
        and len(items) == 1
        and items[0][1:] == (0, None)
    )


def proof(added_items):
    return {
        "basis": EVIDENCE["basis"],
        "sha256": digest(canonical(EVIDENCE)),
        "region": REGION,
        "tiles_replaced": len(XS),
        "tiles_added": len(XS) * (len(ADDED_ROWS) + 1),
        "items_added": added_items,
    }


def overlay_outputs(outputs, root=ROOT):
    index = json.loads(_read(outputs, root, INDEX))
    if SOURCE in index["source"]:
        if index["source"][SOURCE] != proof(len(XS) * (len(ADDED_ROWS) + 1)):
            raise ValueError("QH_JIMMY_STUB_EVIDENCE_CHANGED_REQUIRES_REBUILD")
        validate_overlay(outputs, root, index)
        return
    summary = json.loads(_read(outputs, root, SUMMARY))
    palette = index["palette"]
    lookup = {}
    for item in set(PASSAGE) | set(CLOSING):
        rows = [i for i, row in enumerate(palette) if row["source_item_id"] == item]
        if len(rows) != 1 or palette[rows[0]]["key"] != f"oteryn:item.tibia.i{item}":
            raise ValueError("QH_JIMMY_STUB_PALETTE_IDENTITY")
        lookup[item] = rows[0]
    original = _read(outputs, root, REGION)
    z, rx, ry, sectors = codec.decode_region(original)
    if (z, rx, ry) != (FLOOR, XS[0] // 256, REPLACED_ROW // 256):
        raise ValueError("QH_JIMMY_STUB_WRONG_REGION")
    wanted = layout()
    replaced = {(x, REPLACED_ROW): item for x, item in zip(XS, CLOSING)}
    seen, encoded, added = set(), {}, 0
    for local, tiles in sectors:
        sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
        changed = []
        for tile in tiles:
            at = tile[:2]
            if at in replaced:
                if not _plain(tile) or palette[tile[5][0][0]]["source_item_id"] != replaced[at]:
                    raise ValueError("QH_JIMMY_STUB_SOURCE_WALL_DRIFT")
                tile = (*at, 0, 0, (), [(lookup[wanted[at][0]], 0, None)])
                seen.add(at)
            elif at in wanted:
                raise ValueError("QH_JIMMY_STUB_TARGET_TILE_ALREADY_PRESENT")
            changed.append(tile)
        if (sx, sy) == (XS[0] // 32, REPLACED_ROW // 32):
            for at, items in wanted.items():
                if at not in replaced:
                    changed.append((*at, 0, 0, (), [(lookup[items[0]], 0, None)]))
                    added += 1
            changed.sort(key=lambda t: (t[1], t[0]))
        encoded[local] = codec.encode_sector_tiles(sx, sy, changed)
    if seen != set(replaced) or added != len(wanted) - len(replaced):
        raise ValueError("QH_JIMMY_STUB_SOURCE_TILE_ABSENT")
    raw = codec.encode_region(z, rx, ry, encoded)
    outputs[REGION] = raw
    (row,) = [r for r in index["regions"] if r["path"] == REGION]
    row["sha256"] = digest(raw)
    row["tiles"] += added
    row["items"] += added
    index["totals"]["tiles"] += added
    index["totals"]["items"] += added
    index["source"][SOURCE] = proof(added)
    index["source"] = dict(sorted(index["source"].items()))
    summary["source"] = index["source"]
    summary["totals"] = index["totals"].copy()
    summary["bytes_on_disk"] += len(raw) - len(original)
    floor = str(FLOOR)
    summary["tiles_by_floor"][floor] = summary["tiles_by_floor"].get(floor, 0) + added
    summary["palette"]["families"]["item"]["occurrences"] += added
    outputs[INDEX] = canonical(index)
    outputs[SUMMARY] = canonical(summary)
    validate_overlay(outputs, root, index)


def donor_delta(summary):
    """Tiles and items this overlay adds on top of the unchanged donor world."""
    declared = summary.get("source", {}).get(SOURCE)
    if declared is None:
        return {"tiles": 0, "items": 0}
    if declared != proof(declared.get("items_added")):
        raise ValueError("QH_JIMMY_STUB_EVIDENCE_DRIFT")
    return {"tiles": declared["tiles_added"], "items": declared["items_added"]}


def validate_overlay(outputs, root, index):
    found = {}
    for _, tiles in codec.decode_region(_read(outputs, root, REGION))[3]:
        for tile in tiles:
            if tile[:2] in layout():
                if not _plain(tile):
                    raise ValueError("QH_JIMMY_STUB_UNEXPECTED_ATTRIBUTES")
                found[tile[:2]] = [index["palette"][tile[5][0][0]]["source_item_id"]]
    if found != layout():
        raise ValueError("QH_JIMMY_STUB_EXACT_LAYOUT_DRIFT")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    outputs = {}
    overlay_outputs(outputs, args.root)
    for path, raw in outputs.items():
        target = args.root / path
        if args.check:
            if not target.exists() or target.read_bytes() != raw:
                raise SystemExit("QH_JIMMY_STUB_STALE_AUTHORED_WORLD:" + path)
        else:
            target.write_bytes(raw)
