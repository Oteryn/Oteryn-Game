"""Bind the source-backed citizenship portals to the existing World tiles."""

import argparse
import hashlib
import json
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
SOURCE = "content/world/provenance/citizenship-portals.json"


def canonical(document):
    return (json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n").encode()


def overlay_outputs(outputs):
    capture = (ROOT / SOURCE).read_bytes()
    evidence = json.loads(capture)
    index = json.loads(outputs[INDEX])
    summary = json.loads(outputs[SUMMARY])
    (portal,) = [
        i
        for i, item in enumerate(index["palette"])
        if item["key"] == evidence["appearance"]
    ]
    wanted = {tuple(row["position"]): row["unique"] for row in evidence["placements"]}
    if len(wanted) != len(evidence["placements"]) or len(set(wanted.values())) != len(
        wanted
    ):
        raise ValueError("duplicate citizenship tile or unique ID")

    total_added_items = 0
    for region in index["regions"]:
        path = region["path"]
        if not any(
            codec.region_name(z, x // 256, y // 256) == Path(path).name
            for x, y, z in wanted
        ):
            continue
        old = outputs[path]
        z, rx, ry, sectors = codec.decode_region(old)
        added_items = added_uniques = 0
        encoded = {}
        for local, tiles in sectors:
            for x, y, _, house, _, items in tiles:
                uid = wanted.pop((x, y, z), None)
                if uid is None:
                    continue
                if house or not items:
                    raise ValueError(f"invalid citizenship tile {(x, y, z)}")
                slots = [
                    i
                    for i, (item, depth, _) in enumerate(items)
                    if item == portal and depth == 0
                ]
                if not slots:
                    slots = [len(items)]
                    items.append((portal, 0, None))
                    added_items += 1
                (slot,) = slots
                attrs = items[slot][2] or {}
                if attrs.get("unique", uid) != uid:
                    raise ValueError(f"citizenship UID conflict at {(x, y, z)}")
                if "unique" not in attrs:
                    items[slot] = (portal, 0, {**attrs, "unique": uid})
                    added_uniques += 1
            sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
            encoded[local] = codec.encode_sector_tiles(sx, sy, tiles)
        if added_uniques:
            raw = codec.encode_region(z, rx, ry, encoded)
            outputs[path] = raw
            region["sha256"] = hashlib.sha256(raw).hexdigest()
            total_added_items += added_items
            region["items"] += added_items
            index["totals"]["items"] += added_items
            summary["bytes_on_disk"] += len(raw) - len(old)
            summary["item_attributes"]["unique"] += added_uniques
    if wanted:
        raise ValueError(f"missing citizenship tiles: {wanted}")
    previous_added_items = (
        index["source"].get("citizenship_portals", {}).get("items_added", 0)
    )
    index["source"]["citizenship_portals"] = {
        "items_added": previous_added_items + total_added_items,
        "path": SOURCE,
        "sha256": hashlib.sha256(capture).hexdigest(),
    }
    summary["palette"]["families"]["item"]["occurrences"] += total_added_items
    summary["source"] = index["source"]
    summary["totals"] = index["totals"]
    outputs[INDEX] = canonical(index)
    outputs[SUMMARY] = canonical(summary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    evidence = json.loads((ROOT / SOURCE).read_bytes())
    regions = {
        codec.region_name(z, x // 256, y // 256)
        for x, y, z in (row["position"] for row in evidence["placements"])
    }
    index = json.loads((ROOT / INDEX).read_bytes())
    paths = [
        INDEX,
        SUMMARY,
        *(row["path"] for row in index["regions"] if Path(row["path"]).name in regions),
    ]
    before = {path: (ROOT / path).read_bytes() for path in paths}
    after = before.copy()
    overlay_outputs(after)
    for path, data in after.items():
        if data != before[path]:
            if args.check:
                raise SystemExit(f"citizenship placement drift: {path}")
            (ROOT / path).write_bytes(data)


if __name__ == "__main__":
    main()
