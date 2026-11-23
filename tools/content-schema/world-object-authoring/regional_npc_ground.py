"""Qualify the floors under Doubleday and Javala with the retained client appearances."""

import argparse
import json
from functools import lru_cache

from activity_static import REGIONAL_NPC_GROUND_NOTE as NOTE
from activity_static import compose_terrain_notes
from grizzly_ground import ROOT, appearance_only_ground

GROUNDS = {20658: 100, 53254: 150}
SHARD = "content/world/terrain/terrain-08751-08752.json"
PALETTE = "content/world/placements/index.json"
MARKER = "content/world/terrain/index.json"


@lru_cache(maxsize=4)
def records(root=ROOT):
    return [
        appearance_only_ground(item, speed, root) for item, speed in GROUNDS.items()
    ]


def outputs(root=ROOT):
    catalogue = records(root)
    palette = json.loads((root / PALETTE).read_bytes())
    for record in catalogue:
        item = record["provenance"]["source_item_id"]
        (entry,) = [row for row in palette["palette"] if row["source_item_id"] == item]
        entry.update(key=record["identity"]["key"], provisional=False)
    marker = json.loads((root / MARKER).read_bytes())
    marker["notes"] = compose_terrain_notes(marker["notes"].replace(NOTE, "") + NOTE)
    return {
        SHARD: json.dumps(
            {"family": "Terrain", "records": catalogue},
            sort_keys=True,
            separators=(",", ":"),
        )
        + "\n",
        PALETTE: json.dumps(palette, sort_keys=True, separators=(",", ":")) + "\n",
        MARKER: json.dumps(marker, indent=2, ensure_ascii=False) + "\n",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, expected in outputs().items():
        target = ROOT / path
        if args.check:
            if target.read_text() != expected:
                raise SystemExit(f"regional NPC ground drift: {path}")
        else:
            target.write_text(expected)
