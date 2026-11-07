"""Restore only17pinned Spike action stamps on existing source objects.

The map contains the items; startup/tables/item.lua assigns their action ids later.
Use --apply [--check] on committed regions; the base converter calls overlay_outputs.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = "tools/content-schema/world-authoring/samples/spike-portal-bindings.json"
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"


def canonical(value):
    return (json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n").encode()


def capture(path):
    sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
    import lua_tables
    text = path.read_text()
    digest = hashlib.sha256(text.encode()).hexdigest()
    if digest != "7f551a398529e18ccad1ba56d740a4961874bbddd4736dd2df00272ddf6ea6ad":
        raise ValueError("Spike source differs from pinned Crystal201")
    table = lua_tables.assignments(text, {"ItemAction"})["ItemAction"]
    bindings=[]
    for field in table["fields"]:
        if field["key"] not in range(4227,4244): continue
        value=lua_tables.as_python(field["value"])
        positions=value["itemPos"]
        for pos in positions if isinstance(positions,list) else [positions]:
            bindings.append({"position":[pos[k] for k in "xyz"],"item":value["itemId"],"action":field["key"],"source_line":field["line"]})
    if len(bindings)!=17: raise ValueError("exact17Spike sourceportal bindings required")
    return {"withheld":[{"action":4226,"reason":"Source premium admission requires independent real Premium owner; no matching admitted production callback, do not stamp."}],"schema":1,"repository":"zimbadev/crystalserver","revision":"201189f871d23597b82caf88fd611454a2ddefb8","source":"data-global/startup/tables/item.lua","sha256":digest,"rules":["data-global/scripts/quests/the_spike_tasks/movements_spike_teleport.lua:1-58"],"bindings":sorted(bindings,key=lambda row:row["position"])}


def overlay_outputs(outputs, require_complete=False):
    capture = (ROOT / CAPTURE).read_bytes()
    document = json.loads(capture)
    handlers=json.loads((ROOT/"content/quests/world-callbacks.json").read_bytes())["callbacks"]
    for row in document["bindings"]:
        matches=[c for c in handlers if c["key"]==f"oteryn:quest-callback/spike-portal-{row['action']}" and c["trigger"]=="step_in" and c["selector"].get("action")==row["action"] and c.get("teleport") and not c.get("storage") and not c.get("offerings")]
        if len(matches)!=1: raise ValueError(f"Spike source action has no exact admitted handler: {row['action']}")
    index = json.loads(outputs[INDEX])
    by_region = {}
    for row in document["bindings"]:
        x, y, z = row["position"]
        path = "content/world/placements/" + codec.region_name(z, x // 256, y // 256)
        by_region.setdefault(path, {})[(x, y)] = row
    imported = {"unique": 0, "action": 0}
    byte_delta = 0
    for region in index["regions"]:
        path = region["path"]
        bindings = by_region.get(path)
        if not bindings:
            continue
        previous = outputs[path]
        z, rx, ry, sectors = codec.decode_region(previous)
        encoded = {}
        matched = set()
        changed = False
        for local, tiles in sectors:
            for x, y, _, house, _, items in tiles:
                row = bindings.get((x, y))
                if row is None:
                    continue
                attribute = "unique" if "unique" in row else "action"
                matches = [
                    (i, attrs)
                    for i, (palette, depth, attrs) in enumerate(items)
                    if depth == 0
                    and index["palette"][palette]["source_item_id"] == row["item"]
                ]
                if house or len(matches) != 1:
                    raise ValueError(f"Spike portal item mismatch at {(x, y, z)}")
                slot, attrs = matches[0]
                if (attrs or {}).get(attribute, row[attribute]) != row[attribute]:
                    raise ValueError(f"Spike portal stamp conflict at {(x, y, z)}")
                replacement = {**(attrs or {}), attribute: row[attribute]}
                changed |= replacement != attrs
                items[slot] = (items[slot][0], 0, replacement)
                matched.add((x, y))
                imported[attribute] += 1
            encoded[local] = codec.encode_sector_tiles(
                rx * 8 + local % 8, ry * 8 + local // 8, tiles
            )
        if require_complete and matched != bindings.keys():
            raise ValueError(f"missing Spike portal positions: {bindings.keys() - matched}")
        raw = codec.encode_region(z, rx, ry, encoded) if changed else previous
        outputs[path] = raw
        byte_delta += len(raw) - len(previous)
        region["sha256"] = hashlib.sha256(raw).hexdigest()
    if require_complete and sum(imported.values()) != len(document["bindings"]):
        raise ValueError("missing Spike portal region")
    before = index["source"].get("spike_portals", {}).get("imported", {})
    index["source"]["spike_portals"] = {
        "path": CAPTURE,
        "sha256": hashlib.sha256(capture).hexdigest(),
        "imported": imported,
    }
    outputs[INDEX] = canonical(index)
    summary = json.loads(outputs[SUMMARY])
    summary["source"] = index["source"]
    for attribute, count in imported.items():
        if count != before.get(attribute, 0):
            summary["item_attributes"][attribute] = (
                summary["item_attributes"].get(attribute, 0)
                + count
                - before.get(attribute, 0)
            )
    summary["bytes_on_disk"] += byte_delta
    outputs[SUMMARY] = canonical(summary)


def apply(check):
    index = json.loads((ROOT / INDEX).read_bytes())
    outputs = {
        path: (ROOT / path).read_bytes()
        for path in [INDEX, SUMMARY, *(r["path"] for r in index["regions"])]
    }
    overlay_outputs(outputs, require_complete=True)
    for path, data in outputs.items():
        if (ROOT / path).read_bytes() != data:
            if check:
                raise ValueError(f"SPIKE_PORTAL_BINDING_DRIFT: {path}")
            (ROOT / path).write_bytes(data)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--crystal-items", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.crystal_items:
        data = (json.dumps(capture(args.crystal_items), indent=2) + "\n").encode()
        if args.check:
            if (ROOT / CAPTURE).read_bytes() != data:
                raise ValueError("SPIKE_PORTAL_SOURCE_DRIFT")
        else:
            (ROOT / CAPTURE).write_bytes(data)
    if args.apply:
        apply(args.check)
