"""The source Ram is one immovable static appearance, never a portable Item.

Crystal201189f actions_reliable_ram.lua selects appearance53439 at31926/31896/7.
The retained B3 supplies its original stack ordinal; the admitted official15.30
artifact supplies physical flags. No source region or Item binding is invented.
"""

from __future__ import annotations

import argparse
import copy
import json
from functools import lru_cache
from pathlib import Path

import grizzly_ground

ROOT = Path(__file__).resolve().parents[3]
IDENTIFIER = 53439
KEY = f"oteryn:terrain.tibia.i{IDENTIFIER}"
PROFILE = "OTERYN_OFFICIAL_CLIENT_QUEST_RAM_WORLD/v1"
SHARD = "content/world/terrain/terrain-08750-08750.json"
NOTE = " Additionally one source-qualified Reliable Ram static appearance; total 8694 Terrain records."


def encode(value):
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode()


@lru_cache(maxsize=1)
def record():
    # This existing qualifier verifies the complete official artifact and the
    # admitted appearance-membership manifest before yielding source provenance.
    bindings = json.loads(
        (ROOT / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    if any(row["external_id"] == str(IDENTIFIER) for row in bindings["bindings"]):
        raise ValueError("Reliable Ram appearance-only route overlaps an Item binding")
    ground = json.loads(grizzly_ground.outputs()[grizzly_ground.SHARD])["records"][0]
    source = copy.deepcopy(ground["provenance"]["source"])
    source["profile"] = PROFILE
    raw = (ROOT / source["artifact"]).read_bytes()
    appearance = grizzly_ground.read_appearances(raw).get(IDENTIFIER)
    if appearance is None or appearance.flags != frozenset({"unmove", "unpass"}):
        raise ValueError("Reliable Ram official physical flags differ")
    admitted = json.loads(
        (ROOT / "imports/official/appearance-membership/admitted.json").read_bytes()
    )
    membership = next(
        row for row in admitted["files"] if row["label"] == "client-15.30"
    )
    members = json.loads(
        (
            ROOT / "imports/official/appearance-membership" / membership["manifest"]
        ).read_bytes()
    )
    if not any(row[0] == IDENTIFIER for row in members["entries"]):
        raise ValueError("Reliable Ram lacks official appearance membership")
    known = lambda value: {"state": "KNOWN", "value": value}
    unknown = lambda: {"state": "UNKNOWN"}
    return {
        "identity": {"family": "Terrain", "key": KEY, "revision": "definition-r1"},
        "kind": known("wall"),
        "walkable": known(False),
        "blocks_sight": known(False),
        "blocks_projectile": known(False),
        "floor_change": unknown(),
        "automap": known({"shown": False}),
        "client_projection": known({"appearance_id": IDENTIFIER}),
        "provenance": {
            "source": source,
            "source_item_id": IDENTIFIER,
            "route": {"owner": "Terrain", "reason": "appearance_only_quest_target"},
            "fields": {
                key: [f"official_client:{source['revision']}.{fact}"]
                for key, fact in [
                    ("kind", "flags.unpass"),
                    ("walkable", "flags.unpass"),
                    ("blocks_sight", "flags.unsight"),
                    ("blocks_projectile", "flags.unsight"),
                    ("automap", "flags.automap"),
                    ("client_projection", "id"),
                ]
            },
        },
    }


def outputs():
    index_path = "content/world/placements/index.json"
    index = json.loads((ROOT / index_path).read_bytes())
    rows = [row for row in index["palette"] if row["source_item_id"] == IDENTIFIER]
    if len(rows) != 1:
        raise ValueError("Reliable Ram must retain one palette position")
    rows[0].update(key=KEY, provisional=False)
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((ROOT / marker_path).read_bytes())
    if NOTE not in marker["notes"]:
        marker["notes"] += NOTE
    return {
        SHARD: encode({"family": "Terrain", "records": [record()]}),
        index_path: encode(index),
        marker_path: (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode(),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if not path.exists() or path.read_bytes() != raw:
                raise SystemExit(f"stale Reliable Ram static appearance: {name}")
        else:
            path.write_bytes(raw)
