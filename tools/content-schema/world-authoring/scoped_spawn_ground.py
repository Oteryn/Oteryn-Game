"""Admit the captured Crystal spawn grounds at their reviewed placement cells."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter, defaultdict
from functools import lru_cache
from pathlib import Path

import world_region_codec as codec
from convert_world_metadata import canonical

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = "imports/crystalserver/spawn-reconciliation/slice-2.json"
CRYSTAL_DELTA_CAPTURE = "imports/crystalserver/spawn-delta-201189f8.json"
COMPILER_CELLS = "tools/world-bundle-compiler/scoped-spawn-ground.json"
INDEX = "content/world/placements/index.json"
SHARD = "content/world/terrain/terrain-08794-08845.json"
MARKER = "content/world/terrain/index.json"
NOTE = " Additionally 52 source-qualified spawn grounds bound at the slice 2 cells."


def captured_grounds(root=ROOT):
    grounds = ground_cells((root / CAPTURE).read_bytes())
    delta = root / CRYSTAL_DELTA_CAPTURE
    if delta.is_file():
        # The delta's Michael the Stalwart uses the same appearance as slice 2.
        # Keep both placements scoped to specific captured cells.
        cell = (33863, 32115, 0)
        groups = json.loads(delta.read_bytes())["groups"]
        if cell in grounds or not any(
            point["name"] == "michael the stalwart" and tuple(point["cell"]) == cell
            for group in groups
            for point in group["points"]
        ):
            raise ValueError("Crystal delta ground differs from its spawn capture")
        grounds[cell] = 50815
    return grounds


def ground_cells(raw):
    points = json.loads(raw)["points"]
    grounds = {}
    for row in points:
        if "ground" in row:
            cell = tuple(row["cell"])
            if cell in grounds:
                raise ValueError(f"duplicate captured ground at {cell}")
            grounds[cell] = row["ground"]
    return grounds


def scoped_slots(palette):
    return {
        slot: row["source_item_id"]
        for slot, row in enumerate(palette)
        if row.get("scoped_spawn_ground")
    }


def plain_ground(flags, house, depth, attrs):
    return not (flags & 1 or house or depth or attrs)


@lru_cache(maxsize=1)
def ground_ids(root=ROOT):
    return frozenset(captured_grounds(root).values())


@lru_cache(maxsize=1)
def records(root=ROOT):
    sys.path.insert(
        0, str(Path(__file__).resolve().parents[1] / "world-object-authoring")
    )
    from summer_ground import ground_records

    result = {}
    for record in ground_records(ground_ids(root), root):
        record["provenance"]["route"]["reason"] = "scoped_spawn_ground"
        result[record["provenance"]["source_item_id"]] = record
    return result


def catalogue_outputs(marker_raw, root=ROOT):
    records_by_id = records(root)
    from activity_static import compose_terrain_notes

    marker = json.loads(marker_raw)
    if NOTE not in marker["notes"]:
        marker["notes"] += NOTE
    marker["notes"] = compose_terrain_notes(marker["notes"])
    return {
        SHARD: canonical(
            {"family": "Terrain", "records": list(records_by_id.values())}
        ),
        MARKER: (json.dumps(marker, ensure_ascii=False, indent=2) + "\n").encode(),
    }


def overlay_outputs(outputs, root=ROOT, *, declared=None):
    grounds = captured_grounds(root)

    def read(path):
        return outputs[path] if path in outputs else (root / path).read_bytes()

    index = json.loads(read(INDEX))
    palette = index["palette"]
    existing = scoped_slots(palette)
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
    if missing := groups.keys() - regions.keys():
        raise ValueError(f"missing spawn ground regions: {sorted(missing)}")
    replacements = Counter()
    bytes_delta = 0
    for path in sorted(regions if existing else groups):
        cells = groups.get(path, {})
        raw = read(path)
        z, rx, ry, sectors = codec.decode_region(raw)
        encoded = {}
        for local, tiles in sectors:
            sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
            for x, y, flags, house, _, placements in tiles:
                cell = (x, y, z)
                if existing:
                    for slot, depth, attrs in placements:
                        if slot in existing and (
                            grounds.get(cell) != existing[slot]
                            or not plain_ground(flags, house, depth, attrs)
                        ):
                            raise ValueError(
                                f"unapproved scoped spawn ground at {cell}"
                            )
                item = cells.pop((x, y), None)
                if item is None:
                    continue
                candidates = [
                    i
                    for i, (slot, depth, attrs) in enumerate(placements)
                    if plain_ground(flags, house, depth, attrs)
                    and palette[slot]["source_item_id"] == item
                ]
                if len(candidates) != 1:
                    raise ValueError(
                        f"spawn ground differs from capture at {(x, y, z)}"
                    )
                at = candidates[0]
                old_slot = placements[at][0]
                if old_slot != scoped[item]:
                    replacements[old_slot] += 1
                    placements[at] = (scoped[item], 0, None)
            if path in groups:
                encoded[local] = codec.encode_sector_tiles(sx, sy, tiles)
        if path not in groups:
            continue
        if cells:
            raise ValueError(f"missing spawn ground in {path}: {sorted(cells)}")
        payload = codec.encode_region(z, rx, ry, encoded)
        bytes_delta += len(payload) - len(raw)
        regions[path]["sha256"] = hashlib.sha256(payload).hexdigest()
        outputs[path] = payload
    outputs[INDEX] = canonical(index)
    if declared is not None:
        from convert_world_base import SUMMARY, key_family

        path = SUMMARY.relative_to(ROOT).as_posix()
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
    outputs[COMPILER_CELLS] = canonical(
        {
            "points": [
                {"cell": cell, "ground": ground}
                for cell, ground in sorted(captured_grounds().items())
            ]
        }
    )
    for path, raw in outputs.items():
        target = ROOT / path
        if args.check:
            if not target.is_file() or target.read_bytes() != raw:
                raise SystemExit(f"stale spawn ground: {path}")
        elif not target.is_file() or target.read_bytes() != raw:
            target.write_bytes(raw)


if __name__ == "__main__":
    main()
