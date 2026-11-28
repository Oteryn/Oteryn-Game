"""Qualify the Bear Cave shovel target as Crystal's walkable ground."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
from client_appearance_reader import read_appearances

REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
APPEARANCES = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
SHARD = "content/world/terrain/terrain-08751-08751.json"
INDEX = "content/world/terrain/index.json"
NOTE = " Additionally the Crystal Bear Cave shovel target retains its Item 593 ground identity."


def known(value):
    return {"state": "KNOWN", "value": value}


def outputs(root=ROOT, *, terrain_index=None):
    raw = (root / f"content/assets/files/appearances-{APPEARANCES}.dat").read_bytes()
    if hashlib.sha256(raw).hexdigest() != APPEARANCES:
        raise ValueError("classic quest ground appearance digest")
    appearance = read_appearances(raw)[593]
    bindings = [
        row["target"]
        for row in json.loads(
            (root / "imports/crystalserver/bindings/items.json").read_bytes()
        )["bindings"]
        if row["external_id"] == "593"
    ]
    pointer = {
        "family": "Item",
        "key": "oteryn:item.tibia.i593",
        "revision": "definition-r1",
    }
    if (
        "bank" not in appearance.flags
        or {"unpass", "unshoot"} & appearance.flags
        or appearance.speed != 160
        or appearance.automap_color != 24
        or bindings != [pointer]
    ):
        raise ValueError("unqualified Bear Cave ground 593")
    record = {
        "identity": {
            "family": "Terrain",
            "key": "oteryn:terrain.tibia.i593",
            "revision": "definition-r1",
        },
        "kind": known("ground"),
        "walkable": known(True),
        "ground_speed": known(appearance.speed),
        "blocks_projectile": known(False),
        "blocks_sight": {"state": "UNKNOWN"},
        "floor_change": {"state": "UNKNOWN"},
        "automap": known({"shown": True, "color": appearance.automap_color}),
        "client_projection": known({"appearance_id": 593}),
        "provenance": {
            "source": {
                "engine": "crystal",
                "repository": "zimbadev/crystalserver",
                "revision": REVISION,
            },
            "item_pointer": pointer,
            "source_item_id": 593,
            "route": {"owner": "Terrain", "reason": "ground_or_border"},
            "fields": {
                field: [f"official_client:{APPEARANCES}.{fact}"]
                for field, fact in (
                    ("kind", "flags.bank"),
                    ("walkable", "flags.unpass"),
                    ("ground_speed", "bank.waypoints"),
                    ("blocks_projectile", "flags.unshoot"),
                    ("automap", "flags.automap"),
                    ("client_projection", "id"),
                )
            },
        },
    }
    index = json.loads(
        (root / INDEX).read_bytes() if terrain_index is None else terrain_index
    )
    import activity_static

    index["notes"] = activity_static.compose_terrain_notes(index["notes"] + NOTE)
    return {
        SHARD: json.dumps({"family": "Terrain", "records": [record]}, indent=2) + "\n",
        INDEX: json.dumps(index, indent=2, ensure_ascii=False) + "\n",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, value in outputs().items():
        path = ROOT / name
        if args.check:
            if path.read_bytes() != value.encode():
                raise ValueError(f"classic quest ground drift: {name}")
        else:
            path.write_bytes(value.encode())
