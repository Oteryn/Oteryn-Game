#!/usr/bin/env python3
"""Convert the floor-change item types of the pinned CrystalServer items.xml.

Writes the ``WorldObject.FloorChange`` family (content/world/objects/) and the committed
capture summary. One record per item type that declares a ``floorchange`` attribute
(stairs, ramps, holes and trapdoors). The record key derives from the item's identity key,
resolved by the same rule as the base map palette: the ``ots/item_server_id`` binding
target, else the provisional donor key. Occurrences are counted from the committed
``WorldPlacement.Base`` region files (top-level tile items, container contents excluded).

Ladders, rope spots and sewer grates are scripted ``use`` actions in the server, not
``floorchange`` attributes, so they are not part of this family.

    python convert_floor_changes.py --crystal-root /path/to/crystalserver [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from collections import Counter
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path
from xml.etree import ElementTree

import convert_world_base as base
import convert_world_metadata as metadata
import otbm_reader
import world_region_codec as codec
from convert_world_metadata import ConvertError, canonical

ROOT = metadata.ROOT
HERE = metadata.HERE
DIRECTORY = "content/world/objects"
STEM = "floor-changes"
GENERATOR = "tools/content-schema/world-authoring/convert_floor_changes.py"
SUMMARY = HERE / "samples/floor-changes-capture-v1.json"
PLACEMENTS_INDEX = f"{base.DIRECTORY}/index.json"
KEY_PREFIX = "oteryn:world_object.floor_change."
NAMESPACE = "crystalserver/items-xml-id"
ITEM_KEY_PREFIX = "oteryn:item."

SOURCE = {
    **metadata.SOURCE,
    "files": [row for row in base.SOURCE["files"] if row["path"] == base.ITEMS_XML],
}
# items.xml `floorchange` value -> normalized kind. The engine (src/items/tile.cpp,
# Tile::queryDestination) reads them as tile flags: `down` sends a creature one floor down;
# the others send it one floor up and move it by the listed step:
#   north (0, -1)   south (0, +1)   east (+1, 0)   west (-1, 0)
#   southalt (0, +2)   eastalt (+2, 0)
FLOOR_CHANGE = {
    "down": "down",
    "north": "up_north",
    "south": "up_south",
    "east": "up_east",
    "west": "up_west",
    "southalt": "up_south_alt",
    "eastalt": "up_east_alt",
}
# Kinds the engine does not take from items.xml `floorchange`.
EXCLUDED = [
    {
        "kind": "ladder_up",
        "reason": "scripted use action, no static floorchange attribute in items.xml",
    },
    {
        "kind": "rope_spot",
        "reason": "scripted use action with a rope, no static floorchange attribute",
    },
    {
        "kind": "sewer_grate",
        "reason": "scripted use action, no static floorchange attribute",
    },
    {
        "kind": "shovel_or_pick_hole",
        "reason": "runtime terrain change from a tool use, not an item attribute",
    },
]
ENGINE_REFERENCE = {
    "destination": "src/items/tile.cpp Tile::queryDestination",
    "parser": "src/items/functions/item/item_parse.cpp ItemParse::parseFloorChange",
    "values": "src/items/functions/item/item_parse.hpp TileStatesMap",
}


def record_key(item_key: str) -> str:
    """The WorldObject key derived from the item identity key."""
    if item_key.startswith(ITEM_KEY_PREFIX):
        tail = item_key[len(ITEM_KEY_PREFIX) :]
    elif item_key.startswith(base.DONOR_PREFIX):
        tail = "donor_" + item_key[len(base.DONOR_PREFIX) :]
    else:
        raise ConvertError(f"{item_key!r} is neither an Oteryn nor a donor item key")
    slug = re.sub(r"[^a-z0-9]+", "_", tail.lower()).strip("_")
    if not slug:
        raise ConvertError(f"{item_key!r} has no production slug")
    return KEY_PREFIX + slug


def floor_change_items(xml: bytes) -> dict[int, dict]:
    """``{server id: {"floor_change", "name"}}`` for every id with a ``floorchange``.

    Ranges (fromid/toid) expand to one entry per id. Fails closed on an unknown value, a
    second declaration, or an id that items.xml declares in two nodes.
    """
    declared: Counter = Counter()
    found: dict[int, dict] = {}
    for element in ElementTree.fromstring(xml).iter("item"):
        if "id" in element.attrib:
            ids = [int(element.attrib["id"])]
        else:
            ids = list(
                range(int(element.attrib["fromid"]), int(element.attrib["toid"]) + 1)
            )
        declared.update(ids)
        values = [
            attribute.attrib.get("value", "")
            for attribute in element.findall("attribute")
            if attribute.attrib.get("key", "").lower() == "floorchange"
        ]
        if not values:
            continue
        name = element.attrib.get("name", "")
        if len(values) != 1 or not name:
            raise ConvertError(f"items {ids[0]}: needs one floorchange and a name")
        kind = FLOOR_CHANGE.get(values[0].lower())
        if kind is None:
            raise ConvertError(f"items {ids[0]}: unknown floorchange {values[0]!r}")
        for server_id in ids:
            found[server_id] = {"floor_change": kind, "name": name}
    repeated = sorted(i for i in found if declared[i] > 1)
    if repeated:
        raise ConvertError(f"items.xml declares ids twice: {repeated[:5]}")
    return found


_WANTED: set[int] = set()


def _init(wanted: set[int]) -> None:
    _WANTED.update(wanted)


def _count_region(path: str) -> Counter:
    counts: Counter = Counter()
    _z, _rx, _ry, sectors = codec.decode_region(Path(path).read_bytes())
    for _local, tiles in sectors:
        for tile in tiles:
            for palette, depth, _attrs in tile[5]:
                if depth == 0 and palette in _WANTED:
                    counts[palette] += 1
    return counts


def count_tile_items(root: Path, wanted: set[int], workers: int = 1) -> Counter:
    """Top-level tile items per palette index in ``wanted``, over every region file."""
    index = json.loads((root / PLACEMENTS_INDEX).read_text(encoding="utf-8"))
    paths = [str(root / row["path"]) for row in index["regions"]]
    total: Counter = Counter()
    if workers > 1 and len(paths) > 8:
        with ProcessPoolExecutor(
            workers, initializer=_init, initargs=(wanted,)
        ) as pool:
            for counts in pool.map(_count_region, paths, chunksize=16):
                total.update(counts)
    else:
        _init(wanted)
        for path in paths:
            total.update(_count_region(path))
    return total


def committed_palette_index(root: Path) -> dict[int, int]:
    """``{server id: palette index}`` of the committed base map palette."""
    path = root / PLACEMENTS_INDEX
    if not path.is_file():
        raise ConvertError(f"{PLACEMENTS_INDEX}: the base map is not committed")
    palette = json.loads(path.read_text(encoding="utf-8"))["palette"]
    return {row["source_item_id"]: i for i, row in enumerate(palette)}


def build(
    xml: bytes, root: Path = ROOT, bindings: bytes | None = None, workers: int = 1
) -> dict[str, bytes]:
    if bindings is None:
        bindings = base.ITEM_BINDINGS.read_bytes()
    bound = base.bound_keys(bindings)
    items = floor_change_items(xml)
    if not items:
        raise ConvertError("items.xml declares no floorchange")
    palette_index = committed_palette_index(root)
    counts = count_tile_items(
        root, {palette_index[i] for i in items if i in palette_index}, workers
    )
    records = []
    for server_id, row in items.items():
        entry = base.palette_entry(server_id, bound)
        key = record_key(entry["key"])
        occurrences = (
            counts[palette_index[server_id]] if server_id in palette_index else 0
        )
        records.append(
            {
                "declaration": {
                    "floor_change": row["floor_change"],
                    "identity": {"key": key, "revision": metadata.REVISION},
                    "item": metadata.ref("Item", entry["key"]),
                    "kind": "WorldObject",
                    "name": row["name"],
                    "object_kind": "floor_change",
                    "occurrences_on_base_map": occurrences,
                    "provisional_item": entry["provisional"],
                    "source_item_id": server_id,
                },
                "source_bindings": [
                    metadata.binding("WorldObject", key, NAMESPACE, str(server_id))
                ],
            }
        )
    records.sort(key=lambda r: r["declaration"]["identity"]["key"])
    keys = [r["declaration"]["identity"]["key"] for r in records]
    if len(set(keys)) != len(keys):
        raise ConvertError("two floor-change item types share a record key")
    if len(records) > metadata.SHARD_SIZE:
        raise ConvertError("more floor-change records than one shard holds")
    shard_path = f"{DIRECTORY}/{STEM}-00000-{len(records) - 1:05d}.json"
    shard = {
        "family": "WorldObject.FloorChange",
        "records": records,
        "schema": "OTERYN_WORLD_OBJECT_AUTHORING_SHARD/v1",
        "shard": {
            "count": len(records),
            "end": len(records) - 1,
            "index": 0,
            "start": 0,
        },
    }
    index = {
        "family": "WorldObject.FloorChange",
        "generator": GENERATOR,
        "item_bindings": {
            "path": str(base.ITEM_BINDINGS.relative_to(ROOT)),
            "sha256": hashlib.sha256(bindings).hexdigest(),
        },
        "population_state": "POPULATED",
        "record_count": len(records),
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "shard_size": metadata.SHARD_SIZE,
        "shards": [shard_path],
        "source": SOURCE,
    }
    return {
        f"{DIRECTORY}/index.json": canonical(index),
        shard_path: canonical(shard),
        str(SUMMARY.relative_to(ROOT)): canonical(capture_summary(records)),
    }


def capture_summary(records: list[dict]) -> dict:
    kinds: dict[str, dict] = {}
    for kind in FLOOR_CHANGE.values():
        rows = [
            r["declaration"]
            for r in records
            if r["declaration"]["floor_change"] == kind
        ]
        kinds[kind] = {
            "item_types": len(rows),
            "occurrences": sum(r["occurrences_on_base_map"] for r in rows),
        }
    declarations = [r["declaration"] for r in records]
    return {
        "engine_reference": ENGINE_REFERENCE,
        "excluded": EXCLUDED,
        "floor_change_values": dict(FLOOR_CHANGE),
        "item_types": len(records),
        "item_types_on_base_map": sum(
            d["occurrences_on_base_map"] > 0 for d in declarations
        ),
        "kinds": kinds,
        "occurrences_on_base_map": sum(
            d["occurrences_on_base_map"] for d in declarations
        ),
        "provisional_items": sum(d["provisional_item"] for d in declarations),
        "schema": "OTERYN_FLOOR_CHANGE_SOURCE_CAPTURE/v1",
        "source": SOURCE,
    }


def read_source(crystal_root: Path) -> bytes:
    row = SOURCE["files"][0]
    data = (crystal_root / row["path"]).read_bytes()
    if hashlib.sha256(data).hexdigest() != row["sha256"]:
        raise ConvertError(f"{row['path']}: sha256 differs from the pinned source")
    return data


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--crystal-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    parser.add_argument("--workers", type=int, default=min(4, os.cpu_count() or 1))
    args = parser.parse_args()
    try:
        out = build(read_source(args.crystal_root), workers=args.workers)
    except (ConvertError, codec.CodecError, otbm_reader.OtbmError, OSError) as error:
        print(f"FAIL {error}", file=sys.stderr)
        return 1
    stale = [
        path
        for path, data in out.items()
        if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
    ]
    directory = ROOT / DIRECTORY
    extra = sorted(
        str(p.relative_to(ROOT))
        for p in directory.iterdir()
        if str(p.relative_to(ROOT)) not in out
    )
    if args.check:
        for path in stale:
            print(f"STALE {path}", file=sys.stderr)
        for path in extra:
            print(f"EXTRA {path}", file=sys.stderr)
        return 1 if stale or extra else 0
    for path in extra:
        (ROOT / path).unlink()
    for path in stale:
        (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / path).write_bytes(out[path])
    print(f"wrote {len(stale)} of {len(out)} files, removed {len(extra)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
