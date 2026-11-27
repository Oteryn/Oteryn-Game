"""Restore the Global Blue Valley return sphere on Crystal's authored tile."""

import argparse
import hashlib
import json

import world_region_codec as codec
from quest_door_bindings import ROOT, canonical

INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
SELECTORS = (
    "tools/content-schema/quest-authoring/batch3_a/world/the_way_of_the_monk.json"
)
RETURN = "oteryn:quest-callback/batch3/the_way_of_the_monk/blue-valley-2"
# Crystal 201189f8 data-global/world/world.otbm uses this placeholder without
# placement attributes. The Global object uses the same scripted return tile.
DONOR_ITEM = 49948


def return_placement(root):
    callbacks = json.loads((root / SELECTORS).read_bytes())["callbacks"]
    (callback,) = [row for row in callbacks if row["key"] == RETURN]
    selector = callback["selector"]
    position = selector["position"]
    x, y, z = position["x"], position["y"], -position["floor"]
    path = f"content/world/placements/{codec.region_name(z, x // 256, y // 256)}"
    return path, (x, y, z), selector["appearance"], callback["sources"]


def overlay_outputs(outputs, root=ROOT):
    path, (x, y, z), item, sources = return_placement(root)
    if path not in outputs:
        raise ValueError(f"Guarded Crystal Sphere region missing: {path}")
    index = json.loads(outputs[INDEX])
    summary = json.loads(outputs[SUMMARY])
    palette = index["palette"]
    item_key = f"oteryn:item.tibia.i{item}"
    entries = [i for i, row in enumerate(palette) if row["source_item_id"] == item]
    if not entries:
        entries.append(len(palette))
        palette.append({"key": item_key, "provisional": False, "source_item_id": item})
        summary["palette"]["entries"] += 1
        summary["palette"]["families"]["item"]["entries"] += 1
    (replacement,) = entries
    if palette[replacement]["key"] != item_key or palette[replacement]["provisional"]:
        raise ValueError(f"Guarded Crystal Sphere has an unexpected palette route: {item_key}")
    # A donor recapture retains this palette slot, initially marked unused.
    palette[replacement].pop("retired", None)

    original = outputs[path]
    z, rx, ry, sectors = codec.decode_region(original)
    (items,) = [
        tile[5] for _, tiles in sectors for tile in tiles if tile[:2] == (x, y)
    ]
    (slot,) = [
        i for i, (entry, depth, attrs) in enumerate(items)
        if depth == 0 and attrs is None and palette[entry]["source_item_id"] in (DONOR_ITEM, item)
    ]
    if items[slot][0] != replacement:
        items[slot] = (replacement, 0, None)
        raw = codec.encode_region(z, rx, ry, {
            local: codec.encode_sector_tiles(rx * 8 + local % 8, ry * 8 + local // 8, tiles)
            for local, tiles in sectors
        })
        outputs[path] = raw
        (region,) = [row for row in index["regions"] if row["path"] == path]
        region["sha256"] = hashlib.sha256(raw).hexdigest()
        summary["bytes_on_disk"] += len(raw) - len(original)
    index["source"]["guarded_crystal_sphere"] = {
        "position": [x, y, z],
        "item": item,
        "replaces": DONOR_ITEM,
        "sources": sources,
    }
    summary["source"] = index["source"]
    outputs[INDEX] = canonical(index)
    outputs[SUMMARY] = canonical(summary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    path, _, _, _ = return_placement(ROOT)
    outputs = {name: (ROOT / name).read_bytes() for name in (INDEX, SUMMARY, path)}
    overlay_outputs(outputs)
    for name, raw in outputs.items():
        target = ROOT / name
        if args.check:
            if target.read_bytes() != raw:
                raise ValueError(f"Guarded Crystal Sphere placement drift: {name}")
        else:
            target.write_bytes(raw)


if __name__ == "__main__":
    main()
