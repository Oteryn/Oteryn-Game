"""Restore only the pinned startup selectors consumed by shared quest access.

The normalized capture comes from Crystal201189f's LeverUnique, TileUnique,
TileAction and TeleportAction tables. No placement or item identity is invented.
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
        "lever.lua": {"LeverUnique": {30004, 30005, 30025, 30030, 30034}},
        "tile.lua": {"TileUnique": {25010}, "TileAction": {50022}},
        "teleport.lua": {"TeleportAction": {50022}},
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
                if not isinstance(field["key"], int) or field["key"] not in ids:
                    continue
                value = lua_tables.as_python(field["value"])
                for item in value if isinstance(value, list) else [value]:
                    positions = item["itemPos"]
                    for position in (
                        positions if isinstance(positions, list) else [positions]
                    ):
                        bindings.append(
                            {
                                "position": [position[k] for k in "xyz"],
                                "item": item["itemId"],
                                "attribute": "unique"
                                if group.endswith("Unique")
                                else "action",
                                "value": field["key"],
                                "source": path,
                                "line": field["line"],
                            }
                        )
    expected = json.loads((ROOT / CAPTURE).read_bytes())
    if sources != expected["sources"]:
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
