"""Refresh palette statistics from committed regions after catalogue qualification.

The base map and all other captured source facts remain unchanged. The XML input
must match the digest already recorded by convert_world_base.py.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

import convert_world_base as base
import world_region_codec as codec


def outputs(items_xml: Path, root: Path = base.ROOT) -> dict[str, bytes]:
    path = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
    summary = json.loads((root / path).read_bytes())
    expected = next(
        row["sha256"]
        for row in summary["source"]["files"]
        if row["path"] == base.ITEMS_XML
    )
    xml = items_xml.read_bytes()
    if hashlib.sha256(xml).hexdigest() != expected:
        raise ValueError("palette capture XML differs from the pinned source")
    declared = base.items_xml_ids(xml)
    index = json.loads((root / "content/world/placements/index.json").read_bytes())
    used: Counter[int] = Counter()
    for region in index["regions"]:
        raw = (root / region["path"]).read_bytes()
        if hashlib.sha256(raw).hexdigest() != region["sha256"]:
            raise ValueError(f"palette capture region differs: {region['path']}")
        for _, tiles in codec.decode_region(raw)[3]:
            for _, _, _, _, _, items in tiles:
                used.update(slot for slot, _, _ in items)
    if sum(used.values()) != index["totals"]["items"]:
        raise ValueError("palette capture occurrence count differs from the map")
    families = {
        name: {"entries": 0, "occurrences": 0}
        for name in ("terrain", "world_object", "item")
    }
    provisional = {
        name: {"entries": 0, "occurrences": 0}
        for name in ("appearance_only", "in_items_xml")
    }
    # Same ownership/split rule as convert_world_base.py:1027-1041; recount
    # actual region references rather than adjusting old totals by a constant.
    for slot, row in enumerate(index["palette"]):
        family = base.key_family(row["key"], row["provisional"])
        target = (
            provisional[
                "in_items_xml"
                if row["source_item_id"] in declared
                else "appearance_only"
            ]
            if row["provisional"]
            else families[family]
        )
        target["entries"] += 1
        target["occurrences"] += used[slot]
    summary["palette"] = {
        "entries": len(index["palette"]),
        "families": families,
        "provisional": {
            **provisional,
            **{
                key: sum(row[key] for row in provisional.values())
                for key in ("entries", "occurrences")
            },
        },
    }
    return {path: base.canonical(summary)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--items-xml", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, data in outputs(args.items_xml).items():
        path = base.ROOT / name
        if args.check:
            if path.read_bytes() != data:
                raise ValueError(f"stale palette capture: {name}")
        else:
            path.write_bytes(data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
