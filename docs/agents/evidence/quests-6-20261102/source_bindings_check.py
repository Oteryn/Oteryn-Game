"""Replay the batch-6 source packet against exact Git blobs; never activate runtime.

Usage: python source_bindings_check.py [--donor /path/to/crystalserver]
Without --donor, use ~/oteryn-cache/ref/crystal-git (the existing local cache).
The complete OTBM structure is scanned; attributes are decoded only for selected cells.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
import sys
import xml.etree.ElementTree as ET

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
OTERYN_HEAD = "b595422d912bbfdc8447f6e31053a1fc322fbe44"
BASE = "798e5b1ce27f441d5cbeece333dc7ac45e19d348"
NPCS = ["Telas", "Golem Servant", "Milos", "Boveas", "Cerdras", "Dreadeye", "Scutty", "Ceiron", "Ustan", "A Majestic Warwolf", "Elane", "Legola"]
REWARDS = [6052, 6053, 6054, 6075, 6157, 6158, 6258]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def git(root, *args):
    return subprocess.check_output(["git", "--no-replace-objects", *args], cwd=root, timeout=30)


def verify(packet, donor, root):
    require(packet["donor"]["revision"] == PIN and packet["oteryn_head"] == OTERYN_HEAD, "wrong source pins")
    require(packet["classification"] == "OTS_HYPOTHESIS_ONLY", "wrong evidence classification")
    require(packet["evidence_status"] == "PROVEN_SOURCE_FACTS_ONLY", "wrong evidence scope")
    require(packet["runtime_activation"] is False, "source packet cannot activate runtime")
    require(packet["unparsed_tables"] == [] and packet["missing_called_tables"] == [], "incomplete source table parse")
    tree = {}
    for row in git(donor, "ls-tree", "-r", "-z", PIN).split(b"\0"):
        if row:
            meta, name = row.split(b"\t", 1)
            tree[name.decode()] = meta.split()[2].decode()
    sources = {}
    for name, descriptor in packet["source_inputs"].items():
        relative = Path(name)
        require(not relative.is_absolute() and ".." not in relative.parts, "source path escapes donor")
        data = git(donor, "show", PIN + ":" + name)
        blob = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
        require(blob == tree.get(name) == descriptor["git_blob_sha1"], "Git blob differs: " + name)
        require(len(data) == descriptor["bytes"] and hashlib.sha256(data).hexdigest() == descriptor["sha256"], "source bytes differ: " + name)
        sources[name] = data
    for name, digest in packet["oteryn_inputs"].items():
        data = (root / name).read_bytes()
        require(hashlib.sha256(data).hexdigest() == digest, "Oteryn input differs: " + name)
        require(data == git(root, "show", OTERYN_HEAD + ":" + name), "Oteryn input is not the pinned blob")
    sys.path.insert(0, str(root / "tools/content-schema/quest-authoring"))
    sys.path.insert(0, str(root / "tools/content-schema/world-authoring"))
    import lua_tables
    import otbm_reader

    loader = "data-global/scripts/globalevents/others/map_attributes_loader.lua"
    calls = []
    for line, text in enumerate(sources[loader].decode().splitlines(), 1):
        match = re.search(r"\b(loadLuaMapAction|loadLuaMapUnique)\((\w+)\)", text)
        if match:
            calls.append({"kind": "action" if match[1].endswith("Action") else "unique", "table": match[2], "caller_line": line})
    require(calls == packet["startup_calls"], "startup call chain differs")
    tables = {}
    for name, data in sources.items():
        if name.startswith("data-global/startup/tables/") and any(re.search(r"\b" + re.escape(c["table"]) + r"\s*=", data.decode()) for c in calls):
            for table, node in lua_tables.assignments(data.decode(), {c["table"] for c in calls}).items():
                require(table not in tables, "duplicate table definition")
                tables[table] = (name, node)
    require(set(tables) == {c["table"] for c in calls}, "called table missing")
    old = json.loads(git(root, "show", BASE + ":content/quests/world-callbacks.json"))["callbacks"]
    old_keys = {c["key"] for c in old}
    current = json.loads((root / "content/quests/world-callbacks.json").read_bytes())["callbacks"]
    requests = []
    for callback in current:
        if callback["key"] not in old_keys:
            for kind in ("action", "unique"):
                if callback["selector"].get(kind) is not None:
                    requests.append({"callback": callback["key"], "quest": callback["quest"], "kind": kind, "id": callback["selector"][kind], "selector": callback["selector"]})
    require(len(requests) == packet["callback_requests"] == 67, "unexpected callback scope")
    requests.extend({"reward_uid": uid, "kind": "unique", "id": uid} for uid in REWARDS)
    expected = []
    for request in requests:
        matches = []
        for call in calls:
            if call["kind"] != request["kind"]:
                continue
            name, node = tables[call["table"]]
            fields = [f for f in node["fields"] if f["key"] == request["id"]]
            require(len(fields) <= 1, "ambiguous selected source key")
            for field in fields:
                value = lua_tables.as_python(field["value"])
                for block_index, block in enumerate(value if isinstance(value, list) else [value]):
                    positions = block["itemPos"]
                    positions = positions if isinstance(positions, list) else [positions]
                    matches.append({"table": call["table"], "table_entry_line": field["line"], "path": name, "block_index": block_index, "item_id": block["itemId"], "positions": positions, "loader_path": loader, "loader_line": call["caller_line"], "raw_map_item_lookup_required": True, "oteryn_runtime_binding_admitted": False})
        require(bool(matches), "selected binding missing")
        expected.append({**request, "status": "SOURCE_STARTUP_BINDING_FOUND", "matches": matches})
    require(expected == packet["binding_records"], "binding projection differs")
    require(len(expected) == packet["source_bindings_found"] == 74, "unexpected binding count")
    xml_bytes = sources["data-global/world/world-npc.xml"]
    xml = ET.fromstring(xml_bytes)
    expected_npcs = []
    for name in NPCS:
        matches = []
        for group_index, group in enumerate(xml):
            for child_index, child in enumerate(group):
                if child.get("name", "").casefold() == name.casefold():
                    lines = [i for i, text in enumerate(xml_bytes.decode().splitlines(), 1) if re.search(r'name="' + re.escape(child.attrib["name"]) + r'"', text)]
                    matches.append({"xml_path": f"/npcs/npc[{group_index+1}]/npc[{child_index+1}]", "source_name": child.attrib["name"], "name_occurrence_lines": lines, "group_attributes": dict(group.attrib), "npc_attributes": dict(child.attrib)})
        require(bool(matches), "selected NPC missing")
        expected_npcs.append({"name": name, "source_occurrences": matches, "status": "SOURCE_SPAWN_FOUND", "oteryn_runtime_binding_admitted": False})
    require(expected_npcs == packet["npc_source_records"], "NPC projection differs")
    wanted = {tuple(p[k] for k in ("x", "y", "z")) for row in expected for match in row["matches"] for p in match["positions"]}
    for row in expected_npcs:
        for match in row["source_occurrences"]:
            group, npc = match["group_attributes"], match["npc_attributes"]
            wanted.add((int(group["centerx"]) + int(npc["x"]), int(group["centery"]) + int(npc["y"]), int(group["centerz"])))
    facts = otbm_reader.MapFacts()
    selected = {}
    areas = 0
    def handle(node):
        nonlocal areas
        if node[0] != otbm_reader.NODE_TILE_AREA:
            return
        areas += 1
        bx, by, z = struct.unpack_from("<HHB", node[1], 0)
        for tile in node[2]:
            require(tile[0] in (otbm_reader.NODE_TILE, otbm_reader.NODE_HOUSE_TILE), "unexpected tile node")
            pos = (bx + tile[1][0], by + tile[1][1], z)
            facts.tiles += 1
            if pos in wanted:
                require(pos not in selected, "duplicate selected tile")
                house, flags, zones, items = otbm_reader._tile_content(tile, facts)
                selected[pos] = {"position": dict(zip(("x", "y", "z"), pos)), "flags": flags, "house_id": house, "zones": list(zones), "items": [{"item_id": item, "depth": depth, "attributes": {str(k): v for k, v in attrs.items()}} for item, depth, attrs in items]}
    tree_root = otbm_reader._scan(otbm_reader.load_bytes(sources["data-global/world/world.otbm"]), handle)
    require(not facts.unknown_item_attrs and not facts.unknown_tile_attrs, "unknown selected attributes")
    require(set(selected) == wanted and len(wanted) == 92, "selected source tile missing")
    projection = packet["donor_map"]
    actual_tiles = json.loads(json.dumps([selected[k] for k in sorted(selected)]))
    require(actual_tiles == projection["selected_tiles"], "map projection differs")
    require((areas, facts.tiles) == (projection["tile_areas_scanned"], projection["tiles_scanned"]), "map scan counts differ")
    require(tuple(struct.unpack_from("<IHH", tree_root[1], 0)) == (projection["map_version"], projection["map_width"], projection["map_height"]), "map header differs")
    literal = dynamic = 0
    for row in expected:
        for match in row["matches"]:
            for pos in match["positions"]:
                if match["item_id"] is False:
                    dynamic += 1
                    continue
                tile = selected[tuple(pos[k] for k in ("x", "y", "z"))]
                hits = [i for i in tile["items"] if i["item_id"] == match["item_id"]]
                require(len(hits) == 1 and hits[0]["depth"] == 0, "literal item is missing, ambiguous, or nested")
                literal += 1
    require((literal, dynamic) == (83, 4), "selected lookup count differs")
    return {"callbacks": 67, "reward_descriptors": 7, "npc_names": 12, "npc_xml_occurrences": sum(len(r["source_occurrences"]) for r in expected_npcs), "selected_cells": len(selected), "literal_item_occurrence_checks": literal, "dynamic_item_false_checks_not_qualified": dynamic, "runtime_activation": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--donor", type=Path, default=Path.home() / "oteryn-cache/ref/crystal-git",
                        help="local Git repository containing the pinned Crystal commit")
    parser.add_argument("--packet", type=Path, default=Path(__file__).with_name("source-bindings-20261103.json.gz"))
    args = parser.parse_args()
    if not args.donor.is_dir():
        parser.error("local donor repository is missing; pass --donor /path/to/crystalserver")
    root = Path(__file__).resolve().parents[4]
    packet = json.loads(gzip.decompress(args.packet.read_bytes()))
    print(json.dumps(verify(packet, args.donor, root), sort_keys=True))


if __name__ == "__main__":
    main()
