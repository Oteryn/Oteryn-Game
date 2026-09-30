"""Structural and semantic validation of the Area authoring schema candidate v1,
never runtime activation."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parent
MONSTER = ROOT.parent / "monster-authoring" / "monster.schema.json"
SCHEMAS = [
    json.loads(path.read_text(encoding="utf-8"))
    for path in (ROOT / "area.schema.json", MONSTER)
]
REGISTRY = Registry().with_resources(
    (s["$id"], Resource.from_contents(s)) for s in SCHEMAS
)
KEY = {
    kind: re.compile(rf"^oteryn:content\.area\.{kind}\.[a-z0-9_]+$")
    for kind in ("region", "subregion", "city")
}


def pointer(path):
    return "/" + "/".join(str(k).replace("~", "~0").replace("/", "~1") for k in path)


def structural(data) -> list[str]:
    validator = Draft202012Validator(SCHEMAS[0], registry=REGISTRY)
    return [
        f"{pointer(e.absolute_path)}: {e.message}"
        for e in sorted(
            validator.iter_errors(data), key=lambda e: list(e.absolute_path)
        )
    ]


def semantic(data) -> list[str]:
    errors: list[str] = []
    areas = data["areas"]
    regions = {a["identity"]["key"] for a in areas if a["kind"] == "region"}
    seen: dict[str, dict[object, int]] = {"key": {}, "source_id": {}, "name": {}}
    for i, area in enumerate(areas):
        at = f"/areas/{i}"
        key, kind = area["identity"]["key"], area["kind"]
        if not KEY[kind].match(key):
            errors.append(f"{at}/identity/key: not an oteryn:content.area.{kind}.* key")
        for field, value in (
            ("key", key),
            ("source_id", area["provenance"]["source_id"]),
            ("name", (kind == "region", area["name"])),
        ):
            if value in seen[field]:
                errors.append(
                    f"{at}: duplicate {field} {value!r} (first at /areas/{seen[field][value]})"
                )
            seen[field].setdefault(value, i)
        parent = area["parent"]
        if kind == "region" and parent is not None:
            errors.append(f"{at}/parent: a region has no parent")
        if kind != "region" and (parent is None or parent["key"] not in regions):
            errors.append(f"{at}/parent: not a region of this catalogue")
        if area["name"] != " ".join(area["provenance"]["source_name"].split()):
            errors.append(f"{at}/name: not the whitespace-normalized source_name")
        hometown, evidence = area["hometown"], area["provenance"]["hometown"]
        if hometown is not None and kind != "city":
            errors.append(f"{at}/hometown: only a city is a hometown")
        if (hometown is None) != (evidence is None):
            errors.append(
                f"{at}/provenance/hometown: must be set exactly with hometown"
            )
        if (
            hometown is not None
            and evidence is not None
            and hometown["temple"] != evidence["engine_town"]["temple"]
            and evidence["owner_check"] is None
        ):
            errors.append(
                f"{at}/hometown/temple: differs from the engine temple without an owner_check"
            )
    return errors


def validate(data) -> list[str]:
    errors = structural(data)
    return errors or semantic(data)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path)
    args = parser.parse_args(argv)
    failed = False
    for path in args.paths:
        errors = validate(json.loads(path.read_text(encoding="utf-8")))
        for error in errors:
            print(f"{path}: {error}", file=sys.stderr)
        failed |= bool(errors)
        if not errors:
            print(f"{path}: ok")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
