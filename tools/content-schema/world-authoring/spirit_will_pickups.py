"""Restore the three Spirit quest objects at TibiaWiki's mapper coordinates."""

import argparse
import hashlib
import json

import quest_access_bindings as world
import world_region_codec as codec

SOURCE = "spirit_will_pickups"
# The retained Crystal map has the surfaces but omits the three quest objects.
# TibiaWiki, The Spirit Will Get You Quest/Spoiler, 2022-10-19T13:39:34Z.
PICKUPS = (
    (33177, 31021, 5, (9929, 6373), 2473),
    (33161, 31056, 4, (10534, 8696), 2469),
    (33166, 31056, 6, (9856,), 2472),
)


def overlay_outputs(outputs, root=world.ROOT):
    def read(path):
        return outputs[path] if path in outputs else (root / path).read_bytes()

    index = json.loads(read(world.INDEX))
    summary = json.loads(read(world.SUMMARY))
    previous = index["source"].get(SOURCE)
    evidence = {
        "source": "TibiaWiki: The Spirit Will Get You Quest/Spoiler (2022-10-19)",
        "items_added": len(PICKUPS),
        "positions": [[x, y, z, item] for x, y, z, _, item in PICKUPS],
    }
    if previous is not None and previous != evidence:
        raise ValueError("Spirit pickup evidence drift")

    indices = {entry["source_item_id"]: i for i, entry in enumerate(index["palette"])}
    regions = {region["path"]: region for region in index["regions"]}
    for x, y, z, base, item in PICKUPS:
        path = "content/world/placements/" + codec.region_name(z, x // 256, y // 256)
        original = read(path)
        floor, rx, ry, sectors = codec.decode_region(original)
        if (floor, rx, ry) != (z, x // 256, y // 256):
            raise ValueError(f"Spirit pickup region drift: {path}")
        sector = (y // 32 % 8) * 8 + x // 32 % 8
        tiles = dict(sectors)[sector]
        ((_, _, _, house, _, entries),) = [tile for tile in tiles if tile[:2] == (x, y)]
        expected = base + ((item,) if previous else ())
        if house or entries != [
            (indices[appearance], 0, None) for appearance in expected
        ]:
            raise ValueError(f"Spirit pickup stack drift at {(x, y, z)}")
        if previous:
            continue
        entries.append((indices[item], 0, None))
        raw = codec.encode_region(
            z,
            rx,
            ry,
            {
                local: codec.encode_sector_tiles(
                    rx * 8 + local % 8, ry * 8 + local // 8, tiles
                )
                for local, tiles in sectors
            },
        )
        outputs[path] = raw
        regions[path]["sha256"] = hashlib.sha256(raw).hexdigest()
        regions[path]["items"] += 1
        summary["bytes_on_disk"] += len(raw) - len(original)

    if previous is None:
        index["source"][SOURCE] = evidence
        index["totals"]["items"] += len(PICKUPS)
        summary["source"] = index["source"]
        summary["totals"] = index["totals"].copy()
        summary["palette"]["families"]["item"]["occurrences"] += len(PICKUPS)
        outputs[world.INDEX] = world.canonical(index)
        outputs[world.SUMMARY] = world.canonical(summary)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    outputs = {}
    overlay_outputs(outputs)
    if args.check and outputs:
        raise SystemExit("Spirit quest pickup placements are stale")
    for path, raw in outputs.items():
        (world.ROOT / path).write_bytes(raw)
