#!/usr/bin/env python3
"""Extract the exact Crystal Soulpit room and pinned Canary loader facts.

The original OTBM is reference input only and is never retained in the repository.
The generated capture is inert normalized coordinates/Item identities, not a map grant.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

from build_source_world import bool_xml, policies

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "world-authoring"))
import otbm_reader

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "tools/content-schema/native-gameplay/crystal-soulpit-room.json"
PIN = ROOT / "crates/gameplay/world/src/map/soulpit_capture_pin.rs"
CRYSTAL = "201189f871d23597b82caf88fd611454a2ddefb8"
MAP_SHA256 = "c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85"
MAP_GIT_BLOB = "e61658ec0ab7b70ec35025ae969426d6fc57ae75"
CANARY = "47dfd51f45280a59a1d3e50ba7edd573d7234446"
INPUTS = {
    "data/items/appearances.dat": "aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50",
    "data/items/items.xml": "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2",
}


def generate(source_map, source_root):
    raw = source_map.read_bytes()
    if hashlib.sha256(raw).hexdigest() != MAP_SHA256:
        raise ValueError("Crystal Soulpit map digest mismatch")
    if hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest() != MAP_GIT_BLOB:
        raise ValueError("Crystal Soulpit map git blob mismatch")
    rows = []

    def selected(x, y, z, flags, house, zones, items):
        in_arena = 32362 <= x <= 32390 and 31132 <= y <= 31153
        in_staging = x == 32375 and 31158 <= y <= 31162
        is_return = (x, y) == (32373, 31158)
        if z != 8 or not (in_arena or in_staging or is_return):
            return
        # These exact source cells carry neither item attributes nor nested contents.
        # Fail if a later source cut needs a wider normalization rather than dropping data.
        if house or zones or any(depth != 0 or attrs for _, depth, attrs in items):
            raise ValueError("unsupported Soulpit source placement attributes")
        rows.append({"x": x, "y": y, "floor": z, "flags": flags or 0,
                     "items": [item for item, _, _ in items]})

    otbm_reader.read_tiles(raw, selected)
    rows.sort(key=lambda row: (row["floor"], row["y"], row["x"]))
    ids = {item for row in rows for item in row["items"]}
    inputs = {}
    for name, expected in INPUTS.items():
        inputs[name] = (source_root / name).read_bytes()
        if hashlib.sha256(inputs[name]).hexdigest() != expected:
            raise ValueError(f"Canary loader input digest mismatch: {name}")
    palette = []
    for policy in policies(inputs["data/items/appearances.dat"], inputs["data/items/items.xml"], ids):
        for attribute in policy["xml_attributes"]:
            if attribute.get("key", "").lower() == "blocking":
                policy["block_solid"] = bool_xml(attribute.get("value", ""))
        palette.append({"item": policy["server_item_id"], "ground": policy["ground"],
                        "ground_speed": policy["ground_speed"], "solid": policy["block_solid"],
                        "projectile": policy["block_projectile"], "pickupable": policy["pickupable"],
                        "door": policy["dynamic_kind"] == "door"})
    document = {"schema": "OTERYN_SOULPIT_ROOM_CAPTURE/v2", "evidence_class": "OTS_HYPOTHESIS_ONLY",
                "source_repository": "zimbadev/crystalserver", "source_revision": CRYSTAL,
                "source_path": "data-global/world/world.otbm", "source_git_blob": MAP_GIT_BLOB,
                "source_sha256": MAP_SHA256, "layout_source": "data-global/lib/others/soulpit.lua:31-58,138-140,178-179",
                "loader_repository": "opentibiabr/canary", "loader_revision": CANARY,
                "loader_inputs": INPUTS,
                "palette_columns": ["item", "ground", "ground_speed", "solid", "projectile", "pickupable", "door"],
                "tile_columns": ["x", "y", "floor", "flags", "items"]}
    # Stable metadata ordering and one compact source record per line. Column names
    # remain explicit, while hundreds of repeated property names are omitted.
    lines = ["{"]
    for key, value in sorted(document.items()):
        lines.append("  " + json.dumps(key) + ":" + json.dumps(value, separators=(",", ":")) + ",")
    for key, columns, records in (("palette", document["palette_columns"], palette),
                                  ("tiles", document["tile_columns"], rows)):
        lines.append("  " + json.dumps(key) + ":[")
        for index, record in enumerate(records):
            suffix = "," if index + 1 < len(records) else ""
            lines.append("    " + json.dumps([record[column] for column in columns], separators=(",", ":")) + suffix)
        lines.append("  ]" + ("," if key == "palette" else ""))
    lines.append("}")
    result = ("\n".join(lines) + "\n").encode()
    digest = hashlib.sha256(result).hexdigest()
    pin = ("// Generated by tools/content-schema/native-gameplay/build_soulpit_room.py.\n"
           f'pub(super) const CAPTURE_SHA256: &str =\n    "{digest}";\n')
    return result, pin


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-map", type=Path, required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data, pin = generate(args.source_map, args.source_root)
    if args.check:
        if OUTPUT.read_bytes() != data or PIN.read_text() != pin:
            raise SystemExit("Soulpit source capture is stale")
    else:
        OUTPUT.write_bytes(data)
        PIN.write_text(pin)
    print(f"Soulpit source capture: {len(json.loads(data)['tiles'])} source tiles; sha256={hashlib.sha256(data).hexdigest()}")


if __name__ == "__main__":
    main()
