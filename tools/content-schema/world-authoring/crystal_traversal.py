#!/usr/bin/env python3
"""Apply the four public-world teleport corrections from Crystal 201189f8."""

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

import convert_world_metadata as metadata
import world_region_codec as codec

ROOT = metadata.ROOT
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
OTBM_SHA256 = "c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85"
INDEX = "content/world/placements/index.json"
SOURCE = "crystal_traversal"
METADATA_CAPTURE = "tools/content-schema/world-authoring/samples/source-capture-v1.json"
BASE_CAPTURE = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
TRANSITIONS = "content/world/transitions/index.json"
FAMILY = "Transition.Teleport"
# (source, appearance, previous destination, donor destination, donor ground).
TELEPORTS = (
    ((32867, 32826, 1), 775, None, (32866, 32831, 7), 556),
    ((33006, 32396, 15), 34111, (33901, 31881, 15), (34106, 32052, 13), 42852),
    ((33044, 32409, 15), 34111, (33079, 32395, 15), (34106, 32052, 13), 42852),
    ((33965, 31979, 9), 34111, None, (33175, 31764, 6), 486),
)


def transition(at, item, destination):
    x, y, z = at
    key = f"oteryn:transition.teleport.x{x}_y{y}_z{z}"
    binding = metadata.binding(
        "Transition", key, metadata.MAP_NAMESPACE, f"{x},{y},{z}:{item}"
    )
    binding["source_revision"] = REVISION
    return {
        "declaration": {
            "from": metadata.position(*at),
            "identity": {"key": key, "revision": metadata.REVISION},
            "kind": "Transition",
            "object": metadata.ref("Item", f"oteryn:item.tibia.i{item}"),
            "to": metadata.position(*destination),
            "transition_kind": "teleport",
        },
        "source_bindings": [binding],
    }


def donor_transitions():
    return {
        row["declaration"]["identity"]["key"]: row
        for row in (transition(at, item, to) for at, item, _, to, _ in TELEPORTS)
    }


def proof():
    return {
        "revision": REVISION,
        "path": metadata.OTBM,
        "sha256": OTBM_SHA256,
        "items_added": 1,
    }


def donor_delta(summary):
    source = summary.get("source", {}).get(SOURCE)
    if source is None:
        return 0
    if source != proof():
        raise ValueError("Crystal traversal donor proof differs")
    return source["items_added"]


def patch_tile(items, palette, slot, previous, to, ground):
    identifier = palette[slot]["source_item_id"]
    if not items or palette[items[0][0]]["source_item_id"] != ground:
        raise ValueError(f"teleport {identifier} source ground differs")
    matches = [
        i for i, (index, depth, _) in enumerate(items) if index == slot and depth == 0
    ]
    if not matches and identifier == 775:
        return [*items, (slot, 0, {"teleport": to})], 1
    if len(matches) != 1:
        raise ValueError(f"teleport {identifier} source object differs")
    ordinal = matches[0]
    attrs = items[ordinal][2] or {}
    if attrs.get("teleport") not in (previous, to):
        raise ValueError(f"teleport {identifier} previous destination differs")
    items[ordinal] = (slot, 0, attrs | {"teleport": to})
    return items, 0


def placement_outputs(root, outputs):
    def read(path):
        return outputs[path] if path in outputs else (root / path).read_bytes()

    index = json.loads(read(INDEX))
    summary = json.loads(read(BASE_CAPTURE))
    palette = index["palette"]
    slots = {row["source_item_id"]: at for at, row in enumerate(palette)}
    regions = defaultdict(dict)
    for (x, y, z), item, previous, to, ground in TELEPORTS:
        if palette[slots[item]]["key"] != f"oteryn:item.tibia.i{item}":
            raise ValueError(f"teleport Item identity differs: {item}")
        regions[codec.region_name(z, x // 256, y // 256)][x, y] = (
            slots[item],
            previous,
            to,
            ground,
        )
    applied = attributes = disk_delta = 0
    for row in index["regions"]:
        name = Path(row["path"]).name
        if name not in regions:
            continue
        pending = regions.pop(name)
        raw = read(row["path"])
        if hashlib.sha256(raw).hexdigest() != row["sha256"]:
            raise ValueError(f"teleport region digest differs: {name}")
        z, rx, ry, sectors = codec.decode_region(raw)
        encoded = {}
        added = 0
        for local, tiles in sectors:
            for i, (x, y, flags, house, zones, items) in enumerate(tiles):
                edit = pending.pop((x, y), None)
                if edit is not None:
                    before = sum("teleport" in (attrs or {}) for _, _, attrs in items)
                    items, delta = patch_tile(items, palette, *edit)
                    attributes += (
                        sum("teleport" in (attrs or {}) for _, _, attrs in items)
                        - before
                    )
                    tiles[i] = (x, y, flags, house, zones, items)
                    added += delta
            sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
            encoded[local] = codec.encode_sector_tiles(sx, sy, tiles)
        if pending:
            raise ValueError(f"teleport source tile absent: {sorted(pending)}")
        new = codec.encode_region(z, rx, ry, encoded)
        row["sha256"] = hashlib.sha256(new).hexdigest()
        row["items"] += added
        applied += added
        disk_delta += len(new) - len(raw)
        outputs[row["path"]] = new
    if regions or applied != (0 if SOURCE in index["source"] else 1):
        raise ValueError("Crystal traversal placement scope differs")
    index["totals"]["items"] += applied
    index["source"][SOURCE] = proof()
    outputs[INDEX] = metadata.canonical(index)
    summary["source"][SOURCE] = proof()
    summary["totals"]["items"] += applied
    summary["palette"]["families"]["item"]["occurrences"] += applied
    summary["item_attributes"]["teleport"] += attributes
    summary["bytes_on_disk"] += disk_delta
    outputs[BASE_CAPTURE] = metadata.canonical(summary)


def patch_transitions(rows):
    records = {row["declaration"]["identity"]["key"]: row for row in rows}
    replacements = donor_transitions()
    for source, item, previous, destination, _ in TELEPORTS:
        replacement = transition(source, item, destination)
        key = replacement["declaration"]["identity"]["key"]
        original = records.get(key)
        if original != replacement:
            expected = (
                None
                if previous is None
                else transition(source, item, previous)["declaration"]
            )
            if (original and original["declaration"]) != expected:
                raise ValueError(f"{key}: previous transition differs")
    records.update(replacements)
    return metadata.unique(list(records.values()), FAMILY)


def transition_capture(capture, records):
    capture["families"][FAMILY] = len(records)
    counts = Counter(row["declaration"]["object"]["family"] for row in records)
    capture["teleport_object_families"] = {
        key: counts[key] for key in ("Item", "Terrain", "WorldObject")
    }


def transition_outputs(root, outputs):
    index = json.loads((root / TRANSITIONS).read_bytes())
    old_paths = set(index["shards"])
    records = patch_transitions(
        row
        for path in old_paths
        for row in json.loads((root / path).read_bytes())["records"]
    )
    outputs.update(metadata.shard_files(FAMILY, records, source=index["source"]))
    capture = json.loads((root / METADATA_CAPTURE).read_bytes())
    transition_capture(capture, records)
    outputs[METADATA_CAPTURE] = metadata.canonical(capture)
    return old_paths - outputs.keys()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    outputs = {}
    placement_outputs(ROOT, outputs)
    stale = transition_outputs(ROOT, outputs)
    for path, data in outputs.items():
        path = ROOT / path
        if path.is_file() and path.read_bytes() == data:
            continue
        if args.check:
            raise SystemExit(f"Crystal traversal output differs: {path}")
        path.write_bytes(data)
    for path in stale:
        if args.check:
            raise SystemExit(f"obsolete Crystal traversal transition shard: {path}")
        (ROOT / path).unlink()


if __name__ == "__main__":
    main()
