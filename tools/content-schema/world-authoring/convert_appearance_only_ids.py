#!/usr/bin/env python3
"""List the appearance-only ids of the base map as evidence for the WO lane.

An appearance-only id is a palette id that the pinned CrystalServer `items.xml` does not
declare and that only the official client knows. It has no Item and no A12 section 4.6
catalogue record, so its palette key stays provisional. Each row carries the id, its
appearance class and its occurrences on the committed base map. `class` follows one ordered
rule over the client flags:

    bank flag            -> ground      (walkable surface; `speed` is the bank waypoints value)
    else clip flag       -> border      (edge pieces laid over a ground)
    else unpass flag     -> blocking    (creatures cannot pass)
    else                 -> decoration

Writes `samples/appearance-only-ids-v1.json` from the committed palette, the region files and
the client `appearances-<sha256>.dat` (verified against the client asset manifest).

    python convert_appearance_only_ids.py --crystal-root PATH [--check]
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

import client_appearance_reader as appearances
import convert_map_regions as regions
import convert_world_base as world_base
import convert_world_metadata as metadata
import world_region_codec as codec
from convert_world_metadata import ConvertError, canonical

ROOT = metadata.ROOT
HERE = metadata.HERE
SAMPLE = HERE / "samples/appearance-only-ids-v1.json"
PLACEMENTS_INDEX = f"{world_base.DIRECTORY}/index.json"
APPEARANCES_NAME = re.compile(r"^appearances-([0-9a-f]{64})\.dat$")
CLASSES = ("ground", "border", "blocking", "decoration")
CLASS_RULE = [
    "bank flag: ground; the bank waypoints value is `speed`",
    "else clip flag: border",
    "else unpass flag: blocking",
    "else: decoration",
]


def appearance_class(flags: frozenset[str]) -> str:
    if "bank" in flags:
        return "ground"
    if "clip" in flags:
        return "border"
    if "unpass" in flags:
        return "blocking"
    return "decoration"


def load_client(root: Path) -> tuple[dict[int, appearances.Appearance], dict]:
    """The decoded appearances and their pin, verified against the client asset manifest."""
    manifest_doc = json.loads((root / regions.MANIFEST).read_text(encoding="utf-8"))
    files = {row["name"]: row["sha256"] for row in manifest_doc["files"]}
    names = sorted(n for n in files if APPEARANCES_NAME.fullmatch(n))
    if len(names) != 1:
        raise ConvertError("the client manifest must list exactly one appearances file")
    name = names[0]
    data = (root / regions.FILES / name).read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if files[name] != digest or name != f"appearances-{digest}.dat":
        raise ConvertError(f"{name}: sha256 differs from the client asset manifest")
    source = {
        "client_version": regions.CLIENT_VERSION,
        "evidence": "OfficialClient",
        "files": [{"path": f"{regions.FILES}/{name}", "sha256": digest}],
        "manifest": {
            "archive_sha256": manifest_doc["archive_sha256"],
            "path": regions.MANIFEST,
        },
        "source_key": regions.SOURCE_KEY,
    }
    return appearances.read_appearances(data), source


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


def read_items_xml(crystal_root: Path) -> bytes:
    """The sha256-pinned items.xml of the source checkout."""
    row = next(
        r for r in world_base.SOURCE["files"] if r["path"] == world_base.ITEMS_XML
    )
    data = (crystal_root / row["path"]).read_bytes()
    if hashlib.sha256(data).hexdigest() != row["sha256"]:
        raise ConvertError(f"{row['path']}: sha256 differs from the pinned source")
    return data


def build(declared: set[int], root: Path = ROOT, workers: int = 1) -> dict[str, bytes]:
    """`declared` is every id the pinned items.xml declares."""
    client, source = load_client(root)
    palette = json.loads((root / PLACEMENTS_INDEX).read_text(encoding="utf-8"))[
        "palette"
    ]
    position = {row["source_item_id"]: i for i, row in enumerate(palette)}
    ids = sorted(
        row["source_item_id"]
        for row in palette
        if row["provisional"]
        and row["source_item_id"] not in declared
        and row["source_item_id"] in client
    )
    if not ids:
        raise ConvertError("no appearance-only ids")
    counts = count_tile_items(root, {position[i] for i in ids}, workers)
    rows = []
    for appearance_id in ids:
        appearance = client[appearance_id]
        row: dict = {
            "class": appearance_class(appearance.flags),
            "id": appearance_id,
            "occurrences_on_base_map": counts[position[appearance_id]],
        }
        if appearance.name:
            row["name"] = appearance.name
        if "bank" in appearance.flags:
            if appearance.speed is None:
                raise ConvertError(f"id {appearance_id}: bank flag without a speed")
            row["speed"] = appearance.speed
        rows.append(row)
    classes = {
        name: {
            "ids": sum(r["class"] == name for r in rows),
            "occurrences_on_base_map": sum(
                r["occurrences_on_base_map"] for r in rows if r["class"] == name
            ),
        }
        for name in CLASSES
    }
    document = {
        "appearances": {"objects": len(client), **source["files"][0]},
        "class_rule": CLASS_RULE,
        "classes": classes,
        "ids": len(rows),
        "occurrences_on_base_map": sum(r["occurrences_on_base_map"] for r in rows),
        "palette_provisional_entries": sum(r["provisional"] for r in palette),
        "rows": rows,
        "schema": "OTERYN_APPEARANCE_ONLY_IDS_EVIDENCE/v1",
        "source": source,
    }
    return {str(SAMPLE.relative_to(ROOT)): canonical(document)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--crystal-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    parser.add_argument("--workers", type=int, default=min(4, os.cpu_count() or 1))
    args = parser.parse_args()
    try:
        raw = read_items_xml(args.crystal_root)
        out = build(world_base.items_xml_ids(raw), workers=args.workers)
    except (ConvertError, OSError) as error:
        print(f"FAIL {error}", file=sys.stderr)
        return 1
    stale = [
        path
        for path, data in out.items()
        if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
    ]
    if args.check:
        for path in stale:
            print(f"STALE {path}", file=sys.stderr)
        return 1 if stale else 0
    for path in stale:
        (ROOT / path).write_bytes(out[path])
    print(f"wrote {len(stale)} of {len(out)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
