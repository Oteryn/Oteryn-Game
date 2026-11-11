"""Refresh palette statistics from committed regions after catalogue qualification.

The base map and donor-only captured facts remain unchanged. With --summary,
recount all region facts after merging independent overlays. The XML input must
match the digest already recorded by convert_world_base.py.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

import convert_world_base as base
import validate_world_base as validate
import world_region_codec as codec


def outputs(
    items_xml: Path,
    root: Path = base.ROOT,
    *,
    refresh_summary=False,
    refresh_palette=False,
) -> dict[str, bytes]:
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
    totals: Counter[str] = Counter()
    floors: Counter[int] = Counter()
    attributes: Counter[str] = Counter()
    if refresh_summary:
        if index["source"]["files"] != summary["source"]["files"]:
            raise ValueError("summary capture donor input pins differ from the index")
        validate._init(
            str(root),
            (summary["map"]["width"], summary["map"]["height"]),
            len(index["palette"]),
        )
    for region in index["regions"]:
        if refresh_summary:
            result = validate.check_region(region)
            if result["errors"]:
                raise ValueError("summary capture: " + "; ".join(result["errors"]))
            used.update(result["palette"])
            attributes.update(result["attributes"])
            floors[result["z"]] += result["tiles"]
            totals.update(
                {
                    key: result[key]
                    for key in ("sectors", "tiles", "items", "houses", "zones")
                }
            )
            totals.update(regions=1, bytes=result["size"])
            continue
        raw = (root / region["path"]).read_bytes()
        if hashlib.sha256(raw).hexdigest() != region["sha256"]:
            raise ValueError(f"palette capture region differs: {region['path']}")
        for _, tiles in codec.decode_region(raw)[3]:
            for _, _, _, _, _, items in tiles:
                used.update(slot for slot, _, _ in items)
    if sum(used.values()) != index["totals"]["items"]:
        raise ValueError("palette capture occurrence count differs from the map")
    generated = {}
    if refresh_palette:
        bindings_path = root / "imports/crystalserver/bindings/items.json"
        bindings = bindings_path.read_bytes()
        if hashlib.sha256(bindings).hexdigest() != index["item_bindings"]["sha256"]:
            raise ValueError("palette capture Item binding pin differs")
        occurrences = Counter(
            {
                row["source_item_id"]: used[slot]
                for slot, row in enumerate(index["palette"])
                if used[slot]
            }
        )
        index["palette"] = base.build_palette(
            base.bound_keys(bindings, base.defined_item_keys(root)),
            occurrences,
            previous=index["palette"],
            terrain=base.terrain_keys(root),
            world_object=base.world_object_keys(root),
        )
        generated["content/world/placements/index.json"] = base.canonical(index)
    if refresh_summary:
        counted = {key: totals[key] for key in sorted(validate.TOTAL_KEYS)}
        if counted != index["totals"]:
            raise ValueError("summary capture totals differ from the index")
        summary["source"] = index["source"]
        summary["totals"] = counted
        summary["bytes_on_disk"] = totals["bytes"]
        summary["tiles_with_house"] = totals["houses"]
        summary["tiles_with_zone"] = totals["zones"]
        summary["tiles_by_floor"] = {
            str(z): count for z, count in sorted(floors.items())
        }
        attributes.pop("depth", None)
        summary["item_attributes"] = dict(sorted(attributes.items()))
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
    return generated | {path: base.canonical(summary)}


def binding_pin_outputs(root=base.ROOT):
    """Refresh provenance only after proving the exact approved additive binding."""
    import sys

    sys.path.insert(0, str(root / "tools/content-schema/native-gameplay"))
    from build_spell_item_identities import historical_binding_bytes

    name = "content/world/placements/index.json"
    index = json.loads((root / name).read_bytes())
    descriptor = index["item_bindings"]
    raw = (root / descriptor["path"]).read_bytes()
    historical_binding_bytes(raw, descriptor["sha256"])
    descriptor["sha256"] = hashlib.sha256(raw).hexdigest()
    return {name: base.canonical(index)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--items-xml", type=Path)
    parser.add_argument("--binding-pin", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument(
        "--palette",
        action="store_true",
        help="refresh stable palette entries from current Item and World catalogues",
    )
    parser.add_argument(
        "--summary",
        action="store_true",
        help="recount merged region facts and copy their source provenance; retain donor-only facts",
    )
    args = parser.parse_args()
    if args.binding_pin:
        if args.items_xml or args.summary or args.palette:
            parser.error("--binding-pin is a provenance-only operation")
        generated = binding_pin_outputs()
    else:
        if args.items_xml is None:
            parser.error("--items-xml is required for palette/summary recapture")
        generated = outputs(
            args.items_xml, refresh_summary=args.summary, refresh_palette=args.palette
        )
    for name, data in generated.items():
        path = base.ROOT / name
        if args.check:
            if path.read_bytes() != data:
                raise ValueError(f"stale palette capture: {name}")
        else:
            path.write_bytes(data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
