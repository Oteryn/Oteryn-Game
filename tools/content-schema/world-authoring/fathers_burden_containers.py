"""Restore the six shared Father's Burden map container contents from Crystal."""

import argparse
import hashlib
import json
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
SOURCE = "fathers_burden_containers"
# Crystal 201189f8 data-global/startup/tables/container_quest.lua.
MATERIALS = {
    (31987, 31223, 7): (2469, 11547, 283),
    (32815, 32502, 8): (2469, 11551, 401),
    (33215, 31413, 10): (11591, 11545, 498),
    (33235, 32803, 12): (1994, 11552, 568),
    (32122, 31912, 14): (1983, 11546, 584),
    (32636, 31985, 14): (2469, 11549, 600),
}
PROOF = {
    "path": "data-global/startup/tables/container_quest.lua",
    "revision": "201189f871d23597b82caf88fd611454a2ddefb8",
    "sha256": "c18f0caf3d2c9f1d0c084722dfcb55523dbb1811166863a4f72d052bd2ae24b9",
    "items_added": len(MATERIALS),
}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def overlay_outputs(outputs):
    index = json.loads(outputs[INDEX])
    summary = json.loads(outputs[SUMMARY])
    regions = {row["path"]: row for row in index["regions"]}
    palette = index["palette"]
    slots = {row["source_item_id"]: i for i, row in enumerate(palette)}
    installed = index["source"].get(SOURCE)
    if installed is not None and installed != PROOF:
        raise ValueError("Father's Burden container source proof drift")
    for (x, y, z), (container, material, line) in MATERIALS.items():
        path = f"content/world/placements/{codec.region_name(z, x // 256, y // 256)}"
        raw = outputs[path]
        _, rx, ry, sectors = codec.parse_region(raw)
        local = (y // 32 % 8) * 8 + x // 32 % 8
        payloads = {
            sector: codec.zstandard.ZstdDecompressor().decompress(
                raw[offset : offset + length], max_output_size=codec.MAX_SECTOR_BYTES
            )
            for sector, offset, length in sectors
        }
        tiles = codec.decode_sector(payloads[local], x // 32, y // 32)
        (tile,) = [tile for tile in tiles if tile[:2] == (x, y)]
        if tile[3]:
            raise ValueError(
                f"Quest container unexpectedly inside a house: {(x, y, z)}"
            )
        items = tile[5]
        (parent,) = [
            i
            for i, (slot, depth, _) in enumerate(items)
            if depth == 0 and palette[slot]["source_item_id"] == container
        ]
        child = {
            "key": f"oteryn:item.tibia.i{material}",
            "provisional": False,
            "source_item_id": material,
        }
        if material not in slots:
            slots[material] = len(palette)
            palette.append(child)
            summary["palette"]["entries"] += 1
            summary["palette"]["families"]["item"]["entries"] += 1
        if not installed and palette[slots[material]] == {**child, "retired": True}:
            # A full recapture initially sees no startup-only container contents.
            palette[slots[material]] = child
        if palette[slots[material]] != child:
            raise ValueError(f"Unexpected palette binding for material {material}")
        start = parent + 1
        end = next(
            (i for i in range(start, len(items)) if items[i][1] == 0), len(items)
        )
        value = (slots[material], 1, None)
        if installed:
            if items[start:end] != [value]:
                raise ValueError(f"Crystal container_quest.lua:{line} contents drift")
            continue
        if start != end:
            raise ValueError(
                f"Crystal container_quest.lua:{line} source already has contents"
            )
        items.insert(start, value)
        payloads[local] = codec.encode_sector_tiles(x // 32, y // 32, tiles)
        updated = codec.encode_region(z, rx, ry, payloads)
        outputs[path] = updated
        regions[path]["sha256"] = hashlib.sha256(updated).hexdigest()
        regions[path]["items"] += 1
        index["totals"]["items"] += 1
        summary["bytes_on_disk"] += len(updated) - len(raw)
        summary["palette"]["families"]["item"]["occurrences"] += 1
    index["source"][SOURCE] = PROOF
    summary["source"] = index["source"]
    summary["totals"] = index["totals"].copy()
    outputs[INDEX] = canonical(index)
    outputs[SUMMARY] = canonical(summary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    paths = [
        INDEX,
        SUMMARY,
        *{
            f"content/world/placements/{codec.region_name(z, x // 256, y // 256)}"
            for x, y, z in MATERIALS
        },
    ]
    previous = {path: (ROOT / path).read_bytes() for path in paths}
    outputs = previous.copy()
    overlay_outputs(outputs)
    for path in paths:
        if outputs[path] != previous[path]:
            if args.check:
                raise ValueError(f"Father's Burden container overlay drift: {path}")
            (ROOT / path).write_bytes(outputs[path])


if __name__ == "__main__":
    main()
