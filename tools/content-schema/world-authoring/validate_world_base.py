#!/usr/bin/env python3
"""Structural and semantic validation of the committed WorldPlacement.Base family.

Decodes every region file, so the run takes about a minute on four cores.

    python validate_world_base.py [--root REPOSITORY_ROOT] [--workers N]
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

import world_region_codec as codec
from convert_world_base import PINNED_TOTALS

HERE = Path(__file__).resolve().parent
DIRECTORY = "content/world/placements"
INDEX = f"{DIRECTORY}/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
ITEM_BINDINGS = "imports/crystalserver/bindings/items.json"
REGION_PATH = re.compile(
    rf"^{re.escape(DIRECTORY)}/region-z(\d{{2}})-x(\d{{3}})-y(\d{{3}})\.b3$"
)
REGISTRY_KEY = re.compile(r"^oteryn:item\.registry\.i(\d{8})$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
INDEX_KEYS = {
    "codec",
    "coordinate_frame",
    "family",
    "generator",
    "item_bindings",
    "population_state",
    "region_size",
    "regions",
    "schema",
    "sector_size",
    "shards",
    "source",
    "totals",
    "zstd_level",
}
REGION_KEYS = {"items", "path", "sha256", "tiles"}
TOTAL_KEYS = {"items", "regions", "sectors", "tiles"}


class ValidationError(Exception):
    pass


def canonical(value) -> bytes:
    text = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return (text + "\n").encode()


def load(root: Path, path: str, strict: bool = True):
    try:
        raw = (root / path).read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        raise ValidationError(f"{path}: unreadable ({error})") from error
    if strict and raw != canonical(value):
        raise ValidationError(f"{path}: not canonical JSON")
    return value


_STATE: dict = {}


def _init(root: str, extent: tuple[int, int], registries: set[int]) -> None:
    _STATE.update(root=Path(root), extent=extent, registries=registries)


def check_region(row: dict) -> dict:
    """Validate one region file in isolation; runs in a worker process."""
    path = row["path"]
    result = {
        "errors": [],
        "size": 0,
        "sectors": 0,
        "tiles": 0,
        "items": 0,
        "houses": 0,
        "zones": 0,
        "z": -1,
        "attributes": Counter(),
    }
    errors = result["errors"]
    try:
        data = (_STATE["root"] / path).read_bytes()
    except OSError as error:
        errors.append(f"{path}: unreadable ({error})")
        return result
    result["size"] = len(data)
    if hashlib.sha256(data).hexdigest() != row["sha256"]:
        errors.append(f"{path}: sha256 differs from the index")
    match = REGION_PATH.match(path)
    seen: set[int] = set()
    try:
        z, rx, ry, sectors = codec.decode_region(data, seen)
    except (codec.CodecError, OverflowError) as error:
        errors.append(f"{path}: {error}")
        return result
    if match is None or (z, rx, ry) != tuple(map(int, match.groups())):
        errors.append(f"{path}: header floor/region differs from the file name")
    result["z"] = z
    result["sectors"] = len(sectors)
    width, height = _STATE["extent"]
    tiles = items = houses = zones = 0
    attributes = result["attributes"]
    for local, sector in sectors:
        if not sector:
            errors.append(f"{path}: sector {local} is empty")
        previous = (-1, -1)
        for x, y, _flags, house, tile_zones, tile_items in sector:
            if (y, x) <= previous:
                errors.append(f"{path}: tiles not strictly sorted at ({x}, {y})")
                break
            previous = (y, x)
            if not (x < width and y < height):
                errors.append(f"{path}: tile ({x}, {y}) outside the map extent")
                break
            if (x // codec.REGION_SIZE, y // codec.REGION_SIZE) != (rx, ry):
                errors.append(f"{path}: tile ({x}, {y}) outside its region")
                break
            houses += house != 0
            zones += bool(tile_zones)
            items += len(tile_items)
            for _registry, depth, attrs in tile_items:
                if depth:
                    attributes["depth"] += 1
                if attrs:
                    attributes.update(attrs.keys())
        tiles += len(sector)
    unknown = seen - _STATE["registries"]
    if unknown:
        errors.append(
            f"{path}: registry numbers without a binding: {sorted(unknown)[:10]}"
        )
    if (tiles, items) != (row["tiles"], row["items"]):
        errors.append(
            f"{path}: decoded {tiles} tiles/{items} items, index says {row['tiles']}/{row['items']}"
        )
    result.update(tiles=tiles, items=items, houses=houses, zones=zones)
    return result


def check_index(index: dict, summary: dict, errors: list[str]) -> None:
    if set(index) != INDEX_KEYS:
        errors.append(f"{INDEX}: keys must be exactly {sorted(INDEX_KEYS)}")
    expected = {
        "codec": codec.CODEC,
        "family": "WorldPlacement.Base",
        "population_state": "POPULATED",
        "region_size": codec.REGION_SIZE,
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "sector_size": codec.SECTOR_SIZE,
        "zstd_level": codec.ZSTD_LEVEL,
    }
    for key, value in expected.items():
        if index.get(key) != value:
            errors.append(f"{INDEX}: {key} must be {value!r}")
    if index.get("source") != summary.get("source"):
        errors.append(f"{INDEX}: source differs from the capture summary")
    frame = index.get("coordinate_frame")
    if frame != "global-target-2026-09-27":
        errors.append(f"{INDEX}: unexpected coordinate_frame {frame!r}")
    bindings = index.get("item_bindings", {})
    if bindings.get("path") != ITEM_BINDINGS or not SHA256.match(
        str(bindings.get("sha256"))
    ):
        errors.append(
            f"{INDEX}: item_bindings must name {ITEM_BINDINGS} and its sha256"
        )
    generator = str(index.get("generator"))
    if not (HERE.parents[2] / generator).is_file():
        errors.append(f"{INDEX}: generator {generator!r} does not exist")


def unpopulated(root: Path) -> bool:
    """True while the directory is still the READY_UNPOPULATED marker and nothing else."""
    try:
        marker = json.loads((root / INDEX).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return False
    return (
        marker.get("schema") == "OTERYN_GAME_TREE_DIRECTORY/v1"
        and marker.get("population_state") == "READY_UNPOPULATED"
        and [p.name for p in (root / DIRECTORY).iterdir()] == ["index.json"]
    )


def validate(root: Path, pinned: dict | None = None, workers: int = 1) -> list[str]:
    errors: list[str] = []
    if unpopulated(root):
        return errors
    index = load(root, INDEX)
    summary = load(root, SUMMARY)
    check_index(index, summary, errors)
    rows = index.get("regions", [])
    paths = [row.get("path") for row in rows]
    if index.get("shards") != paths:
        errors.append(f"{INDEX}: shards must equal the region paths, in order")
    if paths != sorted(set(paths)):
        errors.append(f"{INDEX}: regions must be unique and sorted by path")
    for row in rows:
        if set(row) != REGION_KEYS or not REGION_PATH.match(str(row.get("path"))):
            errors.append(f"{INDEX}: malformed region row {row!r}"[:200])
        elif (
            not SHA256.match(str(row["sha256"])) or min(row["tiles"], row["items"]) < 0
        ):
            errors.append(f"{INDEX}: malformed region row {row['path']}")
    if errors:
        return errors
    directory = root / DIRECTORY
    actual = sorted(str(p.relative_to(root)) for p in directory.iterdir())
    if actual != sorted([INDEX, *paths]):
        stray = sorted(set(actual) ^ {INDEX, *paths})
        errors.append(
            f"{DIRECTORY}: files other than the index and its shards: {stray}"
        )
        return errors

    registries = set()
    for row in load(root, ITEM_BINDINGS, strict=False)["bindings"]:
        match = REGISTRY_KEY.match(row["target"]["key"])
        if match:
            registries.add(int(match.group(1)))
    extent = (summary["map"]["width"], summary["map"]["height"])
    args = (str(root), extent, registries)
    if workers > 1 and len(rows) > 8:
        with ProcessPoolExecutor(workers, initializer=_init, initargs=args) as pool:
            results = list(pool.map(check_region, rows, chunksize=16))
    else:
        _init(*args)
        results = [check_region(row) for row in rows]

    floors: Counter = Counter()
    attributes: Counter = Counter()
    totals = Counter()
    for result in results:
        errors.extend(result["errors"])
        floors[result["z"]] += result["tiles"]
        attributes.update(result["attributes"])
        totals.update(
            regions=1,
            sectors=result["sectors"],
            tiles=result["tiles"],
            items=result["items"],
            bytes=result["size"],
            houses=result["houses"],
            zones=result["zones"],
        )
    counted = {key: totals[key] for key in sorted(TOTAL_KEYS)}
    if index["totals"] != counted:
        errors.append(
            f"{INDEX}: totals {index['totals']} differ from the decoded {counted}"
        )
    if summary.get("totals") != index["totals"]:
        errors.append(f"{SUMMARY}: totals differ from the index")
    if summary.get("bytes_on_disk") != totals["bytes"]:
        errors.append(f"{SUMMARY}: bytes_on_disk differs from the region files")
    if summary.get("tiles_with_house") != totals["houses"]:
        errors.append(f"{SUMMARY}: tiles_with_house differs from the decoded tiles")
    if summary.get("tiles_with_zone") != totals["zones"]:
        errors.append(f"{SUMMARY}: tiles_with_zone differs from the decoded tiles")
    expected_floors = {str(z): n for z, n in sorted(floors.items())}
    if summary.get("tiles_by_floor") != expected_floors:
        errors.append(f"{SUMMARY}: tiles_by_floor differs from the decoded tiles")
    attributes.pop("depth", None)
    if summary.get("item_attributes") != dict(sorted(attributes.items())):
        errors.append(f"{SUMMARY}: item_attributes differs from the decoded items")
    if summary.get("schema") != "OTERYN_WORLD_BASE_SOURCE_CAPTURE/v1":
        errors.append(f"{SUMMARY}: wrong schema")
    if any(
        summary.get("rejected_items", {}).get(k) != 0
        for k in ("unbound_item_ids", "unsupported_attributes")
    ) or set(summary.get("rejected_items", {})) != {
        "unbound_item_ids",
        "unsupported_attributes",
    }:
        errors.append(f"{SUMMARY}: rejected_items must all be 0")
    info = summary.get("codec", {})
    if (info.get("name"), info.get("zstd_level")) != (codec.CODEC, codec.ZSTD_LEVEL):
        errors.append(f"{SUMMARY}: codec differs from the index")
    if not all(isinstance(v, str) and v for v in info.get("zstd", {}).values()) or set(
        info.get("zstd", {})
    ) != {"backend", "libzstd", "python_package"}:
        errors.append(f"{SUMMARY}: codec.zstd must record backend, libzstd and package")
    if summary.get("map", {}).get("floors") != [0, codec.MAX_FLOOR]:
        errors.append(f"{SUMMARY}: map floors must be [0, {codec.MAX_FLOOR}]")
    if pinned is not None and {k: index["totals"][k] for k in pinned} != pinned:
        errors.append(f"{INDEX}: totals differ from the pinned source {pinned}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=HERE.parents[2])
    parser.add_argument("--workers", type=int, default=min(4, os.cpu_count() or 1))
    args = parser.parse_args()
    try:
        errors = validate(args.root.resolve(), PINNED_TOTALS, args.workers)
    except ValidationError as error:
        errors = [str(error)]
    for error in errors[:50]:
        print(f"FAIL {error}", file=sys.stderr)
    if errors:
        return 1
    state = "unpopulated marker" if unpopulated(args.root.resolve()) else "populated"
    print(f"PASS world base family ({state})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
