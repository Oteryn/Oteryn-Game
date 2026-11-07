"""Restore only Crystal's citizenship and Svargrond gate startup stamps.

The map contains the items; startup/tables/teleport.lua assigns their ids later.
Use --apply [--check] on committed regions; the base converter calls overlay_outputs.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = "tools/content-schema/world-authoring/samples/citizen-bindings.json"
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"


def canonical(value):
    return (json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n").encode()


def capture(path):
    """Normalize the bounded registration subset from the pinned donor table."""
    sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
    import lua_tables

    text = path.read_text()
    digest = hashlib.sha256(text.encode()).hexdigest()
    if digest != "9e8f807264ec6ab85e263976136666ff38c195f1921cc30e73fd4431370fe87e":
        raise ValueError("citizen source differs from pinned Crystal 201189f8")
    tables = lua_tables.assignments(text, {"TeleportUnique", "TeleportAction"})
    bindings = []
    citizens = set(range(9056, 9069)) | {9240, 9500, 9510, 9515}
    for table, ids, attribute in [
        ("TeleportUnique", citizens, "unique"),
        ("TeleportAction", {30032}, "action"),
    ]:
        for field in tables[table]["fields"]:
            if field["key"] not in ids:
                continue
            value = lua_tables.as_python(field["value"])
            positions = value["itemPos"]
            for position in positions if isinstance(positions, list) else [positions]:
                bindings.append(
                    {
                        "position": [position[k] for k in "xyz"],
                        "item": value["itemId"],
                        attribute: field["key"],
                        "source_line": field["line"],
                    }
                )
    if len(bindings) != 18:
        raise ValueError("expected exactly 17 civic portals and the Svargrond entrance")
    return {
        "schema": 1,
        "repository": "zimbadev/crystalserver",
        "revision": "201189f871d23597b82caf88fd611454a2ddefb8",
        "source": "data-global/startup/tables/teleport.lua",
        "sha256": digest,
        "classification": "OTS_HYPOTHESIS_ONLY",
        "rules": [
            "data-global/scripts/movements/teleport/citizen.lua:1-49",
            "data-global/scripts/movements/teleport/citizen_svargrond.lua:3-22",
        ],
        "bindings": sorted(bindings, key=lambda row: row["position"]),
    }


def overlay_outputs(outputs, require_complete=False):
    capture = (ROOT / CAPTURE).read_bytes()
    document = json.loads(capture)
    index = json.loads(outputs[INDEX])
    by_region = {}
    for row in document["bindings"]:
        x, y, z = row["position"]
        path = "content/world/placements/" + codec.region_name(z, x // 256, y // 256)
        by_region.setdefault(path, {})[(x, y)] = row
    imported = {"unique": 0, "action": 0}
    byte_delta = 0
    for region in index["regions"]:
        path = region["path"]
        bindings = by_region.get(path)
        if not bindings:
            continue
        previous = outputs[path]
        z, rx, ry, sectors = codec.decode_region(previous)
        encoded = {}
        matched = set()
        changed = False
        for local, tiles in sectors:
            for x, y, _, house, _, items in tiles:
                row = bindings.get((x, y))
                if row is None:
                    continue
                attribute = "unique" if "unique" in row else "action"
                matches = [
                    (i, attrs)
                    for i, (palette, depth, attrs) in enumerate(items)
                    if depth == 0
                    and index["palette"][palette]["source_item_id"] == row["item"]
                ]
                if house or len(matches) != 1:
                    raise ValueError(f"citizen item mismatch at {(x, y, z)}")
                slot, attrs = matches[0]
                if (attrs or {}).get(attribute, row[attribute]) != row[attribute]:
                    raise ValueError(f"citizen stamp conflict at {(x, y, z)}")
                replacement = {**(attrs or {}), attribute: row[attribute]}
                changed |= replacement != attrs
                items[slot] = (items[slot][0], 0, replacement)
                matched.add((x, y))
                imported[attribute] += 1
            encoded[local] = codec.encode_sector_tiles(
                rx * 8 + local % 8, ry * 8 + local // 8, tiles
            )
        if require_complete and matched != bindings.keys():
            raise ValueError(f"missing citizen positions: {bindings.keys() - matched}")
        raw = codec.encode_region(z, rx, ry, encoded) if changed else previous
        outputs[path] = raw
        byte_delta += len(raw) - len(previous)
        region["sha256"] = hashlib.sha256(raw).hexdigest()
    if require_complete and sum(imported.values()) != len(document["bindings"]):
        raise ValueError("missing citizen region")
    before = index["source"].get("citizenship", {}).get("imported", {})
    index["source"]["citizenship"] = {
        "path": CAPTURE,
        "sha256": hashlib.sha256(capture).hexdigest(),
        "imported": imported,
    }
    outputs[INDEX] = canonical(index)
    summary = json.loads(outputs[SUMMARY])
    summary["source"] = index["source"]
    for attribute, count in imported.items():
        if count != before.get(attribute, 0):
            summary["item_attributes"][attribute] = (
                summary["item_attributes"].get(attribute, 0)
                + count
                - before.get(attribute, 0)
            )
    summary["bytes_on_disk"] += byte_delta
    outputs[SUMMARY] = canonical(summary)


def apply(check):
    index = json.loads((ROOT / INDEX).read_bytes())
    outputs = {
        path: (ROOT / path).read_bytes()
        for path in [INDEX, SUMMARY, *(r["path"] for r in index["regions"])]
    }
    overlay_outputs(outputs, require_complete=True)
    for path, data in outputs.items():
        if (ROOT / path).read_bytes() != data:
            if check:
                raise ValueError(f"CITIZEN_BINDING_DRIFT: {path}")
            (ROOT / path).write_bytes(data)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--crystal-teleports", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.crystal_teleports:
        data = (json.dumps(capture(args.crystal_teleports), indent=2) + "\n").encode()
        if args.check:
            if (ROOT / CAPTURE).read_bytes() != data:
                raise ValueError("CITIZEN_SOURCE_DRIFT")
        else:
            (ROOT / CAPTURE).write_bytes(data)
    if args.apply:
        apply(args.check)
