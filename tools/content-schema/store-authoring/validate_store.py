"""Structural and semantic validation of the Store offer authoring schema candidate v1,
never runtime activation."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource
from store_constants import OFFER_TYPES

ROOT = Path(__file__).resolve().parent
MONSTER = ROOT.parent / "monster-authoring" / "monster.schema.json"
SCHEMA_FILES = {
    "store-offer.schema.json": ROOT / "store-offer.schema.json",
    "monster.schema.json": MONSTER,
}
SCHEMAS = {
    name: json.loads(path.read_text(encoding="utf-8"))
    for name, path in SCHEMA_FILES.items()
}
REGISTRY = Registry().with_resources(
    (s["$id"], Resource.from_contents(s)) for s in SCHEMAS.values()
)


def pointer(path):
    return "/" + "/".join(str(k).replace("~", "~0").replace("/", "~1") for k in path)


def structural(data) -> list[str]:
    validator = Draft202012Validator(
        SCHEMAS["store-offer.schema.json"], registry=REGISTRY
    )
    return [
        f"{pointer(e.absolute_path)}: {e.message}"
        for e in sorted(
            validator.iter_errors(data), key=lambda e: list(e.absolute_path)
        )
    ]


def semantic(data) -> list[str]:
    errors = []
    categories = data["categories"]
    offers = data["offers"]

    cat_by_key = {}
    for i, c in enumerate(categories):
        key = c["identity"]["key"]
        if key in cat_by_key:
            errors.append(f"/categories/{i}: duplicate category key {key!r}")
        cat_by_key[key] = c

    for i, c in enumerate(categories):
        parent = c["parent"]
        if parent is not None and parent not in cat_by_key:
            errors.append(
                f"/categories/{i}: parent {parent!r} does not resolve to a category"
            )
        if parent is not None and cat_by_key.get(parent, {}).get("shape") != "group":
            errors.append(f"/categories/{i}: parent {parent!r} is not a group category")
        if c["shape"] == "leaf" and c["subclass_names"] is not None:
            errors.append(
                f"/categories/{i}: leaf category must not carry subclass_names"
            )
        if c["shape"] == "group" and not c["subclass_names"]:
            errors.append(
                f"/categories/{i}: group category must carry non-empty subclass_names"
            )

    offer_keys = set()
    for i, o in enumerate(offers):
        key = o["identity"]["key"]
        if key in offer_keys:
            errors.append(f"/offers/{i}: duplicate offer key {key!r}")
        offer_keys.add(key)
        cat_key = o["category"]
        cat = cat_by_key.get(cat_key)
        if cat is None:
            errors.append(f"/offers/{i}: category {cat_key!r} does not resolve")
        elif cat["shape"] != "leaf":
            errors.append(
                f"/offers/{i}: category {cat_key!r} is a group category, cannot carry offers"
            )

        offer_type = o["offer_type"]
        expected_kind = OFFER_TYPES.get(offer_type, (None, None, None, None))[2]
        actual_kind = o["product"]["kind"]
        # `service` offer types never carry a product other than `service`; every other
        # offer_type maps to exactly one product kind (store_constants.OFFER_TYPES).
        if expected_kind is not None and actual_kind != expected_kind:
            errors.append(
                f"/offers/{i}: offer_type {offer_type!r} expects product.kind {expected_kind!r}, found {actual_kind!r}"
            )

        if actual_kind == "item":
            item = o["product"]["item"]
            non_null = [k for k in ("count", "charges") if item[k] is not None]
            if len(non_null) != 1:
                errors.append(
                    f"/offers/{i}: product.item must set exactly one of count/charges, found {non_null}"
                )
            if offer_type == "item_bed" and len(item["item_refs"]) != 2:
                errors.append(
                    f"/offers/{i}: item_bed must resolve exactly 2 item_refs (two unwrap boxes)"
                )
            if offer_type != "item_bed" and len(item["item_refs"]) != 1:
                errors.append(
                    f"/offers/{i}: offer_type {offer_type!r} must resolve exactly 1 item_ref"
                )
            if offer_type == "charges" and item["charges"] is None:
                errors.append(
                    f"/offers/{i}: offer_type charges must set product.item.charges"
                )
            if (
                offer_type in ("stackable", "house", "item_unique")
                and item["count"] is None
            ):
                errors.append(
                    f"/offers/{i}: offer_type {offer_type!r} must set product.item.count"
                )
    return errors


def validate(data) -> list[str]:
    errs = structural(data)
    if errs:
        return errs
    return semantic(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "catalog",
        help="a converter output ({catalog, conversion_report}) or bare {categories, offers} JSON file",
    )
    args = parser.parse_args()
    raw = json.loads(Path(args.catalog).read_text(encoding="utf-8"))
    data = raw["catalog"] if "catalog" in raw and "categories" not in raw else raw
    errors = validate(data)
    if errors:
        for e in errors:
            print(e, file=sys.stderr)
        print(f"FAIL: {len(errors)} error(s)", file=sys.stderr)
        sys.exit(1)
    print(f"PASS: {len(data['categories'])} categories, {len(data['offers'])} offers")


if __name__ == "__main__":
    main()
