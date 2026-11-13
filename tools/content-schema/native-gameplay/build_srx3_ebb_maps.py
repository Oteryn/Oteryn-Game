#!/usr/bin/env python3
"""Capture all pinned Soul War replacement tiles without executing donor Lua.

All source map records, including positions outside the descriptive realm box,
are retained. This producer does not publish an activation or inventory policy.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import struct
import build_source_world as source
import build_activity_source_world as activity

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
MAPS = [{'path': 'data-global/world/quest/soul_war/ebb_and_flow/ebb-flow-empty.otbm', 'git_blob': '9d3fa897425709c4d254f1ac0b598f26e2cbe0bd', 'sha256': '6b52ade34776b4f3ba1ccc047233940bffb3f4f88667de8d3d79f982f79b05ce', 'tiles': 12355}, {'path': 'data-global/world/quest/soul_war/ebb_and_flow/ebb-flow-inundate.otbm', 'git_blob': '01f462c2f7a821fca0e31dab752b4087eb2b84e3', 'sha256': 'f99cee95f55ec67526115def195f9d81fdcb4e487e8dadd2d63fae3f1d1ee726', 'tiles': 34392}, {'path': 'data-global/world/quest/soul_war/ebb_and_flow/ebb-flow.otbm', 'git_blob': '5a8d5d16fefe7f43e2bd163cb1f86eac75068fc0', 'sha256': '38725fb32054e71b9c736d4ac5a8c0072ef24ff2efb64c3b01a6a5e919eb2d7e', 'tiles': 18449}]

def verify_blob(raw, expected):
    actual = hashlib.sha1(f"blob {len(raw)}\0".encode()+raw).hexdigest()
    if actual != expected:
        raise ValueError("pinned source Git blob mismatch")


def rows_from_nodes(nodes):
    rows = []
    seen = set()
    for node in nodes:
        if node[0] != source.otbm.NODE_TILE_AREA:
            continue
        bx, by, z = struct.unpack_from("<HHB", node[1], 0)
        for tile in node[2]:
            if tile[0] not in (source.otbm.NODE_TILE, source.otbm.NODE_HOUSE_TILE):
                raise ValueError("unqualified replacement tile kind")
            facts = source.otbm.MapFacts()
            house, flags, zones, items = source.otbm._tile_content(tile, facts)
            if house is not None or facts.unknown_item_attrs or facts.unknown_tile_attrs:
                raise ValueError("unqualified replacement tile attributes")
            pos = (bx+tile[1][0], by+tile[1][1], z)
            if pos in seen:
                raise ValueError("duplicate replacement source position")
            seen.add(pos)
            rows.append({"source_position": list(pos), "otbm_flags": flags or 0,
                         "zones": list(zones), "items": [
                             {"server_item_id": item, "depth": depth,
                              "attributes": {str(k): v for k, v in attrs.items()}}
                             for item, depth, attrs in items]})
    return sorted(rows, key=lambda row: row["source_position"])


def captures(directory):
    inputs = {}
    closure = []
    for path in ("data/items/appearances.dat", "data/items/items.xml"):
        name, expected = activity.FILES[path]
        raw = (directory/name).read_bytes()
        verify_blob(raw, expected)
        inputs[path] = raw
        closure.append({"path": path, "git_blob": expected,
                        "sha256": hashlib.sha256(raw).hexdigest()})
    result = {}
    for spec in MAPS:
        raw = (directory/Path(spec["path"]).name).read_bytes()
        verify_blob(raw, spec["git_blob"])
        if hashlib.sha256(raw).hexdigest() != spec["sha256"]:
            raise ValueError("replacement source SHA256 mismatch")
        nodes = []
        source.otbm._scan(source.otbm.load_bytes(raw),
                          lambda node: nodes.append(node) if node[0] == source.otbm.NODE_TILE_AREA else None)
        rows = rows_from_nodes(nodes)
        if len(rows) != spec["tiles"]:
            raise ValueError("pinned replacement tile census changed")
        ids = {item["server_item_id"] for row in rows for item in row["items"]}
        policies = source.policies(inputs["data/items/appearances.dat"],
                                   inputs["data/items/items.xml"], ids)
        orders = activity.source_top_orders(inputs["data/items/appearances.dat"], ids)
        for policy in policies:
            policy["source_top_order"] = orders[policy["server_item_id"]]
        result[Path(spec["path"]).stem+".json"] = {
            "schema": "OTERYN_SOUL_WAR_EBB_SOURCE_MAP/v1",
            "source_revision": PIN, "source_path": spec["path"],
            "source_git_blob": spec["git_blob"], "source_sha256": spec["sha256"],
            "source_item_closure": closure,
            "replacement_semantics": "replace-only-present-tiles-ground-and-ordered-items-preserve-creatures-and-absent-cells",
            "item_policies": policies, "tiles": rows}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, required=True)
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    documents = captures(args.source_dir)
    if not args.check:
        args.out_dir.mkdir(parents=True, exist_ok=True)
    for name, document in documents.items():
        raw = (json.dumps(document, separators=(",", ":"))+"\n").encode()
        target = args.out_dir/name
        if args.check:
            if not target.exists() or target.read_bytes() != raw:
                raise SystemExit(f"stale Ebb source capture: {target}")
        else:
            target.write_bytes(raw)
        print(f"{target}: tiles={len(document['tiles'])} sha256={hashlib.sha256(raw).hexdigest()}")


if __name__ == "__main__":
    main()
