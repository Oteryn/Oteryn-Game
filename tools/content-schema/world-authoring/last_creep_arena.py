"""Add M23's explicitly authored arena through the existing B3 World owner.

No donor coordinate or teleport destination is invented. Portal appearances carry
NO raw teleport attribute; existing quest STEP callbacks own their transport.
"""

import argparse
import hashlib
import json
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
INDEX = "content/world/placements/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
DESCRIPTOR = "content/quests/last-creep-native.json"
ARENA = "content/world/placements/region-z15-x080-y080.b3"
SOURCE = "last_creep_native"


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def native(position):
    return position["x"], position["y"], -position["floor"]


def region_path(position):
    x, y, z = native(position)
    return "content/world/placements/" + codec.region_name(z, x // 256, y // 256)


def layout(document):
    tiles = {}
    for left, top, right, bottom in [
        (20481, 20481, 20509, 20495),
        (20482, 20503, 20488, 20509),
    ]:
        for y in range(top, bottom + 1):
            for x in range(left, right + 1):
                tiles[x, y, 15] = [document["ground"]]
    for form in document["forms"]:
        tiles[native(form["marker"])].append(form["appearance"])
        tiles[native(form["portal"])].append(document["portal_appearance"])
    for name in ["return_portal", "exit_portal"]:
        tiles[native(document[name])].append(document["portal_appearance"])
    return tiles


def _read(outputs, root, path):
    return outputs[path] if path in outputs else (root / path).read_bytes()


def _append_marker(raw, position, palette_index, palette):
    z, rx, ry, table = codec.parse_region(raw)
    x, y, expected_z = native(position)
    if (z, rx, ry) != (expected_z, x // 256, y // 256):
        raise ValueError("M23_MARKER_WRONG_REGION")
    found = False
    sectors = {}
    for local, offset, length in table:
        if (rx * 8 + local % 8, ry * 8 + local // 8) != (x // 32, y // 32):
            sectors[local] = codec.zstandard.ZstdDecompressor().decompress(
                raw[offset : offset + length], max_output_size=codec.MAX_SECTOR_BYTES
            )
            continue
        sx, sy = x // 32, y // 32
        payload = codec.zstandard.ZstdDecompressor().decompress(
            raw[offset : offset + length], max_output_size=codec.MAX_SECTOR_BYTES
        )
        tiles = codec.decode_sector(payload, sx, sy)
        changed = []
        for tx, ty, flags, house, zones, items in tiles:
            if (tx, ty) == (x, y):
                if (
                    house
                    or zones
                    or len(items) != 1
                    or items[0][1] != 0
                    or items[0][2] is not None
                    or palette[items[0][0]]["source_item_id"] != 231
                ):
                    raise ValueError(
                        "M23_START_REQUIRES_UNTOUCHED_PLAIN_NONHOUSE_SOURCE_GROUND"
                    )
                items = [*items, (palette_index, 0, None)]
                found = True
            changed.append((tx, ty, flags, house, zones, items))
        sectors[local] = codec.encode_sector_tiles(sx, sy, changed)
    if not found:
        raise ValueError("M23_START_TILE_ABSENT")
    return codec.encode_region(z, rx, ry, sectors)


def require_plain_exit(outputs, root, document, palette):
    position = document["outside"]
    at = native(position)
    raw = _read(outputs, root, region_path(position))
    for _, tiles in codec.decode_region(raw)[3]:
        for x, y, flags, house, zones, items in tiles:
            if (x, y, -position["floor"]) != at:
                continue
            if (
                house
                or zones
                or len(items) != 1
                or items[0][1:] != (0, None)
                or palette[items[0][0]]["source_item_id"] != 231
            ):
                raise ValueError("M23_EXIT_REQUIRES_EXACT_PLAIN_NONHOUSE_GROUND_231")
            return
    raise ValueError("M23_EXIT_SOURCE_TILE_ABSENT")


def overlay_outputs(outputs, root=ROOT, descriptor_bytes=None):
    raw_descriptor = (
        (root / DESCRIPTOR).read_bytes()
        if descriptor_bytes is None
        else descriptor_bytes
    )
    document = json.loads(raw_descriptor)
    if (
        document["basis"] != "CHOSEN_OTERYN_APPROXIMATION"
        or document["source_equivalence"] is not False
    ):
        raise ValueError("M23_AUTHORSHIP_NOT_EXPLICIT")
    index = json.loads(_read(outputs, root, INDEX))
    summary = json.loads(_read(outputs, root, SUMMARY))
    if index["source"].get(SOURCE):
        if index["source"][SOURCE]["sha256"] != digest(raw_descriptor):
            raise ValueError("M23_AUTHORED_OVERLAY_DESCRIPTOR_CHANGED_REQUIRES_REBUILD")
        # Idempotence never overwrites a peer's bytes. Validate own actual layout instead.
        validate_overlay(outputs, root, document, index)
        return
    paths = {r["path"] for r in index["regions"]}
    if ARENA in paths:
        raise ValueError("M23_ARENA_REGION_COLLIDES_WITH_EXISTING_WORLD")
    bounds = json.loads(
        (root / "content/world/worlds/worlds-00000-00000.json").read_bytes()
    )["records"][0]["declaration"]["bounds"]
    positions = list(layout(document)) + [
        native(document[name]) for name in ["start", "outside"]
    ]
    if any(
        not (
            bounds["min_x"] <= x < bounds["max_x_exclusive"]
            and bounds["min_y"] <= y < bounds["max_y_exclusive"]
            and 0 <= z <= 15
        )
        for x, y, z in positions
    ):
        raise ValueError("M23_AUTHORSHIP_OUTSIDE_WORLD_ENVELOPE")
    palette = index["palette"]
    require_plain_exit(outputs, root, document, palette)
    lookup = {row["source_item_id"]: i for i, row in enumerate(palette)}
    ids = {item for items in layout(document).values() for item in items} | {
        document["start_appearance"],
        document["portal_appearance"],
    }
    new_entries = 0
    for item in sorted(ids):
        if item not in lookup:
            lookup[item] = len(palette)
            palette.append(
                {
                    "key": f"oteryn:item.tibia.i{item}",
                    "provisional": False,
                    "source_item_id": item,
                }
            )
            new_entries += 1
        else:
            row = palette[lookup[item]]
            if row["provisional"] or row["key"] != f"oteryn:item.tibia.i{item}":
                raise ValueError("M23_SOURCE_IDENTITY_PALETTE_CONFLICT")
            row.pop("retired", None)
    arena = layout(document)
    tiles = [
        (x, y, 1, 0, (), [(lookup[item], 0, None) for item in items])
        for (x, y, z), items in sorted(arena.items(), key=lambda r: (r[0][1], r[0][0]))
    ]
    arena_raw = codec.encode_region(
        15, 80, 80, {0: codec.encode_sector_tiles(640, 640, tiles)}
    )
    added_items = sum(map(len, arena.values())) + 1
    rows = {r["path"]: r for r in index["regions"]}
    byte_delta = len(arena_raw)
    for name, item in [("start", document["start_appearance"])]:
        path = region_path(document[name])
        previous = _read(outputs, root, path)
        after = _append_marker(previous, document[name], lookup[item], palette)
        outputs[path] = after
        byte_delta += len(after) - len(previous)
        row = rows[path]
        row["sha256"] = digest(after)
        row["items"] += 1
    outputs[ARENA] = arena_raw
    rows[ARENA] = {
        "path": ARENA,
        "sha256": digest(arena_raw),
        "tiles": len(arena),
        "items": added_items - 1,
    }
    index["regions"] = [rows[path] for path in sorted(rows)]
    index["shards"] = [row["path"] for row in index["regions"]]
    index["totals"]["regions"] += 1
    index["totals"]["sectors"] += 1
    index["totals"]["tiles"] += len(arena)
    index["totals"]["items"] += added_items
    proof = {
        "basis": document["basis"],
        "path": DESCRIPTOR,
        "sha256": digest(raw_descriptor),
        "source_equivalence": False,
        "raw_teleports_added": 0,
        "tiles_added": len(arena),
        "items_added": added_items,
        "region": ARENA,
        "source_start_tile_policy": "APPEND_TO_EXACT_PLAIN_231_NONHOUSE_ONLY",
    }
    index["source"][SOURCE] = proof
    summary["source"] = index["source"]
    summary["totals"] = index["totals"].copy()
    summary["bytes_on_disk"] += byte_delta
    summary["tiles_by_floor"]["15"] = summary["tiles_by_floor"].get("15", 0) + len(
        arena
    )
    summary["palette"]["entries"] += new_entries
    summary["palette"]["families"]["item"]["entries"] += new_entries
    summary["palette"]["families"]["item"]["occurrences"] += added_items
    outputs[INDEX] = canonical(index)
    outputs[SUMMARY] = canonical(summary)
    validate_overlay(outputs, root, document, index)


def validate_overlay(outputs, root, document, index):
    require_plain_exit(outputs, root, document, index["palette"])
    expected = layout(document)
    found = {}
    z, _rx, _ry, sectors = codec.decode_region(_read(outputs, root, ARENA))
    for _, tiles in sectors:
        for x, y, flags, house, zones, items in tiles:
            if (
                house
                or zones
                or flags != 1
                or any(depth or attrs is not None for _, depth, attrs in items)
            ):
                raise ValueError("M23_ARENA_UNEXPECTED_ATTRIBUTES_OR_RAW_TELEPORT")
            found[x, y, z] = [
                index["palette"][i]["source_item_id"] for i, _, _ in items
            ]
    if found != expected:
        raise ValueError("M23_ARENA_EXACT_LAYOUT_DRIFT")
    for name, appearance in [("start", document["start_appearance"])]:
        at = native(document[name])
        matched = []
        for _, tiles in codec.decode_region(
            _read(outputs, root, region_path(document[name]))
        )[3]:
            for x, y, flags, house, zones, items in tiles:
                if (x, y, -document[name]["floor"]) == at:
                    matched = items
        if (
            len(matched) != 2
            or matched[1][1:] != (0, None)
            or index["palette"][matched[1][0]]["source_item_id"] != appearance
        ):
            raise ValueError("M23_START_EXACT_BARE_NATIVE_SELECTOR_DRIFT")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    outputs = {}
    overlay_outputs(outputs, args.root)
    for path, raw in outputs.items():
        target = args.root / path
        if args.check:
            if not target.exists() or target.read_bytes() != raw:
                raise ValueError("M23_STALE_AUTHORED_WORLD:" + path)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)
