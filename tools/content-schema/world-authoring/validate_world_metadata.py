#!/usr/bin/env python3
"""Structural and semantic validation of the committed world metadata families.

python validate_world_metadata.py [--root REPOSITORY_ROOT]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

HERE = Path(__file__).resolve().parent
SCHEMA = json.loads((HERE / "world-metadata.schema.json").read_text(encoding="utf-8"))
SUMMARY = "tools/content-schema/world-authoring/samples/source-capture-v1.json"
ITEM_BINDINGS = "imports/crystalserver/bindings/items.json"
FAMILIES = {
    "Area.City": ("content/world/areas/cities", "cities", "Area"),
    "House": ("content/houses", "houses", "House"),
    "Transition.Teleport": ("content/world/transitions", "teleports", "Transition"),
}
SHARD_SIZE = 500


class ValidationError(Exception):
    pass


def load(root: Path, path: str):
    try:
        return json.loads((root / path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValidationError(f"{path}: unreadable ({error})") from error


def canonical(value) -> str:
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    )


def structural(path: str, data, errors: list[str]) -> None:
    for error in Draft202012Validator(SCHEMA).iter_errors(data):
        errors.append(f"{path}: schema: {error.message[:200]}")


def positions(declaration: dict):
    for name in ("temple", "entry", "from", "to"):
        if name in declaration:
            yield declaration[name]
    for door in declaration.get("doors", []):
        yield door["position"]


def family_records(
    root: Path, family: str, summary: dict, errors: list[str]
) -> list[dict]:
    directory, stem, identity_family = FAMILIES[family]
    index_path = f"{directory}/index.json"
    index = load(root, index_path)
    structural(index_path, index, errors)
    if index.get("family") != family:
        errors.append(f"{index_path}: family must be {family}")
    if index.get("source") != summary["source"]:
        errors.append(f"{index_path}: source differs from the capture summary")
    records: list[dict] = []
    expected = []
    for shard_index, shard_path in enumerate(index.get("shards", [])):
        shard = load(root, shard_path)
        structural(shard_path, shard, errors)
        start = len(records)
        chunk = shard.get("records", [])
        end = start + len(chunk) - 1
        name = f"{directory}/{stem}-{start:05d}-{end:05d}.json"
        expected.append(name)
        if shard_path != name:
            errors.append(f"{shard_path}: expected contiguous shard name {name}")
        if shard.get("shard") != {
            "count": len(chunk),
            "end": end,
            "index": shard_index,
            "start": start,
        }:
            errors.append(f"{shard_path}: shard header does not match its records")
        if len(chunk) != SHARD_SIZE and shard_index != len(index["shards"]) - 1:
            errors.append(f"{shard_path}: only the last shard may be partial")
        if (root / shard_path).read_text(encoding="utf-8") != canonical(shard):
            errors.append(f"{shard_path}: not canonical JSON")
        records.extend(chunk)
    if (root / index_path).read_text(encoding="utf-8") != canonical(index):
        errors.append(f"{index_path}: not canonical JSON")
    actual = sorted(str(p.relative_to(root)) for p in (root / directory).iterdir())
    if actual != sorted([index_path, *expected]):
        errors.append(
            f"{directory}: files other than the index and its shards: {sorted(set(actual) - {index_path, *expected})}"
        )
    if index.get("record_count") != len(records) or summary["families"].get(
        family
    ) != len(records):
        errors.append(
            f"{index_path}: record_count differs from shards or capture summary"
        )
    keys = [r.get("declaration", {}).get("identity", {}).get("key") for r in records]
    if keys != sorted(keys) or len(set(keys)) != len(keys):
        errors.append(f"{family}: keys must be unique and sorted")
    for record in records:
        declaration = record.get("declaration", {})
        key = declaration.get("identity", {}).get("key")
        for bound in record.get("source_bindings", []):
            if bound.get("target") != {
                "family": identity_family,
                "key": key,
                "revision": "definition-r1",
            }:
                errors.append(f"{key}: source binding target differs from identity")
            if bound.get("source_revision") != summary["source"]["revision"]:
                errors.append(
                    f"{key}: source binding revision differs from the pinned source"
                )
        for pos in positions(declaration):
            if not (
                pos["x"] < summary["map"]["width"]
                and pos["y"] < summary["map"]["height"]
            ):
                errors.append(f"{key}: position outside the source map extent")
    return records


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    summary = load(root, SUMMARY)
    families = {
        family: family_records(root, family, summary, errors) for family in FAMILIES
    }
    city_keys = {r["declaration"]["identity"]["key"] for r in families["Area.City"]}
    item_keys = {row["target"]["key"] for row in load(root, ITEM_BINDINGS)["bindings"]}
    for record in families["House"]:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        if declaration["city"]["key"] not in city_keys:
            errors.append(
                f"{key}: city {declaration['city']['key']} is not a City Area"
            )
        footprint = declaration["footprint"]
        if sum(row["tiles"] for row in footprint["floors"]) != footprint["tile_count"]:
            errors.append(f"{key}: footprint floors do not sum to tile_count")
        floors = [row["floor"] for row in footprint["floors"]]
        if floors != sorted(set(floors)):
            errors.append(f"{key}: footprint floors must be unique and ascending")
        for door in declaration["doors"]:
            pos = door["position"]
            inside = (
                footprint["min_x"] <= pos["x"] <= footprint["max_x"]
                and footprint["min_y"] <= pos["y"] <= footprint["max_y"]
            )
            if not inside or pos["floor"] not in floors:
                errors.append(
                    f"{key}: door {door['door_id']} outside the house footprint"
                )
    for record in families["Transition.Teleport"]:
        declaration = record["declaration"]
        if "object" in declaration and declaration["object"]["key"] not in item_keys:
            errors.append(
                f"{declaration['identity']['key']}: object is not a bound Item"
            )
        if declaration["from"] == declaration["to"]:
            errors.append(f"{declaration['identity']['key']}: teleport to its own tile")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=HERE.parents[2])
    args = parser.parse_args()
    try:
        errors = validate(args.root.resolve())
    except ValidationError as error:
        errors = [str(error)]
    for error in errors:
        print(f"FAIL {error}", file=sys.stderr)
    if errors:
        return 1
    print("PASS world metadata families")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
