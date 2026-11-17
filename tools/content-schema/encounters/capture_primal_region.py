#!/usr/bin/env python3
"""Capture pinned Crystal Primal map facts; never invent or admit map cells."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "world-authoring"))
import otbm_reader as otbm

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
MAP_PATH = "data-global/world/world.otbm"
MAP_BLOB = "e61658ec0ab7b70ec35025ae969426d6fc57ae75"
LEVER_PATH = "data-global/scripts/actions/bosses_levers/the_primal_manace.lua"


def capture(raw):
    blob = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
    if blob != MAP_BLOB:
        raise ValueError("pinned Crystal map blob mismatch")
    document = json.loads((Path(__file__).resolve().parents[3] / "content/encounters/bosses/crystal-lever-candidates.json").read_bytes())
    if document["source_revision"] != PIN:
        raise ValueError("lever capture revision mismatch")
    lever = next(row for row in document["records"] if row["path"] == LEVER_PATH)
    # Crystal 201189f the_primal_manace.lua:8-20 defines all required positions.
    low, high = lever["zone_from"], lever["zone_to"]
    required = {tuple(p) for p in lever["lever_positions"] + lever["entry_positions"] + [lever["boss_position"], lever["exit_position"]]}
    rows = {}

    def handle(node):
        if node[0] != otbm.NODE_TILE_AREA:
            return
        bx, by, z = struct.unpack_from("<HHB", node[1])
        for tile in node[2]:
            if tile[0] not in (otbm.NODE_TILE, otbm.NODE_HOUSE_TILE):
                raise ValueError("unsupported source tile kind")
            position = (bx + tile[1][0], by + tile[1][1], z)
            if position not in required and not all(a <= value <= b for a, value, b in zip(low, position, high)):
                continue
            if position in rows:
                raise ValueError("duplicate source tile")
            facts = otbm.MapFacts()
            house, flags, zones, items = otbm._tile_content(tile, facts)
            if facts.unknown_item_attrs or facts.unknown_tile_attrs:
                raise ValueError("unknown selected source attributes")
            rows[position] = {"source_position": list(position), "house_id": house, "otbm_flags": flags or 0, "zones": list(zones),
                "items": [{"server_item_id": identity, "depth": depth, "attributes": {str(key): value for key, value in attrs.items()}} for identity, depth, attrs in items]}
            if len(rows) > 1024:
                raise ValueError("Primal source region capacity exceeded")

    otbm._scan(otbm.load_bytes(raw), handle)
    if not required.issubset(rows):
        raise ValueError("required Primal source placement missing")
    return {"schema": "OTERYN_CRYSTAL_PRIMAL_REGION_CAPTURE/v1", "source_repository": "zimbadev/crystalserver", "source_revision": PIN,
        "map_path": MAP_PATH, "map_git_blob": blob, "map_sha256": hashlib.sha256(raw).hexdigest(),
        "lever_path": LEVER_PATH, "lever_sha256": lever["sha256"], "zone_from": low, "zone_to": high,
        "required_positions": [list(p) for p in sorted(required)], "tiles": [rows[p] for p in sorted(rows)]}


def attach_item_semantics(document, raw_bank, raw_xml):
    expected = [("appearances", raw_bank, "37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0"),
                ("items XML", raw_xml, "ea4196beae8f77be898f05de89db81de134886b6")]
    for name, raw, blob in expected:
        actual = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
        if actual != blob:
            raise ValueError(f"pinned Crystal {name} blob mismatch")
    # Reuse the repository item-loader projection. Crystal's source lever
    # the_primal_manace.lua:8-20 remains the authority for region placements.
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "native-gameplay"))
    import build_source_world as source_world
    identities = {item["server_item_id"] for tile in document["tiles"] for item in tile["items"]}
    policies = source_world.policies(raw_bank, raw_xml, identities)
    by_id = {policy["server_item_id"]: policy for policy in policies}
    for tile in document["tiles"]:
        tile["semantics"] = source_world.tile_semantics(tile, by_id)
    document["item_policies"] = policies
    document["item_source_closure"] = [{"path": path, "git_blob": blob, "sha256": hashlib.sha256(raw).hexdigest()}
        for path, (_, raw, blob) in zip(["data/items/appearances.dat", "data/items/items.xml"], expected)]
    return document


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--map", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--appearances", type=Path, required=True)
    parser.add_argument("--items", type=Path, required=True)
    args = parser.parse_args()
    document = attach_item_semantics(capture(args.map.read_bytes()), args.appearances.read_bytes(), args.items.read_bytes())
    args.output.write_text(json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n")
    print(f"Captured {len(document['tiles'])} source tiles; every required placement exists")


if __name__ == "__main__":
    main()
