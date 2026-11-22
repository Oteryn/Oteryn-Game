"""Ground routes at the Crystal 201189f8 Sleeping Dragon and Skullfrost contacts."""

import argparse
import json

from elemental_ground import ROOT, routed_ground
from grizzly_ground import appearance_ground

DRAGON = "content/world/terrain/terrain-08751-08751.json"
SKULLFROST = "content/world/terrain/terrain-08752-08752.json"
PLACEMENTS = "content/world/placements/index.json"
NOTE = " Additionally source ground 470 at the Sleeping Dragon and 50343 at Skullfrost."


def outputs(root=ROOT):
    dragon = routed_ground(470, 95, "OTERYN_SLEEPING_DRAGON_SOURCE_GROUND/v1", root)
    # Crystal world.otbm at 32549,32622,11 contains only this top-level ground.
    ground = appearance_ground(50343, 150, root)
    if ground["walkable"]["value"] is not True:
        raise ValueError("Skullfrost's source appearance is no longer walkable")
    index = json.loads((root / PLACEMENTS).read_bytes())
    (entry,) = [row for row in index["palette"] if row["source_item_id"] == 50343]
    entry.update(key=ground["identity"]["key"], provisional=False)
    return {
        DRAGON: dragon,
        SKULLFROST: (json.dumps({"family": "Terrain", "records": [ground]}, indent=2) + "\n").encode(),
        PLACEMENTS: (json.dumps(index, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode(),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    marker = "content/world/terrain/index.json"
    index = json.loads((ROOT / marker).read_bytes())
    if NOTE not in index["notes"]:
        index["notes"] += NOTE
    files = outputs()
    files[marker] = (json.dumps(index, indent=2, ensure_ascii=False) + "\n").encode()
    for name, raw in files.items():
        path = ROOT / name
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit(f"Crystal NPC ground drift: {name}")
        else:
            path.write_bytes(raw)
