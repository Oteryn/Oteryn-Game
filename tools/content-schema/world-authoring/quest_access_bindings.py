"""Restore only the pinned startup selectors consumed by shared quest access.

The normalized capture comes from Crystal201189f's LeverUnique, TileUnique,
TileAction, TeleportAction, TeleportUnique, TilePickAction and KeyDoorAction tables. No placement or item identity is invented.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = "tools/content-schema/world-authoring/samples/quest-access-bindings.json"
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"


def capture_sources(source_root):
    sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
    import lua_tables

    requested = {
        "lever.lua": {
            "LeverUnique": {
                30004,
                30005,
                30008,
                *range(30018, 30024),
                30025,
                30030,
                30034,
            },
            "LeverAction": {30006, 8013, 55021},
        },
        "tile.lua": {
            "TileUnique": {25010, 25031, 25032, *range(25034, 25038)},
            "TileAction": {25019, 25020, 25021, 50022, *range(50308, 50312)},
        },
        "teleport.lua": {
            "TeleportAction": {50022, 55022, 55024},
            "TeleportUnique": {35014, 35017, 35018, 35019, *range(38018, 38023)},
        },
        "chest.lua": {"ChestUnique": {3164}},
        "item.lua": {
            "ItemAction": {8009, 8010, 8011, 8012, 55023},
            "ItemUnique": {3162, 3163},
        },
        "tile_pick.lua": {"TilePickAction": {101}},
        "door_key.lua": {"KeyDoorAction": {4601}},
    }
    exact = {
        ("LeverAction", 30006): {(32148, 32105, 11): 2772},
        ("TilePickAction", 101): {(32149, 32110, 11): 355, (32302, 32257, 8): 355},
        ("KeyDoorAction", 4601): {(32145, 32100, 11): 1631},
    }
    sources, bindings = [], []
    for name, groups in requested.items():
        path = "data-global/startup/tables/" + name
        raw = (source_root / path).read_bytes()
        sources.append(
            {
                "path": path,
                "sha256": hashlib.sha256(raw).hexdigest(),
                "git_blob_sha1": hashlib.sha1(
                    b"blob " + str(len(raw)).encode() + b"\0" + raw
                ).hexdigest(),
            }
        )
        fields = lua_tables.assignments(raw.decode(), set(groups))
        for group, ids in groups.items():
            for field in fields[group]["fields"]:
                key = field["key"]
                if (
                    group == "KeyDoorAction"
                    and isinstance(key, dict)
                    and key.get("expr") == "Storage.Quest.Key.ID4601"
                ):
                    key = 4601
                if not isinstance(key, int) or key not in ids:
                    continue
                value = lua_tables.as_python(field["value"])
                for item in value if isinstance(value, list) else [value]:
                    selector = (group, key)
                    positions = item["itemPos"]
                    for position in (
                        positions if isinstance(positions, list) else [positions]
                    ):
                        cell = tuple(position[k] for k in "xyz")
                        if selector in exact and (
                            cell not in exact[selector]
                            or item["itemId"] not in (False, exact[selector][cell])
                        ):
                            continue
                        bindings.append(
                            {
                                "position": list(cell),
                                "item": item["itemId"] or exact[selector][cell],
                                "attribute": "unique"
                                if group.endswith("Unique")
                                else "action",
                                "value": key,
                                "source": path,
                                "line": field["line"],
                            }
                        )
    # The capture appended TeleportUnique after the original source groups.
    bindings.sort(key=lambda row: row["source"].endswith("/teleport.lua") and row["attribute"] == "unique")
    expected_exact = {
        (cell, appearance)
        for positions in exact.values()
        for cell, appearance in positions.items()
    }
    if not expected_exact <= {
        (tuple(row["position"]), row["item"]) for row in bindings
    }:
        raise ValueError("missing exact source access selector")
    expected = json.loads((ROOT / CAPTURE).read_bytes())
    pinned = {row["path"]: row for row in expected["sources"]}
    item_path = "data-global/startup/tables/item.lua"
    pinned[item_path] = {
        "path": item_path,
        "sha256": "7f551a398529e18ccad1ba56d740a4961874bbddd4736dd2df00272ddf6ea6ad",
        "git_blob_sha1": "5d5323401d243df8416a38aba33c3d6e738b2d1a",
    }
    chest_path = "data-global/startup/tables/chest.lua"
    pinned[chest_path] = {
        "path": chest_path,
        "sha256": "7c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4",
        "git_blob_sha1": "7b9f823542f9b8d5f99fb6960abe070324c89b8e",
    }
    if {row["path"]: row for row in sources} != pinned:
        raise ValueError("source tables differ from pinned Crystal201189f blobs")
    return {
        "schema": 1,
        "repository": "zimbadev/crystalserver",
        "revision": "201189f871d23597b82caf88fd611454a2ddefb8",
        "evidence_class": "OTS_HYPOTHESIS_ONLY",
        "sources": sources,
        "bindings": bindings,
    }


def canonical(value):
    return (json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n").encode()


def overlay_region(raw, palette, bindings):
    z, rx, ry, table = codec.parse_region(raw)
    selected = {
        tuple(row["position"]): row
        for row in bindings
        if row["position"][2] == z
        and row["position"][0] // 256 == rx
        and row["position"][1] // 256 == ry
    }
    if not selected:
        return raw, {}
    sectors, imported = {}, {}
    changed = False
    for local, offset, length in table:
        payload = codec.zstandard.ZstdDecompressor().decompress(
            raw[offset : offset + length], max_output_size=codec.MAX_SECTOR_BYTES
        )
        sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
        tiles = codec.decode_sector(payload, sx, sy)
        sector_changed = False
        for x, y, _, house, _, items in tiles:
            row = selected.get((x, y, z))
            if row is None:
                continue
            slots = [
                slot
                for slot, (index, depth, _) in enumerate(items)
                if depth == 0 and palette[index]["source_item_id"] == row["item"]
            ]
            if house or len(slots) != 1:
                raise ValueError(f"quest access placement ambiguity: {row}")
            slot = slots[0]
            index, depth, attrs = items[slot]
            attribute, value = row["attribute"], row["value"]
            if (attrs or {}).get(attribute, value) != value:
                raise ValueError(f"quest access selector conflict: {row}")
            imported[(x, y, z)] = attribute
            replacement = {**(attrs or {}), attribute: value}
            if attrs != replacement:
                items[slot] = (index, depth, replacement)
                sector_changed = True
        sectors[local] = (
            codec.encode_sector_tiles(sx, sy, tiles) if sector_changed else payload
        )
        changed |= sector_changed
    if imported.keys() != selected.keys():
        raise ValueError(
            f"missing quest access tiles: {selected.keys() - imported.keys()}"
        )
    return (codec.encode_region(z, rx, ry, sectors) if changed else raw), imported


def overlay_outputs(
    outputs, capture_bytes=None, *, capture_path=CAPTURE, source_name="quest_access"
):
    if capture_bytes is None:
        capture_bytes = (ROOT / CAPTURE).read_bytes()
    capture = json.loads(capture_bytes)
    index = json.loads(outputs[INDEX])
    imported, byte_delta = {}, 0
    for region in index["regions"]:
        path = region["path"]
        previous = outputs[path]
        raw, rows = overlay_region(previous, index["palette"], capture["bindings"])
        imported.update(rows)
        outputs[path] = raw
        byte_delta += len(raw) - len(previous)
        region["sha256"] = hashlib.sha256(raw).hexdigest()
    if len(imported) != len(capture["bindings"]):
        missing = [
            row for row in capture["bindings"] if tuple(row["position"]) not in imported
        ]
        raise ValueError(f"quest access capture did not bind completely: {missing}")
    counts = {
        name: sum(value == name for value in imported.values())
        for name in ("unique", "action")
    }
    previous = index["source"].get(source_name, {}).get("attributes", {})
    index["source"][source_name] = {
        "path": capture_path,
        "sha256": hashlib.sha256(capture_bytes).hexdigest(),
        "attributes": counts,
    }
    outputs[INDEX] = canonical(index)
    summary = json.loads(outputs[SUMMARY])
    summary["source"] = index["source"]
    for name, count in counts.items():
        total = summary["item_attributes"].get(name, 0) + count - previous.get(name, 0)
        if total:
            summary["item_attributes"][name] = total
        else:
            summary["item_attributes"].pop(name, None)
    summary["bytes_on_disk"] += byte_delta
    outputs[SUMMARY] = canonical(summary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--crystal-root", type=Path)
    args = parser.parse_args()
    if args.crystal_root:
        capture = (
            json.dumps(capture_sources(args.crystal_root), indent=2) + "\n"
        ).encode()
        if args.check:
            if (ROOT / CAPTURE).read_bytes() != capture:
                raise ValueError("source capture drift")
        else:
            (ROOT / CAPTURE).write_bytes(capture)
    index = json.loads((ROOT / INDEX).read_bytes())
    paths = [INDEX, SUMMARY] + [region["path"] for region in index["regions"]]
    outputs = {path: (ROOT / path).read_bytes() for path in paths}
    overlay_outputs(outputs)
    for path, raw in outputs.items():
        if args.check:
            if (ROOT / path).read_bytes() != raw:
                raise ValueError(f"stale quest access selectors: {path}")
        elif (ROOT / path).read_bytes() != raw:
            (ROOT / path).write_bytes(raw)


if __name__ == "__main__":
    main()
