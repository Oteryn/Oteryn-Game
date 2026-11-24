#!/usr/bin/env python3
"""Choose nearest compiler-admitted cells for held Crystal Global NPC anchors.

Use --probe to write a trial shard. In a scratch World project with the same
terrain and catalogues, index only that NPC placement shard and run compiler
parity. Pass its report with --parity to resolve the reviewed placement scope.
"""

import argparse
import itertools
import json
from pathlib import Path
import xml.etree.ElementTree as ET

import import_crystal_npc_placements as importer

placements = importer.placements
PREFIX = "oteryn:npc_placement.probe."
AXES = ("x", "y", "z")


def coordinate(cell):
    return dict(zip(AXES, cell))


def distance(a, b):
    return max(abs(a[j] - b[j]) for j in (0, 1))


def spawn_centers(checkout):
    importer.read_sources(checkout)
    result = {}
    for group in ET.parse(checkout / importer.GLOBAL).getroot():
        x, y, z = (int(group.attrib["center" + axis]) for axis in AXES)
        for child in group:
            attrs = child.attrib
            result[attrs["name"], (x + int(attrs.get("x", 0)), y + int(attrs.get("y", 0)), z)] = (x, y, z)
    return result


def write_scope(scope):
    packet = scope.pop("crystal_global_map")
    entries = ",\n".join(f"      {json.dumps(key)}: {placements.dump(value)}"
                           for key, value in packet["npcs"].items())
    prefix = json.dumps(scope, ensure_ascii=False, indent=2).removesuffix("\n}")
    placements.SCOPE.write_text(
        prefix + ',\n  "crystal_global_map": {\n    "source": '
        + placements.dump(packet["source"]) + ',\n    "npcs": {\n'
        + entries + "\n    }\n  }\n}\n"
    )


def plan(scope, held, centers):
    reserved = importer.travel_destinations()
    for name, data in placements.convert().items():
        if name.startswith("placements-"):
            reserved.update(tuple(row["declaration"]["cell"][axis] for axis in ("x", "y", "floor"))
                            for row in json.loads(data)["records"])
    world = json.loads((placements.ROOT / "content/world/worlds/worlds-00000-00000.json").read_text())
    bounds = world["records"][0]["declaration"]["bounds"]
    for key, rows in scope["crystal_global_map"]["npcs"].items():
        for row in rows:
            anchor = placements.cell_of(row["position"])
            if (key, anchor) not in held or "adjustment" in row or "spawn_relocation" in row:
                continue
            x, y, z = center = centers[row["name"], anchor]
            radius = row["spawn_radius"]
            cells = {(cx, cy, z) for cx in range(x - radius, x + radius + 1)
                     for cy in range(y - radius, y + radius + 1)
                     if bounds["min_x"] <= cx < bounds["max_x_exclusive"]
                     and bounds["min_y"] <= cy < bounds["max_y_exclusive"]}
            wiki = row["wiki_comparison"].get("cell")
            yield key, row, anchor, center, sorted(cells - reserved, key=lambda cell: (
                distance(cell, anchor), sum((cell[i] - anchor[i]) ** 2 for i in (0, 1)),
                distance(cell, wiki) if wiki and wiki[2] == z else 0, cell,
            ))


def probe_id(cell):
    x, y, z = cell
    return f"{PREFIX}x{x}_y{y}_z{z}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkout", type=Path)
    parser.add_argument("baseline", type=Path, help="compiler parity before relocation")
    run = parser.add_mutually_exclusive_group(required=True)
    run.add_argument("--probe", type=Path)
    run.add_argument("--parity", type=Path)
    args = parser.parse_args()
    scope = json.loads(placements.SCOPE.read_text())
    baseline = json.loads(args.baseline.read_text())["npcs"]
    held = {(row["npc"], tuple(row["cell"])): row["reason"] for row in baseline["held"]}
    choices = list(plan(scope, held, spawn_centers(args.checkout)))
    cells = sorted({cell for *_, options in choices for cell in options})
    if args.probe:
        keys = iter(scope["crystal_global_map"]["npcs"])
        contacts = itertools.chain.from_iterable(itertools.repeat(key, 8) for key in keys)
        records = [{"declaration": {
            "identity": {"key": probe_id(cell), "revision": "definition-r1"},
            "npc": key, "direction": "north", "cell": dict(zip(("x", "y", "floor"), cell)),
        }} for cell, key in zip(cells, contacts, strict=False)]
        if len(records) != len(cells):
            raise ValueError("too few admitted contacts for the trial shard")
        args.probe.write_text(placements.dump({
            "coordinate_frame": placements.FRAME, "family": "Npc.Placement", "records": records,
        }) + "\n")
        return

    report = json.loads(args.parity.read_text())["npcs"]
    rejected = {row["key"] for row in report["held"]}
    if report["placements"] != len(cells) or not rejected <= {probe_id(cell) for cell in cells}:
        raise ValueError("parity must cover the complete probe shard")
    occupied = set()
    moved, remaining = [], []
    for key, row, anchor, center, options in choices:
        target = next((c for c in options if c not in occupied and probe_id(c) not in rejected), None)
        if target is None:
            reason = held[key, anchor]
            remaining.append({"npc": key, "cell": anchor, "reason": reason,
                              "detail": "no available compiler-admitted cell within the donor spawn radius"})
            continue
        occupied.add(target)
        row["spawn_relocation"] = {"center": coordinate(center), "target": coordinate(target)}
        moved.append({"npc": key, "source": anchor, "target": target})
    write_scope(scope)
    print(json.dumps({"moved": moved, "remaining": remaining}, indent=2))


if __name__ == "__main__":
    main()
