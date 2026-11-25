"""Verify the existing Terrain routes used at the two Crystal NPC contacts."""

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "world-authoring"))

import scoped_spawn_ground
import spawn_ground
from elemental_ground import ROOT, routed_ground
from grizzly_ground import appearance_only_ground

PLACEMENTS = "content/world/placements/index.json"


def check(root=ROOT):
    dragon = json.loads(
        routed_ground(470, 95, "OTERYN_SLEEPING_DRAGON_SOURCE_GROUND/v1", root)
    )["records"][0]
    scoped = scoped_spawn_ground.records(root)[470]
    if {key: value for key, value in dragon.items() if key != "provenance"} != {
        key: value for key, value in scoped.items() if key != "provenance"
    }:
        raise ValueError("Sleeping Dragon's source ground differs from scoped Terrain")
    # Crystal world.otbm at 32549,32622,11 contains only this top-level ground.
    ground = appearance_only_ground(50343, 150, root)
    if ground != spawn_ground.records(root)[50343]:
        raise ValueError("Skullfrost's source ground differs from published Terrain")
    index = json.loads((root / PLACEMENTS).read_bytes())
    (entry,) = [row for row in index["palette"] if row["source_item_id"] == 50343]
    if entry["key"] != ground["identity"]["key"] or entry["provisional"]:
        raise ValueError("Skullfrost's palette route differs from published Terrain")
    capture = json.loads((root / scoped_spawn_ground.COMPILER_CELLS).read_bytes())
    if {
        "cell": list(scoped_spawn_ground.SLEEPING_DRAGON_CELL),
        "ground": 470,
    } not in capture["points"]:
        raise ValueError("Sleeping Dragon's scoped Terrain placement is missing")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.parse_args()
    check()
