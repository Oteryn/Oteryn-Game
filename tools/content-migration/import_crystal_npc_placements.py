#!/usr/bin/env python3
"""Audit pinned Crystal NPC XMLs and add missing Global map contacts to the placement scope.

Run with an extracted Crystal checkout containing the four pinned XMLs. The audit goes
to stdout. Pass --write to update the scope, then run the placement and pin generators.
"""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
import sys

AUTHORING = Path(__file__).resolve().parents[1] / "content-schema/npc-authoring"
sys.path.insert(0, str(AUTHORING))
from convert import read_placement_xml
import convert_placements as placements
from tibiawiki_br_crosscheck import FACTS, br_index, normalize

REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
GLOBAL = "data-global/world/world-npc.xml"
WINTERLIGHT = "data-global/world/annual_events/winterlight_solstice/island-npc.xml"
SOURCES = {
    GLOBAL: "db07b021856008bce61efe816e8a8eea00a01cb9750f1be5e750093758d1c029",
    "data-crystal/world/world-npc.xml": "4e9c3ce7e21b55e78082ec2502b6c187f5e4b890e36d7c7cb592edfa0f27594e",
    "data-global/world/custom/global-custom-npc.xml": "3aaf7740399fc615970d5d0c5f9f4a6eaf8fdc74ad7f455f0bfcfb030c1ea8bd",
    WINTERLIGHT: "2335f7e2097ff12ede5e10d14efc9bbd34ade9be7d4178b2e13362e85b308565",
}
ALIASES = {
    "Death": "death_npc",
    "Skullfrost": "skullfrost_npc",
    "Wyrdin's Apprentice": "wyrdins_apprentice",
}


def npc_key(name):
    slug = re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")
    return "oteryn:npc." + ALIASES.get(name, slug)


def wiki_comparison(page, cell):
    if not page:
        return {"status": "NO_RETAINED_WIKI_PAGE"}
    _, _, floor = cell
    point = min(page["positions"], default=None, key=lambda p: (
        p[2] != floor,
        max(abs(p[axis] - cell[axis]) for axis in (0, 1)),
        abs(p[2] - floor),
    ))
    if point is None:
        status = "NO_RETAINED_WIKI_COORDINATE"
    elif tuple(point) == cell:
        status = "MATCH"
    else:
        status = "DIFFERENT_DONOR_MAP_ANCHOR"
    return {"status": status, "pageid": page["pageid"], "revid": page["revid"],
            "source_sha256": page["sha256"], "title": page["title"], "cell": point}


def travel_destinations():
    return {
        tuple(route["destination"][axis] for axis in ("x", "y", "floor"))
        for shard in (placements.ROOT / "content/services/travel").glob("travel-*.json")
        for record in json.loads(shard.read_text())["records"]
        for route in record["declaration"].get("routes", [])
    }


def read_sources(checkout):
    result = {}
    for path, digest in SOURCES.items():
        source = checkout / path
        if hashlib.sha256(source.read_bytes()).hexdigest() != digest:
            raise ValueError(f"{path}: expected Crystal {REVISION} spawn XML")
        result[path] = read_placement_xml(source, path)
    return result


def audit(checkout, rejected):
    sources = read_sources(checkout)
    base = defaultdict(set)
    for name, data in placements.convert(include_map_import=False).items():
        if name.startswith("placements-"):
            for record in json.loads(data)["records"]:
                row = record["declaration"]
                base[row["npc"]].add(tuple(row["cell"][axis] for axis in ("x", "y", "floor")))
    occupied = set().union(*base.values())
    admitted = placements.admitted_keys()
    facts = json.loads(FACTS.read_text())
    pages = {p["pageid"]: p for p in facts["pages"]}
    by_title, _, _ = br_index(facts)
    crosscheck = {row["npc"]: row["br"]["pageid"] for row in json.loads(
        (AUTHORING / "samples/tibiawiki-br-crosscheck-v1.json").read_text())["npcs"]
        if row.get("br")}
    destinations = travel_destinations()
    # The Crystal Global XML duplicates the island's annual-event placements.
    seasonal_cells = {(npc_key(row["name"]), placements.cell_of(row["position"]))
                      for row in sources[WINTERLIGHT]}
    world = json.loads((placements.ROOT / "content/world/worlds/worlds-00000-00000.json").read_text())
    bounds = world["records"][0]["declaration"]["bounds"]

    rows = sources[GLOBAL]
    global_names = {npc_key(row["name"]) for row in rows if row["engine_spawns"]}
    contenders = defaultdict(set)
    for row in rows:
        if row["engine_spawns"]:
            contenders[placements.cell_of(row["position"])].add(npc_key(row["name"]))
    imported = defaultdict(list)
    skipped = defaultdict(list)
    existing = defaultdict(list)
    for row in rows:
        key = npc_key(row["name"])
        x, y, z = cell = placements.cell_of(row["position"])
        page = by_title.get(normalize(row["name"])) or pages.get(crosscheck.get(key))
        comparison = wiki_comparison(page, cell)
        detail = {"name": row["name"], "cell": cell, "wiki_comparison": comparison}
        if not row["engine_spawns"]:
            skipped["donor_interval_out_of_bounds"].append(detail)
        elif key in base:
            existing[key].append({**detail, "same_cell": cell in base[key]})
        elif (key, cell) in seasonal_cells:
            skipped["seasonal_map_anchor"].append(detail)
        elif key not in admitted:
            skipped["missing_native_definition"].append(detail)
        elif not (bounds["min_x"] <= x < bounds["max_x_exclusive"]
                  and bounds["min_y"] <= y < bounds["max_y_exclusive"]):
            skipped["outside_native_world"].append(detail)
        elif cell in destinations:
            skipped["native_travel_destination"].append(detail)
        elif cell in occupied or len(contenders[cell]) > 1:
            skipped["shared_or_reserved_cell"].append(detail)
        else:
            placement = {field: row[field] for field in
                         ("name", "position", "direction", "spawn_radius", "spawn_interval_s")}
            placement["wiki_comparison"] = comparison
            point = comparison.get("cell")
            if (cell in rejected and point and tuple(point) != cell and point[2] == z
                    and max(abs(point[0] - x), abs(point[1] - y)) <= row["spawn_radius"]):
                target = dict(zip(("x", "y", "z"), point))
                adjustment = {"source": row["position"], "target": target,
                              "reason": f"TibiaWiki BR marker within Crystal spawn radius; "
                                        f"donor anchor held by the compiler ({rejected[cell]})."}
                try:
                    target_cell, _ = placements.choose_cell(
                        placements.ROOT, cell, adjustment, occupied | destinations | contenders.keys(),
                        require_walkable=True)
                except (ValueError, FileNotFoundError):
                    pass
                else:
                    placement["adjustment"] = adjustment
                    occupied.add(target_cell)
            occupied.add(cell)
            imported[key].append(placement)

    other_maps = {}
    for path, map_rows in sources.items():
        if path == GLOBAL:
            continue
        names = {npc_key(row["name"]) for row in map_rows if row["engine_spawns"]}
        other_maps[path] = {"names": len(names), "rows": len(map_rows),
                            "names_absent_from_global": len(names - global_names),
                            "reason": "seasonal_map" if "/annual_events/" in path
                            else "unmaterialized_map"}

    packet = {"source": {"path": GLOBAL, "revision": REVISION, "sha256": SOURCES[GLOBAL]},
              "npcs": dict(sorted(imported.items()))}
    report = {
        "global_names": len(global_names),
        "all_map_names": len({npc_key(row["name"]) for map_rows in sources.values()
                              for row in map_rows if row["engine_spawns"]}),
        "global_rows": len(rows),
        "existing_npcs": len(existing),
        "existing_rows": sum(map(len, existing.values())),
        "existing_different_cells": [row for matches in existing.values()
                                     for row in matches if not row["same_cell"]],
        "import_npcs": len(imported),
        "import_rows": sum(map(len, imported.values())),
        "wiki_comparison": dict(Counter(row["wiki_comparison"]["status"]
                                       for matches in imported.values() for row in matches)),
        "skipped": dict(sorted(skipped.items())),
        "adjusted_rows": sum("adjustment" in row for matches in imported.values() for row in matches),
        "other_maps": other_maps,
    }
    return packet, report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkout", type=Path)
    parser.add_argument("--trial-parity", type=Path,
                        help="parity JSON from a first pass to check for source-backed nearby clear tiles")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    rejected = {}
    if args.trial_parity:
        rejected = {tuple(row["cell"]): row["reason"] for row in
                    json.loads(args.trial_parity.read_text())["npcs"]["held"]}
    packet, report = audit(args.checkout, rejected)
    if args.write:
        scope = json.loads(placements.SCOPE.read_text())
        scope.pop("crystal_global_map", None)
        fields = [f'    "source": {placements.dump(packet["source"])}', '    "npcs": {']
        entries = [f"      {json.dumps(key)}: {placements.dump(value)}"
                   for key, value in packet["npcs"].items()]
        content = "\n".join([fields[0] + ",", fields[1], ",\n".join(entries), "    }"])
        prefix = json.dumps(scope, ensure_ascii=False, indent=2).removesuffix("\n}")
        placements.SCOPE.write_text(
            prefix + ',\n  "crystal_global_map": {\n' + content + '\n  }\n}\n'
        )
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
