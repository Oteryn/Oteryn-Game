#!/usr/bin/env python3
"""Structural and semantic validation of the committed world metadata families.

python validate_world_metadata.py [--root REPOSITORY_ROOT]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

HERE = Path(__file__).resolve().parent
SCHEMA = json.loads((HERE / "world-metadata.schema.json").read_text(encoding="utf-8"))
SUMMARY = "tools/content-schema/world-authoring/samples/source-capture-v1.json"
HUNTING_SUMMARY = (
    "tools/content-schema/world-authoring/samples/hunting-places-capture-v1.json"
)
SNAPSHOT = "imports/tibiawiki/hunting-places/fandom-snapshot-v1.json"
WIKI_NAMESPACE = "tibiawiki-fandom/page-id"
CITY_DIRECTORY = "content/world/areas/cities"
CATALOGUES = {
    "content/world/objects": "WorldObject",
    "content/world/terrain": "Terrain",
}
ITEM_DEFINITIONS = "content/items/definitions"
GENERATORS = {
    "Area.HuntingPlace": "tools/content-schema/world-authoring/convert_hunting_places.py",
}
OWN_SOURCE_REVISION = {"Area.HuntingPlace"}
DEFAULT_GENERATOR = "tools/content-schema/world-authoring/convert_world_metadata.py"
FAMILIES = {
    "Area.HuntingPlace": (
        "content/world/areas/hunting-places",
        "hunting-places",
        "Area",
    ),
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
    for name in ("from", "to", "position"):
        if name in declaration:
            yield declaration[name]


def family_records(
    root: Path, family: str, summary: dict, map_extent: dict, errors: list[str]
) -> list[dict]:
    directory, stem, identity_family = FAMILIES[family]
    index_path = f"{directory}/index.json"
    index = load(root, index_path)
    structural(index_path, index, errors)
    if index.get("family") != family:
        errors.append(f"{index_path}: family must be {family}")
    if index.get("source") != summary["source"]:
        errors.append(f"{index_path}: source differs from the capture summary")
    if index.get("generator") != GENERATORS.get(family, DEFAULT_GENERATOR):
        errors.append(f"{index_path}: generator does not belong to {family}")
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
            if (
                family not in OWN_SOURCE_REVISION
                and bound.get("identity_namespace") != WIKI_NAMESPACE
                and bound.get("source_revision") != summary["source"].get("revision")
            ):
                errors.append(
                    f"{key}: source binding revision differs from the pinned source"
                )
        for pos in positions(declaration):
            if not (pos["x"] < map_extent["width"] and pos["y"] < map_extent["height"]):
                errors.append(f"{key}: position outside the source map extent")
    return records


def area_city_records(root: Path) -> list[dict]:
    """The AREAS-1 City Area records (owned by area-authoring, read-only here)."""
    records: list[dict] = []
    for shard in sorted((root / CITY_DIRECTORY).glob("areas-*.json")):
        records.extend(load(root, str(shard.relative_to(root)))["areas"])
    return records


def catalogue_keys(root: Path) -> dict[str, str]:
    """Record key -> family for the WO-2 WorldObject and Terrain catalogues."""
    found: dict[str, str] = {}
    for directory, family in CATALOGUES.items():
        for shard in sorted((root / directory).glob("*-*.json")):
            for record in load(root, str(shard.relative_to(root)))["records"]:
                found[record["identity"]["key"]] = family
    return found


def hunting_places(
    root: Path, summary: dict, families: dict[str, list[dict]], errors: list[str]
) -> None:
    """HuntingPlace semantics: city refs, snapshot pin, page/revision bindings, capture counts."""
    records = families["Area.HuntingPlace"]
    try:
        raw = (root / SNAPSHOT).read_bytes()
        snapshot = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"{SNAPSHOT}: unreadable ({error})")
        return
    if summary["source"].get("snapshot") != {
        "path": SNAPSHOT,
        "sha256": hashlib.sha256(raw).hexdigest(),
    }:
        errors.append(f"{SNAPSHOT}: differs from the sha256 pinned in the capture")
    if raw.decode("utf-8") != canonical(snapshot):
        errors.append(f"{SNAPSHOT}: not canonical JSON")
    pinned = {str(row["pageid"]): str(row["revid"]) for row in snapshot["pages"]}
    city_keys = {a["identity"]["key"] for a in area_city_records(root)}
    city_names = {a["name"].lower() for a in area_city_records(root)}
    bound, with_ = set(), {"city": 0, "creatures": 0, "levels": 0, "position": 0}
    for record in records:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        for row in record["source_bindings"]:
            if pinned.get(row["external_id"]) != row["source_revision"]:
                errors.append(f"{key}: page/revision is not in the pinned snapshot")
            bound.add(row["external_id"])
        if "city" in declaration:
            with_["city"] += 1
            if declaration["city"]["key"] not in city_keys:
                errors.append(
                    f"{key}: city {declaration['city']['key']} is not an AREAS-1 City Area"
                )
        elif (
            declaration.get("source_facts", {}).get("city_name", "").lower()
            in city_names
        ):
            errors.append(f"{key}: city_name matches an AREAS-1 city but is not linked")
        with_["position"] += "position" in declaration
        with_["levels"] += "recommended_levels" in declaration
        with_["creatures"] += "creature_names" in declaration.get("source_facts", {})
        names = declaration.get("source_facts", {}).get("creature_names", [])
        if names != sorted(names):
            errors.append(f"{key}: creature_names must be sorted")
    if bound != set(pinned) or len(bound) != len(records):
        errors.append(
            "Area.HuntingPlace: records must bind each snapshot page exactly once"
        )
    if summary.get("records_with") != with_:
        errors.append("Area.HuntingPlace: capture counts differ from the records")


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    summary = load(root, SUMMARY)
    hunting_summary = load(root, HUNTING_SUMMARY)
    own_summaries = {"Area.HuntingPlace": hunting_summary}
    families = {
        family: family_records(
            root,
            family,
            own_summaries.get(family, summary),
            summary["map"],
            errors,
        )
        for family in FAMILIES
    }
    if any(": schema:" in error for error in errors):
        return errors  # semantic checks assume schema-valid records
    hunting_places(root, hunting_summary, families, errors)
    item_keys = {
        record["definition"]["identity"]["key"]
        for shard in sorted((root / ITEM_DEFINITIONS).glob("items-*.json"))
        for record in load(root, str(shard.relative_to(root)))["records"]
    }
    catalogue = catalogue_keys(root)
    used = {"Item": 0, "Terrain": 0, "WorldObject": 0}
    for record in families["Transition.Teleport"]:
        declaration = record["declaration"]
        key = declaration["identity"]["key"]
        target = declaration.get("object")
        item_id = (
            record["source_bindings"][0]["external_id"].split(":")[1].split("#")[0]
        )
        item_key = f"oteryn:item.tibia.i{item_id}"
        if item_key in item_keys:
            expected = ("Item", item_key)
        else:
            expected = next(
                (
                    (family, f"oteryn:{prefix}.tibia.i{item_id}")
                    for prefix, family in (
                        ("world-object", "WorldObject"),
                        ("terrain", "Terrain"),
                    )
                    if f"oteryn:{prefix}.tibia.i{item_id}" in catalogue
                ),
                None,
            )
        if target is None:
            errors.append(f"{key}: teleport without an object")
            continue
        used[target["family"]] += 1
        if expected is None:
            errors.append(f"{key}: item {item_id} has no Item record or catalogue key")
        elif (target["family"], target["key"]) != expected:
            errors.append(f"{key}: object must be the canonical key {expected[1]}")
        if declaration["from"] == declaration["to"]:
            errors.append(f"{key}: teleport to its own tile")
    if summary.get("teleport_object_families") != used:
        errors.append(
            "Transition.Teleport: object family counts differ from the capture"
        )
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
