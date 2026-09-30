#!/usr/bin/env python3
"""Validate the committed World family and that the map content stays inside it.

python validate_world_record.py [--root REPOSITORY_ROOT]

Checks the family files (schema, canonical bytes, no stray file beside the legacy locator),
that the record's bounds and floors equal the exact tile bounding box and floors of the
committed base map, that the source map header covers the bounds, and that every
teleport and hunting place position lies inside the bounds
on a declared floor. Base map tiles are inside by the exact-extent equality.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import convert_world_metadata as metadata
import convert_world_record as convert
from jsonschema import Draft202012Validator
from validate_world_metadata import FAMILIES, positions

HERE = Path(__file__).resolve().parent
SCHEMA = json.loads((HERE / "world-record.schema.json").read_text(encoding="utf-8"))
LEGACY_LOCATOR = "world.json"


class ValidationError(Exception):
    pass


def load(root: Path, path: str):
    try:
        return json.loads((root / path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValidationError(f"{path}: unreadable ({error})") from error


def canonical(value) -> str:
    return metadata.canonical(value).decode()


def outside(pos: dict, bounds: dict, floors: set[int]) -> str | None:
    """Why a tile position is not inside the World, or None."""
    if pos["coordinate_frame"] != metadata.COORDINATE_FRAME:
        return "coordinate frame differs"
    if not (
        bounds["min_x"] <= pos["x"] < bounds["max_x_exclusive"]
        and bounds["min_y"] <= pos["y"] < bounds["max_y_exclusive"]
    ):
        return "outside the world bounds"
    if pos["floor"] not in floors:
        return "on an undeclared floor"
    return None


def family_positions(root: Path, family: str) -> list[tuple[str, dict]]:
    """(record key, position) for every position of a metadata family."""
    directory = FAMILIES[family][0]
    index = load(root, f"{directory}/index.json")
    rows = []
    for shard in index["shards"]:
        for record in load(root, shard)["records"]:
            declaration = record["declaration"]
            key = declaration["identity"]["key"]
            rows.extend((key, pos) for pos in positions(declaration))
            footprint = declaration.get("footprint")
            if footprint:
                # The footprint box corners, on each floor the footprint occupies
                # (a footprint lists `floors` or has one `floor`).
                for row in footprint.get("floors") or [footprint]:
                    for x in (footprint["min_x"], footprint["max_x"]):
                        for y in (footprint["min_y"], footprint["max_y"]):
                            rows.append(
                                (
                                    key,
                                    {
                                        "coordinate_frame": footprint.get(
                                            "coordinate_frame"
                                        )
                                        or declaration["entry"]["coordinate_frame"],
                                        "floor": row["floor"],
                                        "x": x,
                                        "y": y,
                                    },
                                )
                            )
    return rows


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    directory = convert.DIRECTORY
    index_path, shard_path = f"{directory}/index.json", convert.SHARD_PATH
    index, shard = load(root, index_path), load(root, shard_path)
    for path, data in ((index_path, index), (shard_path, shard)):
        for error in Draft202012Validator(SCHEMA).iter_errors(data):
            errors.append(f"{path}: schema: {error.message[:200]}")
        if (root / path).read_text(encoding="utf-8") != canonical(data):
            errors.append(f"{path}: not canonical JSON")
    if errors:
        return errors
    present = sorted(p.name for p in (root / directory).iterdir())
    wanted = sorted([LEGACY_LOCATOR, Path(index_path).name, Path(shard_path).name])
    if present != wanted:
        errors.append(f"{directory}: expected exactly {wanted}, found {present}")
    if index["source"] != convert.SOURCE:
        errors.append(f"{index_path}: source differs from the pinned source")
    record = shard["records"][0]
    declaration = record["declaration"]
    key = declaration["identity"]["key"]
    binding = record["source_bindings"][0]
    if binding["target"] != {
        "family": "World",
        "key": key,
        "revision": metadata.REVISION,
    }:
        errors.append(f"{key}: source binding target differs from identity")
    if binding["source_revision"] != metadata.SOURCE["revision"]:
        errors.append(f"{key}: source binding revision differs from the pinned source")
    if binding["external_id"] != convert.MAP_FILE:
        errors.append(f"{key}: source binding is not the pinned map file")
    reference = load(root, convert.REFERENCE)
    if declaration["legacy_world_id"] != reference["world_id"]:
        errors.append(f"{key}: legacy_world_id differs from {convert.REFERENCE}")
    if declaration["coordinate_frame"] != reference["coordinate_frame"]:
        errors.append(f"{key}: coordinate frame differs from {convert.REFERENCE}")
    bounds, floors = declaration["bounds"], declaration["floors"]
    if not (
        bounds["min_x"] < bounds["max_x_exclusive"]
        and bounds["min_y"] < bounds["max_y_exclusive"]
    ):
        errors.append(f"{key}: bounds are empty")
    if floors != sorted(set(floors)):
        errors.append(f"{key}: floors must be strictly increasing")
    base = load(root, convert.BASE_SUMMARY)
    source_map = declaration["source_map"]
    if source_map != {
        "height": base["map"]["height"],
        "otbm_version": base["map"]["otbm_version"],
        "width": base["map"]["width"],
    }:
        errors.append(f"{key}: source_map differs from the base map capture summary")
    if (
        bounds["max_x_exclusive"] > source_map["width"]
        or bounds["max_y_exclusive"] > source_map["height"]
    ):
        errors.append(f"{key}: bounds exceed the source map header extent")
    extent = convert.placement_extent(root)
    exact = {
        "min_x": extent["min_x"],
        "min_y": extent["min_y"],
        "max_x_exclusive": extent["max_x"] + 1,
        "max_y_exclusive": extent["max_y"] + 1,
    }
    if bounds != exact:
        errors.append(f"{key}: bounds {bounds} differ from the base map tiles {exact}")
    if floors != extent["floors"]:
        errors.append(f"{key}: floors differ from the base map floors")
    if sorted(int(z) for z in base["tiles_by_floor"]) != floors:
        errors.append(f"{key}: floors differ from the capture summary tiles_by_floor")
    floor_set = set(floors)
    for family in FAMILIES:
        for owner, pos in family_positions(root, family):
            why = outside(pos, bounds, floor_set)
            if why:
                errors.append(f"{family} {owner}: position {pos} {why}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=HERE.parents[2])
    args = parser.parse_args()
    try:
        errors = validate(args.root.resolve())
    except (ValidationError, OSError, KeyError) as error:
        errors = [str(error)]
    for error in errors:
        print(f"FAIL {error}", file=sys.stderr)
    if errors:
        return 1
    print("PASS world record")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
