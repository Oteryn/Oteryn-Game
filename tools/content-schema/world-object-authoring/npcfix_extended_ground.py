"""Two remaining captured NPC grounds; Item custody is preserved in bound mode."""

import argparse
import copy
import json

import bound_ground
import npcfix_ground
import summer_ground

ROOT = summer_ground.ROOT
IDS = frozenset({32691, 50820})
SHARD = "content/world/terrain/terrain-08618-08619.json"
NOTE = " Additionally two full-anchor NPCFIX grounds; total 8620 Terrain records."


def records(root=ROOT):
    bound = bound_ground.record(root, identifier=32691, expected_speed=150)
    generated = summer_ground.outputs(root, identifier=50820, expected_speed=100,
                                      shard=SHARD, note="")
    return [bound, *json.loads(generated[SHARD])["records"]]


def retarget_palette(index):
    successor = copy.deepcopy(index)
    changes = []
    for identifier in sorted(IDS):
        matches = [row for row in successor["palette"] if row["source_item_id"] == identifier]
        before = {"key": ("oteryn:item.tibia.i32691" if identifier == 32691
                          else "donor:crystalserver@00ce02a5:item/50820"),
                  "provisional": identifier != 32691, "source_item_id": identifier}
        after = {"key": f"oteryn:terrain.tibia.i{identifier}",
                 "provisional": False, "source_item_id": identifier}
        if len(matches) != 1 or matches[0] not in (before, after):
            raise ValueError(f"full-anchor ground palette before drift: {identifier}")
        changes.append((matches[0], after))
    for target, after in changes:
        target.update(after)
    return successor


def outputs(root=ROOT):
    rows = records(root)
    index_path = "content/world/placements/index.json"
    index = retarget_palette(json.loads((root / index_path).read_bytes()))
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((root / marker_path).read_bytes())
    if NOTE not in marker["notes"]:
        marker["notes"] += NOTE
    return {SHARD: npcfix_ground.encode({"family": "Terrain", "records": rows}),
            index_path: npcfix_ground.encode(index),
            marker_path: (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, raw in outputs().items():
        path = ROOT / name
        if args.check:
            if not path.is_file() or path.read_bytes() != raw:
                raise SystemExit("stale full-anchor NPC ground: " + name)
        elif not path.is_file() or path.read_bytes() != raw:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
