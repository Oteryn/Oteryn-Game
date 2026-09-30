#!/usr/bin/env python3
"""Validate the committed WorldObject.FloorChange family.

python validate_floor_changes.py [--root REPOSITORY_ROOT] [--workers N]

Checks schema and canonical bytes of the index, shard and capture summary, that the
directory holds nothing else, the pinned source and item bindings digest, key derivation
and ordering, that every item key is an item binding target with an Item record (or is the
provisional donor key of an unbound or undefined id) and equals the base map palette key, and that every occurrence count
and the capture summary equal a recount over the committed region files.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

import convert_floor_changes as convert
import convert_world_base as base
import convert_world_metadata as metadata
from jsonschema import Draft202012Validator

HERE = Path(__file__).resolve().parent
SCHEMA = json.loads((HERE / "floor-change.schema.json").read_text(encoding="utf-8"))
SUMMARY = str(convert.SUMMARY.relative_to(convert.ROOT))
INDEX = f"{convert.DIRECTORY}/index.json"


class ValidationError(Exception):
    pass


def load(root: Path, path: str):
    try:
        return json.loads((root / path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValidationError(f"{path}: unreadable ({error})") from error


def canonical(value) -> str:
    return metadata.canonical(value).decode()


def structural(path: str, data, errors: list[str]) -> None:
    for error in Draft202012Validator(SCHEMA).iter_errors(data):
        errors.append(f"{path}: schema: {error.message[:200]}")


def validate(root: Path, workers: int = 1) -> list[str]:
    errors: list[str] = []
    index = load(root, INDEX)
    summary = load(root, SUMMARY)
    structural(INDEX, index, errors)
    structural(SUMMARY, summary, errors)
    if errors:
        return errors
    shard_path = index["shards"][0]
    shard = load(root, shard_path)
    structural(shard_path, shard, errors)
    if errors:
        return errors
    for path, data in ((INDEX, index), (SUMMARY, summary), (shard_path, shard)):
        if (root / path).read_text(encoding="utf-8") != canonical(data):
            errors.append(f"{path}: not canonical JSON")
    records = shard["records"]
    expected_path = (
        f"{convert.DIRECTORY}/{convert.STEM}-00000-{len(records) - 1:05d}.json"
    )
    if shard_path != expected_path:
        errors.append(f"{shard_path}: expected contiguous shard name {expected_path}")
    if shard["shard"] != {
        "count": len(records),
        "end": len(records) - 1,
        "index": 0,
        "start": 0,
    }:
        errors.append(f"{shard_path}: shard header does not match its records")
    present = sorted(
        str(p.relative_to(root)) for p in (root / convert.DIRECTORY).iterdir()
    )
    if present != sorted([INDEX, shard_path]):
        errors.append(f"{convert.DIRECTORY}: files other than the index and its shard")
    if index["record_count"] != len(records):
        errors.append(f"{INDEX}: record_count differs from the shard")
    if index["source"] != convert.SOURCE or summary["source"] != convert.SOURCE:
        errors.append("source differs from the pinned items.xml")
    bindings = (root / base.ITEM_BINDINGS.relative_to(convert.ROOT)).read_bytes()
    if index["item_bindings"]["sha256"] != hashlib.sha256(bindings).hexdigest():
        errors.append(f"{INDEX}: item_bindings digest is stale")
    defined = base.defined_item_keys(root)
    bound = base.bound_keys(bindings, defined)
    palette = json.loads((root / convert.PLACEMENTS_INDEX).read_text("utf-8"))[
        "palette"
    ]
    by_id = {row["source_item_id"]: (i, row) for i, row in enumerate(palette)}

    keys = [r["declaration"]["identity"]["key"] for r in records]
    if keys != sorted(keys) or len(set(keys)) != len(keys):
        errors.append("WorldObject.FloorChange: keys must be unique and sorted")
    ids = [r["declaration"]["source_item_id"] for r in records]
    if len(set(ids)) != len(ids):
        errors.append("WorldObject.FloorChange: source_item_id must be unique")
    for record in records:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        server_id = declaration["source_item_id"]
        item = declaration["item"]["key"]
        try:
            derived = convert.record_key(item)
        except metadata.ConvertError as error:
            errors.append(f"{key}: {error}")
            derived = None
        if derived != key:
            errors.append(f"{key}: key does not derive from item {item}")
        if declaration["floor_change"] not in convert.FLOOR_CHANGE.values():
            errors.append(f"{key}: unknown floor_change {declaration['floor_change']}")
        if server_id in bound:
            if item != bound[server_id] or declaration["provisional_item"]:
                errors.append(
                    f"{key}: item differs from its binding {bound[server_id]}"
                )
        elif item != base.donor_key(server_id) or not declaration["provisional_item"]:
            errors.append(f"{key}: unbound id must use the provisional donor key")
        if server_id in by_id and by_id[server_id][1]["key"] != item:
            errors.append(f"{key}: item differs from the base map palette key")
        if declaration["occurrences_on_base_map"] and server_id not in by_id:
            errors.append(f"{key}: occurrences without a palette entry")
        binding = record["source_bindings"][0]
        if binding["target"] != {
            "family": "WorldObject",
            "key": key,
            "revision": metadata.REVISION,
        }:
            errors.append(f"{key}: source binding target differs from identity")
        if binding["external_id"] != str(server_id):
            errors.append(f"{key}: source binding is not the items.xml id")
        if binding["source_revision"] != metadata.SOURCE["revision"]:
            errors.append(f"{key}: source binding revision differs from the pin")
    if errors:
        return errors
    wanted = {by_id[i][0] for i in ids if i in by_id}
    counts = convert.count_tile_items(root, wanted, workers)
    for record in records:
        declaration = record["declaration"]
        server_id = declaration["source_item_id"]
        actual = counts[by_id[server_id][0]] if server_id in by_id else 0
        if declaration["occurrences_on_base_map"] != actual:
            errors.append(
                f"{declaration['identity']['key']}: occurrences "
                f"{declaration['occurrences_on_base_map']} differ from the base map {actual}"
            )
    if summary != convert.capture_summary(records):
        errors.append(f"{SUMMARY}: differs from the records")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=HERE.parents[2])
    parser.add_argument("--workers", type=int, default=min(4, os.cpu_count() or 1))
    args = parser.parse_args()
    try:
        errors = validate(args.root.resolve(), args.workers)
    except (ValidationError, OSError, KeyError) as error:
        errors = [str(error)]
    for error in errors:
        print(f"FAIL {error}", file=sys.stderr)
    if errors:
        return 1
    print("PASS floor-change objects")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
