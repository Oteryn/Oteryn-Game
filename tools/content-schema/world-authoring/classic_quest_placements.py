"""Apply the startup and route placements selected from Crystal 201189f8."""

import argparse
import hashlib
import json
from collections import Counter, defaultdict

from quest_chest_bindings import INDEX, ROOT, SUMMARY, canonical, codec

CAPTURE = "tools/content-schema/world-authoring/samples/classic-quest-placements.json"


def region_path(position):
    x, y, z = position
    return (
        f"content/world/placements/region-z{z:02d}-x{x // 256:03d}-y{y // 256:03d}.b3"
    )


def overlay_region(raw, palette, rows):
    z, rx, ry, table = codec.parse_region(raw)
    selected = defaultdict(list)
    for row in rows:
        selected[tuple(row["position"])].append(row)
    decompressor = codec.zstandard.ZstdDecompressor()
    new_attributes = Counter()
    sectors, found, added = {}, set(), 0
    for local, offset, length in table:
        payload = decompressor.decompress(
            raw[offset : offset + length], max_output_size=codec.MAX_SECTOR_BYTES
        )
        sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
        tiles = codec.decode_sector(payload, sx, sy)
        changed = False
        for x, y, _, house, _, items in tiles:
            position = (x, y, z)
            for row in selected.get(position, ()):
                appearance = row["appearance"]
                matches = [
                    slot
                    for slot, (index, depth, _) in enumerate(items)
                    if depth == 0 and palette[index]["source_item_id"] == appearance
                ]
                if house or len(matches) > 1:
                    raise ValueError(f"ambiguous classic quest placement: {row}")
                attributes = row.get("attributes", {})
                attributes = {
                    key: tuple(value) if key == "teleport" else value
                    for key, value in attributes.items()
                }
                if not matches:
                    if not row.get("insert"):
                        raise ValueError(f"missing classic quest placement: {row}")
                    (index,) = (
                        i
                        for i, item in enumerate(palette)
                        if item["source_item_id"] == appearance
                    )
                    items.append((index, 0, attributes or None))
                    changed = True
                    added += 1
                    new_attributes.update(attributes.keys())
                else:
                    slot = matches[0]
                    index, depth, attrs = items[slot]
                    attrs = attrs or {}
                    if any(
                        attrs.get(key, value) != value
                        for key, value in attributes.items()
                    ):
                        raise ValueError(f"conflicting classic quest placement: {row}")
                    if not attributes.items() <= attrs.items():
                        new_attributes.update(attributes.keys() - attrs.keys())
                        items[slot] = (index, depth, {**attrs, **attributes})
                        changed = True
                found.add(position)
        sectors[local] = (
            codec.encode_sector_tiles(sx, sy, tiles) if changed else payload
        )
    if found != selected.keys():
        raise ValueError(f"missing classic quest tiles: {selected.keys() - found}")
    return codec.encode_region(z, rx, ry, sectors), added, new_attributes


def overlay_outputs(outputs, root=ROOT):
    capture = (root / CAPTURE).read_bytes()
    groups = defaultdict(list)
    for row in json.loads(capture)["placements"]:
        groups[region_path(row["position"])].append(row)
    index = json.loads(outputs[INDEX])
    summary = json.loads(outputs[SUMMARY])
    previous = index["source"].get("classic_quests", {})
    attrs = Counter(
        name
        for rows in groups.values()
        for row in rows
        for name in row.get("attributes", {})
    )
    added, delta = 0, 0
    for region in index["regions"]:
        path = region["path"]
        if path not in groups:
            continue
        baseline = outputs[path]
        outputs[path], count, new_attributes = overlay_region(
            baseline, index["palette"], groups.pop(path)
        )
        for name, value in new_attributes.items():
            counts = summary["item_attributes"]
            counts[name] = counts.get(name, 0) + value
        added += count
        region["items"] += count
        delta += len(outputs[path]) - len(baseline)
        region["sha256"] = hashlib.sha256(outputs[path]).hexdigest()
    if groups:
        raise ValueError(f"missing classic quest regions: {groups.keys()}")
    index["source"]["classic_quests"] = {
        "path": CAPTURE,
        "sha256": hashlib.sha256(capture).hexdigest(),
        "attributes": dict(attrs),
        "items_added": previous.get("items_added", 0) + added,
    }
    summary["source"] = index["source"]
    summary["palette"]["families"]["item"]["occurrences"] += added
    summary["bytes_on_disk"] += delta
    index["totals"]["items"] += added
    summary["totals"]["items"] += added
    outputs[INDEX] = canonical(index)
    outputs[SUMMARY] = canonical(summary)


def donor_delta(summary):
    """Remove only the source-qualified classic quest insertions from the donor totals."""
    registered = summary.get("source", {}).get("classic_quests")
    if registered is None:
        return 0
    capture = (ROOT / CAPTURE).read_bytes()
    rows = json.loads(capture)["placements"]
    expected = {
        "path": CAPTURE,
        "sha256": hashlib.sha256(capture).hexdigest(),
        "attributes": dict(
            Counter(key for row in rows for key in row.get("attributes", {}))
        ),
        "items_added": sum(row.get("insert", False) for row in rows),
    }
    if registered != expected:
        raise ValueError("CLASSIC_QUEST_PLACEMENT_SOURCE_CHANGED")
    return expected["items_added"]


def apply(check):
    paths = {INDEX, SUMMARY} | {
        region_path(row["position"])
        for row in json.loads((ROOT / CAPTURE).read_bytes())["placements"]
    }
    outputs = {path: (ROOT / path).read_bytes() for path in paths}
    overlay_outputs(outputs)
    for path, data in outputs.items():
        target = ROOT / path
        if check:
            if target.read_bytes() != data:
                raise ValueError(f"classic quest placement drift: {path}")
        else:
            target.write_bytes(data)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    apply(parser.parse_args().check)
