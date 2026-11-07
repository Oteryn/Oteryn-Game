"""Admit reviewed NPC ground through the strict official appearance-only producer."""

import argparse
import copy
import json
from pathlib import Path

import summer_ground

ROOT = Path(__file__).resolve().parents[3]
SPECS = {35700: 150, 44783: 150, 53254: 150}
IDS = frozenset(SPECS)
SHARD = "content/world/terrain/terrain-08615-08617.json"
NOTE = " Additionally three NPCFIX reviewed appearance-only grounds; total 8618 Terrain records."


def records(root=ROOT):
    """Keep the existing artifact, membership, bank, speed and no-Item-binding guards."""
    rows = []
    for identifier, speed in sorted(SPECS.items()):
        generated = summer_ground.outputs(
            root, identifier=identifier, expected_speed=speed, shard=SHARD, note=""
        )
        rows.extend(json.loads(generated[SHARD])["records"])
    return rows


def retarget_palette(index, rows):
    """Validate every before-state before returning a separate successor index."""
    identifiers = [row["provenance"]["source_item_id"] for row in rows]
    if len(identifiers) != len(IDS) or set(identifiers) != IDS:
        raise ValueError("reviewed ground identities drifted")
    successor = copy.deepcopy(index)
    changes = []
    for identifier in sorted(IDS):
        matches = [
            row for row in successor["palette"]
            if row["source_item_id"] == identifier
        ]
        before = {
            "key": f"donor:crystalserver@00ce02a5:item/{identifier}",
            "provisional": True,
            "source_item_id": identifier,
        }
        after = {
            "key": f"oteryn:terrain.tibia.i{identifier}",
            "provisional": False,
            "source_item_id": identifier,
        }
        if len(matches) != 1 or matches[0] not in (before, after):
            raise ValueError(f"reviewed ground palette before drift: {identifier}")
        changes.append((matches[0], after))
    for target, after in changes:
        target.update(after)
    return successor


def encode(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":"),
                       ensure_ascii=False) + "\n").encode()


def outputs(root=ROOT):
    rows = records(root)
    index_path = "content/world/placements/index.json"
    index = retarget_palette(json.loads((root / index_path).read_bytes()), rows)
    marker_path = "content/world/terrain/index.json"
    marker = json.loads((root / marker_path).read_bytes())
    if NOTE not in marker["notes"]:
        marker["notes"] += NOTE
    return {
        SHARD: encode({"family": "Terrain", "records": rows}),
        index_path: encode(index),
        marker_path: (json.dumps(marker, indent=2, ensure_ascii=False) + "\n").encode(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = outputs()
    for name, raw in generated.items():
        path = ROOT / name
        if args.check:
            if not path.is_file() or path.read_bytes() != raw:
                raise SystemExit("stale reviewed NPC ground: " + name)
        elif not path.is_file() or path.read_bytes() != raw:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
