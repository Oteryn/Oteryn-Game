"""Structural and semantic validation of the House authoring schema candidate v1,
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
SCHEMA_FILES = {
    "house.schema.json": ROOT / "house.schema.json",
    "monster.schema.json": MONSTER,
}
SCHEMAS = {
    name: json.loads(path.read_text(encoding="utf-8"))
    for name, path in SCHEMA_FILES.items()
}
REGISTRY = Registry().with_resources(
    (s["$id"], Resource.from_contents(s)) for s in SCHEMAS.values()
)
HOUSE_KEY = re.compile(r"^oteryn:content\.house\.[a-z0-9_]+$")
# The only restriction text observed in the 15.30 client; anything else must be
# modeled deliberately rather than guessed.
RESTRICTION_TEXT = {
    "": None,
    "Only Sorcerers can enter.": {"vocations": ["sorcerer"]},
}


def pointer(path):
    return "/" + "/".join(str(k).replace("~", "~0").replace("/", "~1") for k in path)


def structural(data) -> list[str]:
    validator = Draft202012Validator(SCHEMAS["house.schema.json"], registry=REGISTRY)
    return [
        f"{pointer(e.absolute_path)}: {e.message}"
        for e in sorted(
            validator.iter_errors(data), key=lambda e: list(e.absolute_path)
        )
    ]


def inside(position, footprint) -> bool:
    origin = footprint["origin"]
    return (
        origin["x"] <= position["x"] < origin["x"] + footprint["width"]
        and origin["y"] <= position["y"] < origin["y"] + footprint["height"]
        and origin["z"] <= position["z"] < origin["z"] + footprint["floors"]
    )


def semantic(data) -> list[str]:
    errors: list[str] = []
    seen: dict[str, dict[object, int]] = {
        "key": {},
        "source_id": {},
        "engine_house_id": {},
        "name": {},
    }
    for i, house in enumerate(data["houses"]):
        at = f"/houses/{i}"
        key = house["identity"]["key"]
        if not HOUSE_KEY.match(key):
            errors.append(f"{at}/identity/key: not an oteryn:content.house.* key")
        for field, value in (
            ("key", key),
            ("source_id", house["provenance"]["source_id"]),
            ("engine_house_id", house["provenance"]["engine_house"]["house_id"]),
            ("name", house["name"]),
        ):
            if value in seen[field]:
                errors.append(
                    f"{at}: duplicate {field} {value!r} (first at /houses/{seen[field][value]})"
                )
            seen[field].setdefault(value, i)
        if not inside(house["map_marker"], house["footprint"]):
            errors.append(f"{at}/map_marker: outside the footprint")
        if house["name"] != " ".join(house["provenance"]["source_name"].split()):
            errors.append(f"{at}/name: not the whitespace-normalized source_name")
        text = house["provenance"]["restrictions_text"]
        if text not in RESTRICTION_TEXT:
            errors.append(
                f"{at}/provenance/restrictions_text: unmodeled restriction {text!r}"
            )
        elif RESTRICTION_TEXT[text] != house["entry_restriction"]:
            errors.append(f"{at}/entry_restriction: does not match restrictions_text")
        if house["kind"] == "shop" and "(Shop)" not in house["name"]:
            errors.append(f"{at}/kind: shop without '(Shop)' in its name")
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
