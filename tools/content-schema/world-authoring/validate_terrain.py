#!/usr/bin/env python3
"""Validate the committed Terrain family.

python validate_terrain.py [--root REPOSITORY_ROOT] [--workers N]

Checks schema and canonical bytes of the index, shards and capture summary, contiguous shard
names and no stray file, the pinned client appearances file (sha256, client asset manifest),
sorted unique keys derived from the appearance id, every binding, that each record equals a
fresh decode of its appearance (class rule, flags, speed, name, automap colour), that the
base map palette maps each record's id to its key (and each Terrain key to a record), that
every occurrence count equals a recount over the committed region files, and the capture
summary.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

import convert_floor_changes as floor_changes
import convert_terrain as convert
import convert_world_metadata as metadata
from jsonschema import Draft202012Validator

HERE = Path(__file__).resolve().parent
SCHEMA = json.loads(
    (HERE / "terrain-appearance.schema.json").read_text(encoding="utf-8")
)
INDEX = f"{convert.DIRECTORY}/index.json"
STEM = "terrain"
SHARD_SIZE = metadata.SHARD_SIZE


class ValidationError(Exception):
    pass


def load(root: Path, path: str):
    try:
        return json.loads((root / path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValidationError(f"{path}: unreadable ({error})") from error


def canonical(value) -> str:
    return metadata.canonical(value).decode()


def structural(kind: str, path: str, data, errors: list[str]) -> None:
    schema = {"$ref": f"#/$defs/{kind}", "$defs": SCHEMA["$defs"]}
    for error in Draft202012Validator(schema).iter_errors(data):
        errors.append(f"{path}: schema: {error.message[:200]}")


def read_family(root: Path, errors: list[str]) -> tuple[dict, list[dict]]:
    index = load(root, INDEX)
    structural("family_index", INDEX, index, errors)
    records: list[dict] = []
    expected = []
    for shard_index, path in enumerate(index.get("shards", [])):
        shard = load(root, path)
        structural("terrain_shard", path, shard, errors)
        start = len(records)
        chunk = shard.get("records", [])
        end = start + len(chunk) - 1
        name = f"{convert.DIRECTORY}/{STEM}-{start:05d}-{end:05d}.json"
        expected.append(name)
        if path != name:
            errors.append(f"{path}: expected contiguous shard name {name}")
        if shard.get("shard") != {
            "count": len(chunk),
            "end": end,
            "index": shard_index,
            "start": start,
        }:
            errors.append(f"{path}: shard header does not match its records")
        if len(chunk) != SHARD_SIZE and shard_index != len(index["shards"]) - 1:
            errors.append(f"{path}: only the last shard may be partial")
        if (root / path).read_text(encoding="utf-8") != canonical(shard):
            errors.append(f"{path}: not canonical JSON")
        records.extend(chunk)
    if (root / INDEX).read_text(encoding="utf-8") != canonical(index):
        errors.append(f"{INDEX}: not canonical JSON")
    actual = sorted(
        str(p.relative_to(root)) for p in (root / convert.DIRECTORY).iterdir()
    )
    if actual != sorted([INDEX, *expected]):
        errors.append(
            f"{convert.DIRECTORY}: files other than the index and its shards: "
            f"{sorted(set(actual) - {INDEX, *expected})}"
        )
    if index.get("record_count") != len(records):
        errors.append(f"{INDEX}: record_count differs from the shards")
    return index, records


def validate_records(
    root: Path, records: list[dict], client: dict, digest: str, errors: list[str]
) -> None:
    keys = [r["declaration"]["identity"]["key"] for r in records]
    if keys != sorted(keys) or len(set(keys)) != len(keys):
        errors.append("Terrain: keys must be unique and sorted")
    palette = load(root, convert.PLACEMENTS_INDEX)["palette"]
    by_id = {row["source_item_id"]: row for row in palette}
    for record in records:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        appearance_id = declaration["appearance_id"]
        if key != convert.record_key(appearance_id):
            errors.append(f"{key}: key differs from appearance id {appearance_id}")
        (binding,) = record["source_bindings"]
        if (
            binding["external_id"] != str(appearance_id)
            or binding["target"]["key"] != key
            or binding["source_revision"] != digest
        ):
            errors.append(f"{key}: source binding differs from the record")
        appearance = client.get(appearance_id)
        if appearance is None:
            errors.append(
                f"{key}: appearance {appearance_id} is not in the client file"
            )
            continue
        expected = {
            "appearance_id": appearance_id,
            "class": convert.terrain_class(appearance.flags),
            "flags": sorted(appearance.flags),
            "identity": declaration["identity"],
            "kind": "Terrain",
            "occurrences_on_base_map": declaration["occurrences_on_base_map"],
        }
        if appearance.name:
            expected["name"] = appearance.name
        if appearance.automap_color is not None:
            expected["automap_color"] = appearance.automap_color
        if "bank" in appearance.flags:
            expected["speed"] = appearance.speed
        if declaration != expected:
            errors.append(f"{key}: facts differ from the client appearance")
        row = by_id.get(appearance_id)
        if row is None or row["key"] != key or row["provisional"]:
            errors.append(
                f"{key}: base map palette does not map id {appearance_id} to it"
            )
    keyed = {
        row["source_item_id"]
        for row in palette
        if row["key"].startswith(convert.KEY_PREFIX)
    }
    if keyed != {r["declaration"]["appearance_id"] for r in records}:
        errors.append("Terrain: palette terrain keys differ from the records")


def validate(root: Path, workers: int = 1) -> list[str]:
    errors: list[str] = []
    index, records = read_family(root, errors)
    summary = load(root, convert.SUMMARY)
    structural("capture_summary", convert.SUMMARY, summary, errors)
    if errors:
        return errors
    if (root / convert.SUMMARY).read_text(encoding="utf-8") != canonical(summary):
        errors.append(f"{convert.SUMMARY}: not canonical JSON")
    client, source = convert.load_client(root)
    if index["source"] != source or summary["source"] != source:
        errors.append(
            "Terrain: pinned client source differs from the manifest and file"
        )
    if errors:
        return errors
    digest = source["files"][0]["sha256"]
    validate_records(root, records, client, digest, errors)
    if errors:
        return errors
    position = {
        row["source_item_id"]: i
        for i, row in enumerate(load(root, convert.PLACEMENTS_INDEX)["palette"])
    }
    counts = floor_changes.count_tile_items(
        root, {position[r["declaration"]["appearance_id"]] for r in records}, workers
    )
    for record in records:
        declaration = record["declaration"]
        actual = counts[position[declaration["appearance_id"]]]
        if declaration["occurrences_on_base_map"] != actual:
            errors.append(
                f"{declaration['identity']['key']}: occurrences "
                f"{declaration['occurrences_on_base_map']} differ from the base map {actual}"
            )
    if summary != json.loads(
        metadata.canonical(convert.capture_summary(records, client, source))
    ):
        errors.append(f"{convert.SUMMARY}: differs from the records")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=HERE.parents[2])
    parser.add_argument("--workers", type=int, default=min(4, os.cpu_count() or 1))
    args = parser.parse_args()
    try:
        errors = validate(args.root.resolve(), args.workers)
    except (ValidationError, OSError, KeyError, convert.ConvertError) as error:
        errors = [str(error)]
    for error in errors:
        print(f"FAIL {error}", file=sys.stderr)
    if errors:
        return 1
    print("PASS terrain")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
