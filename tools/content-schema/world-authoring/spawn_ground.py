"""Admit the captured Crystal spawn grounds at their reviewed placement cells."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

import world_region_codec as codec
from convert_world_metadata import canonical

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = "imports/crystalserver/spawn-reconciliation/slice-2.json"
INDEX = "content/world/placements/index.json"
SHARD = "content/world/terrain/terrain-08751-08802.json"
MARKER = "content/world/terrain/index.json"
NOTE = " Additionally 52 source-qualified spawn grounds bound at the slice 2 cells."


def captured_grounds(root=ROOT):
    points = json.loads((root / CAPTURE).read_bytes())["points"]
    return {tuple(row["cell"]): row["ground"] for row in points if "ground" in row}


def catalogue_outputs(marker_raw, root=ROOT):
    identifiers = set(captured_grounds(root).values())
    sys.path.insert(
        0, str(Path(__file__).resolve().parents[1] / "world-object-authoring")
    )
    from activity_static import compose_terrain_notes
    from summer_ground import ground_records

    records = ground_records(identifiers, root)
    for record in records:
        record["provenance"]["route"]["reason"] = "scoped_spawn_ground"
    marker = json.loads(marker_raw)
    if NOTE not in marker["notes"]:
        marker["notes"] += NOTE
    marker["notes"] = compose_terrain_notes(marker["notes"])
    return {
        SHARD: canonical({"family": "Terrain", "records": records}),
        MARKER: (json.dumps(marker, ensure_ascii=False, indent=2) + "\n").encode(),
    }


def overlay_outputs(outputs, root=ROOT, *, declared=None):
    grounds = captured_grounds(root)

    def read(path):
        return outputs[path] if path in outputs else (root / path).read_bytes()

    index = json.loads(read(INDEX))
    palette = index["palette"]
    by_key = {row["key"]: i for i, row in enumerate(palette)}
    scoped = {}
    for item in sorted(set(grounds.values())):
        key = f"oteryn:terrain.tibia.i{item}"
        if key not in by_key:
            by_key[key] = len(palette)
            palette.append(
                {
                    "key": key,
                    "provisional": False,
                    "source_item_id": item,
                    "scoped_spawn_ground": True,
                }
            )
        slot = by_key[key]
        if palette[slot].get("scoped_spawn_ground") is not True:
            raise ValueError(
                f"spawn ground {item} already has an unscoped Terrain entry"
            )
        scoped[item] = slot

    groups = defaultdict(dict)
    for (x, y, z), item in grounds.items():
        name = codec.region_name(z, x // 256, y // 256)
        groups[f"content/world/placements/{name}"][x, y] = item
    regions = {row["path"]: row for row in index["regions"]}
    replacements = Counter()
    bytes_delta = 0
    for path, cells in sorted(groups.items()):
        raw = read(path)
        z, rx, ry, sectors = codec.decode_region(raw)
        encoded = {}
        for local, tiles in sectors:
            sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
            for x, y, flags, house, _, placements in tiles:
                item = cells.pop((x, y), None)
                if item is None:
                    continue
                candidates = [
                    i
                    for i, (slot, depth, attrs) in enumerate(placements)
                    if not depth
                    and not attrs
                    and palette[slot]["source_item_id"] == item
                ]
                if len(candidates) != 1 or flags & 1 or house:
                    raise ValueError(
                        f"spawn ground differs from capture at {(x, y, z)}"
                    )
                at = candidates[0]
                old_slot = placements[at][0]
                if old_slot != scoped[item]:
                    replacements[old_slot] += 1
                    placements[at] = (scoped[item], 0, None)
            encoded[local] = codec.encode_sector_tiles(sx, sy, tiles)
        if cells:
            raise ValueError(f"missing spawn ground in {path}: {sorted(cells)}")
        payload = codec.encode_region(z, rx, ry, encoded)
        bytes_delta += len(payload) - len(raw)
        regions[path]["sha256"] = hashlib.sha256(payload).hexdigest()
        outputs[path] = payload
    outputs[INDEX] = canonical(index)
    if declared is not None:
        from convert_world_base import SUMMARY, key_family

        path = SUMMARY.relative_to(root).as_posix()
        summary = json.loads(outputs[path])
        counts = summary["palette"]
        counts["entries"] = len(palette)
        counts["families"]["terrain"]["entries"] += len(scoped)
        counts["families"]["terrain"]["occurrences"] += sum(replacements.values())
        for slot, count in replacements.items():
            row = palette[slot]
            if row["provisional"]:
                kind = (
                    "in_items_xml"
                    if row["source_item_id"] in declared
                    else "appearance_only"
                )
                counts["provisional"][kind]["occurrences"] -= count
                counts["provisional"]["occurrences"] -= count
            else:
                family = key_family(row["key"], False)
                counts["families"][family]["occurrences"] -= count
        summary["bytes_on_disk"] += bytes_delta
        outputs[path] = canonical(summary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    outputs = {}
    overlay_outputs(outputs)
    outputs.update(catalogue_outputs((ROOT / MARKER).read_bytes()))
    for path, raw in outputs.items():
        target = ROOT / path
        if args.check:
            if not target.is_file() or target.read_bytes() != raw:
                raise SystemExit(f"stale spawn ground: {path}")
        elif not target.is_file() or target.read_bytes() != raw:
            target.write_bytes(raw)


if __name__ == "__main__":
    main()
