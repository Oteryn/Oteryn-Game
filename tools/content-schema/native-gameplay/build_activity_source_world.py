#!/usr/bin/env python3
"""Extract the bounded SRX-3 activity rooms from pinned Crystal map/Item facts.

Reads reference files only. No donor script is executed and absent map tiles
stay absent. The artifact keeps the immutable map stack and explicitly retains
mutable placements for the existing source Item owner.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import struct
import sys

import build_source_world as source


AMBIENT_XML_KEYS = {
    "soul-war-spite": {"oteryn:creature.weeping_soul", "oteryn:creature.spiteful_spitter"},
    "soul-war-hatred": {"oteryn:creature.symbol_of_hatred"},
    "soul-war-cruelty": {"oteryn:creature.poor_soul"},
    "soul-war-megalomania": {"oteryn:creature.necromantic_focus", "oteryn:creature.lesser_splinter_of_madness"},
    "callback-ragiaz": {"oteryn:creature.bone_capsule"},
}

def ambient_xml_spawns(name):
    rows = json.loads(Path(__file__).with_name("srx3-spite-spawns.json").read_bytes())
    selected = [row for row in rows if row["creature_key"] in AMBIENT_XML_KEYS[name]]
    expected = {"soul-war-spite": 3, "soul-war-hatred": 2, "soul-war-cruelty": 1,
                "soul-war-megalomania": 3, "callback-ragiaz": 1}[name]
    if len(selected) != expected:
        raise ValueError("bounded ambient XML capture is incomplete for " + name)
    return selected

ROOT = Path(__file__).resolve().parents[3]
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
FILES = {
    "data-global/world/world.otbm": ("crystal-world.otbm", "e61658ec0ab7b70ec35025ae969426d6fc57ae75"),
    "data/items/appearances.dat": ("appearances.dat", "37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0"),
    "data/items/items.xml": ("items.xml", "ea4196beae8f77be898f05de89db81de134886b6"),
}
ROOMS = {
    # Crystal201189f adventures_of_galthen/actions_ahau.lua:3-23.
    # The exit teleporter is outside specPos; capture its literal cell without
    # extending combat bounds or claiming absence across the intervening gap.
    "galthen-ahau": {
        "entry": [34008, 31703, 10],
        "regions": [[33999, 31692, 34018, 31705, 10, 10],
                    [34002, 31706, 34002, 31706, 10, 10],
                    [34033, 31714, 34038, 31714, 10, 10],
                    [34036, 31717, 34036, 31717, 10, 10]],
    },
    # actions_mitmah_vanguard.lua:3-23 and its separate exit Action:2-4,16.
    # The literal USE exit is in the combat rectangle; its destination is not.
    "galthen-mitmah-vanguard": {
        "entry": [34067, 31415, 11],
        "regions": [[34054, 31400, 34081, 31419, 11, 11],
                    [34047, 31431, 34053, 31431, 11, 11],
                    [34060, 31432, 34060, 31432, 11, 11]],
    },
    # W01 fixed-global callback contexts. Literal arrival anchors and room bounds
    # come from pinned Crystal actions_* scripts; callback spectator/ray bounds
    # are the union with their pinned Canary spell literals. These are map
    # contexts only: capture qualification grants no boss/Player entry.
    "callback-lady-tenebris": {"entry": [32911, 31603, 14],
        "regions": [[32900, 31587, 32924, 31611, 14, 14]]},
    "callback-outburst": {"entry": [32234, 31292, 14],
        # Source onUseExtra explicitly queries23482 and transforms it to23483.
        # Current pinned map initially contains23483, so first lever is Lua nil.
        "extra_items": [23482],
        # actions_outburst_lever.lua:7–11,37,41: actual lever/waiting and exit.
        "regions": [[32223, 31273, 32246, 31297, 14, 14],
                    [32207, 31283, 32207, 31288, 14, 14],
                    [32218, 31375, 32218, 31375, 14, 14],
                    # Actual initial23483 builtin teleport dest from pinned OTBM.
                    [32208, 31372, 32208, 31372, 14, 14]]},
    "callback-professor-maxxen": {"entry": [33711, 32052, 15],
        "regions": [[33704, 32040, 33718, 32053, 15, 15]]},
    "callback-razzagorn": {"entry": [33419, 32467, 14],
        "regions": [[33408, 32454, 33440, 32480, 14, 14]]},
    "callback-shulgrax": {"entry": [33485, 32790, 13],
        "regions": [[33473, 32776, 33496, 32798, 13, 13]]},
    "callback-plagirath": {"entry": [33173, 31504, 13],
        "regions": [[33159, 31491, 33185, 31513, 13, 13]]},
    "callback-ragiaz": {"entry": [33482, 32339, 13],
        "regions": [[33472, 32323, 33493, 32347, 13, 13],
                    [33485, 32333, 33485, 32333, 14, 14],
                    [33487, 32333, 33487, 32333, 14, 14]]},
    "callback-zamulosh": {"entry": [33644, 32760, 11],
        "regions": [[33634, 32749, 33654, 32765, 11, 11]]},
    "callback-time-guardian": {"entry": [32977, 31667, 14],
        "regions": [[32962, 31647, 32992, 31677, 14, 14],
                    [32967, 31654, 32989, 31677, 13, 13],
                    [32798, 32649, 32828, 32679, 14, 14]]},
    "callback-mazoran": {"entry": [33585, 32693, 14], "extra_items": [21494],
        "regions": [[33572, 32679, 33599, 32701, 14, 14]]},
    "callback-anomaly": {"entry": [32271, 31257, 14], "extra_items": [22894, 23049],
        "regions": [[32258, 31237, 32284, 31262, 14, 14]]},
    # Crystal201189f world-zones.xml:3 maps boss.brain-head to zone1.
    # The actual pinned OTBM zone1 has476 tiles, bbox31941,32313..31966,32337,z10.
    # actions_portal_brain_head.lua:6,7,70-76 supplies entry/exit/monster anchors.
    "brain-head": {
        "entry": [31963, 32324, 10],
        "regions": [[31941, 32313, 31966, 32337, 10, 10],
                    [31971, 32325, 31971, 32325, 10, 10]],
    },
    "primal": {
        "entry": [33565, 32758, 15],
        "regions": [[33547, 32749, 33570, 32769, 15, 15],
                    [33546, 32751, 33553, 32753, 14, 14],
                    [33519, 32870, 33521, 32872, 15, 15]],
    },
    "soulpit": {
        "extra_items": [47379],
        "entry": [32373, 31151, 8],
        "regions": [[32362, 31132, 32390, 31153, 8, 8],
                    [32372, 31157, 32378, 31163, 8, 8]],
    },
    # Crystal 201189f data-global/lib/quests/soul_war.lua:296-479;
    # data-global/scripts/quests/rotten_blood_quest/rotten_{boss}.lua:1-31.
    # Each capture includes the exact arena, source staging/lever tiles and exit;
    # padding only reads genuine neighboring map cells, never synthesizes terrain.
    'soul-war-spite': {'entry': [33742, 31639, 14],
     'regions': [[33734, 31624, 33751, 31640, 14, 14],
                 [33773, 31633, 33779, 31635, 14, 14],
                 [33620, 31426, 33622, 31428, 10, 10]]},
    # Crystal lib/quests/soul_war.lua:1139-1140 transforms existing ground to409.
    # Retain its real policy without inventing an initially placed white tile.
    'soul-war-malice': {'entry': [33710, 31605, 14], 'extra_items': [409],
     'regions': [[33699, 31590, 33718, 31607, 14, 14],
                 [33678, 31598, 33684, 31600, 14, 14],
                 [33620, 31426, 33622, 31428, 10, 10]]},
    'soul-war-greed': {'entry': [33747, 31671, 14],
     'regions': [[33737, 31658, 33755, 31673, 14, 14],
                 [33775, 31664, 33781, 31666, 14, 14],
                 [33620, 31426, 33622, 31428, 10, 10]]},
    'soul-war-hatred': {'entry': [33743, 31604, 14],
     'regions': [[33735, 31592, 33751, 31606, 14, 14],
                 [33772, 31600, 33778, 31602, 14, 14],
                 [33620, 31426, 33622, 31428, 10, 10]]},
    'soul-war-cruelty': {'entry': [33856, 31872, 7],
     'regions': [[33847, 31858, 33864, 31874, 7, 7],
                 [33853, 31853, 33859, 31855, 6, 6],
                 [33620, 31426, 33622, 31428, 10, 10]]},
    'soul-war-megalomania': {'entry': [33710, 31639, 14], 'extra_items': [409],
     # Includes the genuine room terrain and bounded surrounding source cells.
     # Blue Lua:9 centre3 means actual cross reach is +/-8x and +/-7y.
     'regions': [[33694, 31620, 33719, 31642, 14, 14],
                 [33675, 31633, 33681, 31635, 14, 14],
                 [33620, 31426, 33622, 31428, 10, 10]]},
    'rotten-blood-murcion': {'entry': [33009, 32372, 15],
     'regions': [[33000, 32360, 33017, 32375, 15, 15],
                 [32973, 32364, 32980, 32366, 15, 15],
                 [33903, 31692, 33905, 31694, 15, 15]]},
    'rotten-blood-ichgahal': {'entry': [33004, 32341, 15],
     # rotten_ichgahal.lua raw exit33004,32343,15 has Teleport34111 with
     # the pinned OTBM destination34065,31717,15. Retain its actual arrival
     # stack36344/292 without opening the unsupported raw Teleport tile.
     'regions': [[32998, 32326, 33018, 32343, 15, 15],
                 [32973, 32332, 32980, 32334, 15, 15],
                 [34065, 31717, 34065, 31717, 15, 15]]},
    'rotten-blood-chagorz': {'entry': [33044, 32373, 15],
     'regions': [[33035, 32358, 33052, 32375, 15, 15],
                 [33073, 32366, 33080, 32368, 15, 15],
                 [33903, 31692, 33905, 31694, 15, 15]]},
    'rotten-blood-vemiath': {'entry': [33043, 32341, 15],
     # rotten_vemiath.lua:18 raw exit has source34111 Teleport destination
     # (34001,31820,15), read from the pinned OTBM item attribute8. Capture the
     # genuine arrival tile; no walkability or synthetic tile added.
     'regions': [[33035, 32327, 33052, 32344, 15, 15],
                 [33073, 32332, 33080, 32334, 15, 15],
                 [34001, 31820, 34001, 31820, 15, 15]]},
    'rotten-blood-bakragore': {'entry': [33044, 32407, 15],
     'extra_items': [43497,43498,43507,43508,43983,43984,43985,43986,43987,43988],
     'regions': [[33034, 32389, 33053, 32410, 15, 15],
                 [33009, 32391, 33016, 32393, 15, 15],
                 [33073, 32397, 33080, 32399, 15, 15],
                 # bakragore.lua:369-414; genuine approaches connect all eight
                 # flame/pillar cells. This does not expand the encounter zone.
                 [33066, 32397, 33086, 32399, 15, 15],
                 [33075, 32388, 33077, 32408, 15, 15],
                 [34105, 32051, 34107, 32053, 13, 13]]},
}



def source_top_order(flags):
    # Crystal201189f src/items/items.cpp:203-208; appearances.proto:149-151.
    # clip has priority over top, then bottom; ordinary items retain order0.
    return next((order for field, order in ((2, 1), (4, 3), (3, 2))
                 if bool(flags.get(field, [0])[0])), 0)


def source_top_orders(raw, ids):
    result = {}
    for record in source.fields(raw).get(1, []):
        obj = source.fields(record)
        item_id = obj.get(1, [0])[0]
        if item_id not in ids:
            continue
        if item_id in result:
            raise ValueError("duplicate source top-order appearance")
        result[item_id] = source_top_order(source.fields(obj.get(3, [b""])[0]))
    if set(result) != ids:
        raise ValueError("missing source top-order appearance")
    return result


def extract_many(raw, recipes):
    result = {name: [] for name in recipes}
    seen = {name: set() for name in recipes}

    def visit(node):
        if node[0] != source.otbm.NODE_TILE_AREA:
            return
        bx, by, z = struct.unpack_from("<HHB", node[1], 0)
        candidates = {name: regions for name, regions in recipes.items()
                      if any(e <= z <= f and bx <= c and bx + 255 >= a
                             and by <= d and by + 255 >= b
                             for a, b, c, d, e, f in regions)}
        if not candidates:
            return
        for tile in node[2]:
            if tile[0] not in (source.otbm.NODE_TILE, source.otbm.NODE_HOUSE_TILE):
                raise ValueError("unknown activity tile type")
            x, y = bx + tile[1][0], by + tile[1][1]
            names = [name for name, regions in candidates.items()
                     if any(a <= x <= c and b <= y <= d and e <= z <= f
                            for a, b, c, d, e, f in regions)]
            if not names:
                continue
            facts = source.otbm.MapFacts()
            house, flags, zones, items = source.otbm._tile_content(tile, facts)
            if facts.unknown_item_attrs or facts.unknown_tile_attrs or house is not None:
                raise ValueError("unqualified activity tile attributes")
            row = {
                "source_position": [x, y, z], "native_position": [x, y, -z],
                "otbm_flags": flags or 0, "house_id": house, "zones": list(zones),
                "items": [{"server_item_id": item, "depth": depth,
                           "attributes": {str(k): v for k, v in attrs.items()}}
                          for item, depth, attrs in items],
            }
            for name in names:
                if (x, y, z) in seen[name]:
                    raise ValueError("duplicate activity tile")
                seen[name].add((x, y, z))
                result[name].append(row)

    source.otbm._scan(source.otbm.load_bytes(raw), visit)
    for name, rows in result.items():
        if not rows or len(rows) > 4096:
            raise ValueError(f"activity tile capacity: {name}")
        result[name] = sorted(rows, key=lambda row: row["source_position"])
    return result


def extract(raw, regions):
    return extract_many(raw, {"room": regions})["room"]


def load_inputs(directory):
    blobs, closure = {}, []
    for path, (filename, expected) in FILES.items():
        raw = (directory / filename).read_bytes()
        blob = hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest()
        if blob != expected:
            raise ValueError(f"pinned Crystal blob differs: {path}")
        blobs[path] = raw
        closure.append({"path": path, "git_blob": blob,
                        "sha256": hashlib.sha256(raw).hexdigest()})
    return blobs, closure


def build(directory, name, inputs=None, extracted=None):
    blobs, closure = inputs if inputs is not None else load_inputs(directory)
    room = ROOMS[name]
    rows = (extracted[name] if extracted is not None else
            extract(blobs["data-global/world/world.otbm"], room["regions"]))
    ids = {item["server_item_id"] for row in rows for item in row["items"]}
    ids.update(room.get("extra_items", []))
    policies = source.policies(blobs["data/items/appearances.dat"],
                               blobs["data/items/items.xml"], ids)
    top_orders = source_top_orders(blobs["data/items/appearances.dat"], ids)
    for policy in policies:
        policy["source_top_order"] = top_orders[policy["server_item_id"]]
    by_id = {policy["server_item_id"]: policy for policy in policies}
    for row in rows:
        row["semantics"] = source.tile_semantics(row, by_id)
    source_key = f"oteryn:source/crystal-{name}-activity-201189f"
    bindings = [{"disposition": "ACCEPTED_ALIAS", "source_key": source_key,
                 "source_revision": PIN, "identity_namespace": "ots/item_server_id",
                 "external_id": str(item), "target": {
                     "family": "Item", "key": f"oteryn:item.tibia.i{item}",
                     "revision": "definition-r1"}} for item in sorted(ids)]
    static_path = ROOT / "content/world/source-static-definitions.json"
    static = {row["source_item_id"]: row for row in json.loads(static_path.read_bytes())["records"]}
    native_keys = {row[0] for row in json.loads((ROOT / "tools/content-schema/native-gameplay/item-keys.json").read_bytes())["records"]}
    missing_definitions = []
    for binding in bindings:
        identifier = int(binding["external_id"])
        if identifier in static:
            if static[identifier]["policy"] != by_id[identifier]:
                raise ValueError("pinned static policy differs from its published body")
            binding["target"] = dict(static[identifier]["identity"])
        elif binding["target"]["key"] not in native_keys:
            sys.path.insert(0, str(ROOT / "tools/content-schema/world-object-authoring"))
            import activity_static
            policy = by_id[identifier]
            if identifier not in activity_static.IDENTIFIERS or policy["pickupable"] or policy["movable"] or policy["dynamic_kind"] is not None:
                missing_definitions.append(identifier)
                continue
            family, prefix = ("Terrain", "terrain") if policy["ground"] else ("WorldObject", "world-object")
            binding["target"] = {"family":family,"key":f"oteryn:{prefix}.tibia.i{identifier}","revision":"definition-r1"}
    if missing_definitions:
        raise ValueError(f"source capture has no qualified definition for {missing_definitions}")
    dynamic = {item for row in rows for item in row["semantics"]["unmaterialized_dynamic_items"]}
    regions = room["regions"]
    bounds = [min(r[0] for r in regions), min(r[1] for r in regions),
              max(r[2] for r in regions), max(r[3] for r in regions),
              min(r[4] for r in regions), max(r[5] for r in regions)]
    return {
        "schema": "OTERYN_NATIVE_ACTIVITY_SOURCE_WORLD/v1",
        "candidate_profile": f"native-crystal-{name}-activity-201189f",
        "source_repository": "https://github.com/zimbadev/crystalserver",
        "source_revision": PIN, "source_closure": closure,
        "source_bounds": bounds, "source_regions": regions,
        "source_entry": room["entry"],
        **({"registered_global_events": json.loads((Path(__file__).with_name("srx3-spite-global-event.json")).read_bytes())}
           if name == "soul-war-spite" else {}),
        **({"ambient_xml_spawns": ambient_xml_spawns(name)}
           if name in AMBIENT_XML_KEYS else {}),
        "imports": [], "sources": [],
        "native_frame": {"key": f"oteryn:frame/crystal-{name}-activity-201189f",
                         "map_revision": f"oteryn:map/crystal-{name}-activity-201189f",
                         "origin_x": 0, "origin_y": 0,
                         "floor_transform": "native_floor=-source_z"},
        "source_identity_bindings": bindings,
        "native_item_bindings": [binding for binding in bindings
                                 if int(binding["external_id"]) in dynamic],
        "item_policies": policies, "tiles": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, required=True)
    selection = parser.add_mutually_exclusive_group(required=True)
    selection.add_argument("--room", choices=ROOMS)
    selection.add_argument("--all", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source-regions-only", action="store_true",
                        help="require every previous field/body to remain identical")
    args = parser.parse_args()
    names = list(ROOMS) if args.all else [args.room]
    inputs = load_inputs(args.source_dir)
    extracted = extract_many(inputs[0]["data-global/world/world.otbm"],
                             {name: ROOMS[name]["regions"] for name in names})
    outputs = []
    for name in names:
        document = build(args.source_dir, name, inputs, extracted)
        raw = source.canonical(document) + b"\n"
        path = ROOT / f"content/encounters/{name}/source-room.json"
        if args.source_regions_only:
            previous = json.loads(path.read_bytes())
            previous.pop("source_regions", None)
            unchanged = json.loads(raw)
            unchanged.pop("source_regions")
            if previous != unchanged:
                raise SystemExit(f"non-region source data differs: {path}")
        if args.check and (not path.is_file() or path.read_bytes() != raw):
            raise SystemExit(f"stale activity source room: {path}")
        outputs.append((name, path, raw))
    for name, path, raw in outputs:
        if not args.check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
        print(f"{name}: {len(raw)} bytes sha256={hashlib.sha256(raw).hexdigest()}")


if __name__ == "__main__":
    main()
