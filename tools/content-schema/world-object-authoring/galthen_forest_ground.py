"""Qualify Crystal's ground beneath both ends of the Lost Queen Forest portal."""

import argparse
import json

from activity_static import compose_terrain_notes
from elemental_ground import ROOT, ground_record

SHARD = "content/world/terrain/terrain-08751-08751.json"
NOTE = " Additionally the Lost Queen Forest portal retains its source ground."
ITEM_ID = 31730


def record(root=ROOT):
    # The pinned Crystal appearance loader and the retained official appearance agree.
    return ground_record(root, ITEM_ID, 500, "OTERYN_GALTHEN_FOREST_SOURCE_GROUND/v1")


def output(root=ROOT):
    document = {"family": "Terrain", "records": [record(root)]}
    return (json.dumps(document, indent=2) + "\n").encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    marker_path = ROOT / "content/world/terrain/index.json"
    marker = json.loads(marker_path.read_bytes())
    marker["notes"] = compose_terrain_notes(marker["notes"].replace(NOTE, "") + NOTE)
    generated = {
        ROOT / SHARD: output(),
        marker_path: (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode(),
    }
    for path, raw in generated.items():
        if args.check:
            if not path.is_file() or path.read_bytes() != raw:
                raise SystemExit(f"Lost Queen Forest ground is stale: {path}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
