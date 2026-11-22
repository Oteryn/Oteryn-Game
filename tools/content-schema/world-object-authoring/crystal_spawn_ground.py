"""Admit the official, fixed grounds used by the pinned Crystal spawn delta."""

import argparse
import json

from summer_ground import ROOT, official_ground_records

# The selected native cells already use these palette IDs. The retained 15.30
# appearance records supply their bank flags and waypoints speed.
GROUND_SPEEDS = {
    45317: 100,
    **dict.fromkeys(range(45470, 45477), 150),
    **dict.fromkeys(range(48568, 48574), 150),
    50815: 100,
}
SHARD = "content/world/terrain/terrain-08751-08765.json"
PLACEMENTS = "content/world/placements/index.json"
TERRAIN_INDEX = "content/world/terrain/index.json"
TERRAIN_NOTE = " Additionally 15 official appearance-only grounds cover the Crystal 201189f spawn delta."


def outputs(root=ROOT):
    records = official_ground_records(GROUND_SPEEDS, root)
    keys = {r["provenance"]["source_item_id"]: r["identity"]["key"] for r in records}
    placements = json.loads((root / PLACEMENTS).read_bytes())
    selected = [
        entry for entry in placements["palette"] if entry["source_item_id"] in keys
    ]
    if (
        len(selected) != len(keys)
        or {row["source_item_id"] for row in selected} != keys.keys()
    ):
        raise ValueError("Crystal spawn ground palette membership")
    for entry in selected:
        entry.update(key=keys[entry["source_item_id"]], provisional=False)
    terrain_index = json.loads((root / TERRAIN_INDEX).read_bytes())
    if TERRAIN_NOTE not in terrain_index["notes"]:
        terrain_index["notes"] += TERRAIN_NOTE
    return {
        SHARD: json.dumps(
            {"family": "Terrain", "records": records},
            sort_keys=True,
            indent=2,
            ensure_ascii=False,
        )
        + "\n",
        PLACEMENTS: json.dumps(
            placements, sort_keys=True, separators=(",", ":"), ensure_ascii=False
        )
        + "\n",
        TERRAIN_INDEX: json.dumps(terrain_index, indent=2, ensure_ascii=False) + "\n",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if path.read_text() != raw:
                raise SystemExit(f"stale Crystal spawn ground: {name}")
        else:
            path.write_text(raw)


if __name__ == "__main__":
    main()
