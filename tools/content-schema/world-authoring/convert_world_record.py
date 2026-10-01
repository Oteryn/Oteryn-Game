#!/usr/bin/env python3
"""Convert the committed base map into the World family record.

Writes content/world/worlds/ (family index plus exactly one shard holding the one World
record). The record is derived offline from committed data only: the tile bounding box and
the floors come from the ``WorldPlacement.Base`` region files, the source map header from
``samples/world-base-capture-v1.json``, the legacy world identity from
``content/world/definitions/reference.json``. Bounds follow the half-open horizontal
envelope of OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1: the maximum is exclusive.

    python convert_world_record.py [--check]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import convert_world_metadata as metadata
import world_region_codec as codec
import zstandard
from convert_world_metadata import ConvertError, canonical

ROOT = metadata.ROOT
DIRECTORY = "content/world/worlds"
STEM = "worlds"
GENERATOR = "tools/content-schema/world-authoring/convert_world_record.py"
PLACEMENTS = "content/world/placements"
BASE_SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
REFERENCE = "content/world/definitions/reference.json"
KEY = "oteryn:world.oteryn"
NAME = "Oteryn"
MAP_NAMESPACE = "crystalserver/world-map"
MAP_FILE = "data-global/world/world.otbm"
SOURCE = {
    **metadata.SOURCE,
    "files": [row for row in metadata.SOURCE["files"] if row["path"] == MAP_FILE],
}
SHARD_PATH = f"{DIRECTORY}/{STEM}-00000-00000.json"


def _sector_table(root: Path, index: dict) -> tuple[list, set[int]]:
    """Every non-empty sector as ``(sx, sy, path, offset, length)`` plus the floors."""
    sectors, floors = [], set()
    side = codec.SECTORS_PER_SIDE
    for row in index["regions"]:
        z, rx, ry, table = codec.parse_region((root / row["path"]).read_bytes())
        floors.add(z)
        for local, offset, length in table:
            sectors.append(
                (rx * side + local % side, ry * side + local // side, row["path"])
                + (offset, length)
            )
    return sectors, floors


def placement_extent(root: Path = ROOT) -> dict:
    """The exact tile bounding box and floor list of the committed base map.

    Region sector tables give the sector bounding box; only the sectors on its four edges
    are decoded to find the exact tile coordinates, which keeps this fast.
    """
    index_path = root / PLACEMENTS / "index.json"
    if not index_path.is_file():
        raise ConvertError(f"{PLACEMENTS}/index.json: the base map is not committed")
    sectors, floors = _sector_table(root, json.loads(index_path.read_text("utf-8")))
    if not sectors:
        raise ConvertError("the base map holds no tile")
    edges = {
        "min_x": (min(s[0] for s in sectors), 0, min),
        "max_x": (max(s[0] for s in sectors), 0, max),
        "min_y": (min(s[1] for s in sectors), 1, min),
        "max_y": (max(s[1] for s in sectors), 1, max),
    }
    decompress = zstandard.ZstdDecompressor().decompress
    files: dict[str, bytes] = {}
    extent = {}
    for name, (edge, axis, pick) in edges.items():
        values = []
        for sx, sy, path, offset, length in sectors:
            if (sx, sy)[axis] != edge:
                continue
            data = files.setdefault(path, (root / path).read_bytes())
            payload = decompress(
                data[offset : offset + length], max_output_size=codec.MAX_SECTOR_BYTES
            )
            values.extend(t[axis] for t in codec.decode_sector(payload, sx, sy))
        extent[name] = pick(values)
    return {"floors": sorted(floors), **extent}


def world_record(extent: dict, base_summary: dict, reference: dict) -> dict:
    return {
        "declaration": {
            "bounds": {
                "max_x_exclusive": extent["max_x"] + 1,
                "max_y_exclusive": extent["max_y"] + 1,
                "min_x": extent["min_x"],
                "min_y": extent["min_y"],
            },
            "coordinate_frame": metadata.COORDINATE_FRAME,
            "floors": extent["floors"],
            "identity": {"key": KEY, "revision": metadata.REVISION},
            "kind": "World",
            "legacy_world_id": reference["world_id"],
            "name": NAME,
            "source_map": {
                "height": base_summary["map"]["height"],
                "otbm_version": base_summary["map"]["otbm_version"],
                "width": base_summary["map"]["width"],
            },
        },
        "source_bindings": [
            {
                **metadata.binding("World", KEY, MAP_NAMESPACE, MAP_FILE),
                "external_id": MAP_FILE,
            }
        ],
    }


def build(root: Path = ROOT) -> dict[str, bytes]:
    base_summary = json.loads((root / BASE_SUMMARY).read_text(encoding="utf-8"))
    reference = json.loads((root / REFERENCE).read_text(encoding="utf-8"))
    if reference["coordinate_frame"] != metadata.COORDINATE_FRAME:
        raise ConvertError(f"{REFERENCE}: coordinate frame differs")
    record = world_record(placement_extent(root), base_summary, reference)
    shard = {
        "family": "World",
        "records": [record],
        "schema": "OTERYN_WORLD_AUTHORING_SHARD/v1",
        "shard": {"count": 1, "end": 0, "index": 0, "start": 0},
    }
    index = {
        "coordinate_frame": metadata.COORDINATE_FRAME,
        "family": "World",
        "generator": GENERATOR,
        "population_state": "POPULATED",
        "record_count": 1,
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "shard_size": metadata.SHARD_SIZE,
        "shards": [SHARD_PATH],
        "source": SOURCE,
    }
    return {
        f"{DIRECTORY}/index.json": canonical(index),
        SHARD_PATH: canonical(shard),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    args = parser.parse_args()
    try:
        out = build()
    except (ConvertError, codec.CodecError, KeyError, OSError) as error:
        print(f"FAIL {error}", file=sys.stderr)
        return 1
    stale = [
        path
        for path, data in out.items()
        if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
    ]
    # Everything else in the directory is the legacy locator, which is not ours to touch.
    extra = sorted(
        str(p.relative_to(ROOT))
        for p in (ROOT / DIRECTORY).iterdir()
        if str(p.relative_to(ROOT)) not in out and p.name != "world.json"
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
        (ROOT / path).write_bytes(out[path])
    print(f"wrote {len(stale)} of {len(out)} files, removed {len(extra)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
