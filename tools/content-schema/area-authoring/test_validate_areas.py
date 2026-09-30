"""Positive and negative cases for the Area authoring schema candidate v1, on the built
catalogue: each mutation must be rejected for the intended reason."""

import copy
import sys

from build_areas import SCHEMA, build
from validate_areas import validate

AREAS, _ = build()
CATALOG = {"schema": SCHEMA, "areas": AREAS}


def first(data, kind, hometown=None):
    for area in data["areas"]:
        if area["kind"] == kind and (
            hometown is None or bool(area["hometown"]) == hometown
        ):
            return area
    raise LookupError(kind)


def mutate(fn):
    data = copy.deepcopy(CATALOG)
    fn(data)
    return data


NEGATIVE = {
    "unknown field": (
        lambda d: first(d, "region").update(owner="x"),
        "Additional properties",
    ),
    "bad kind": (lambda d: first(d, "region").update(kind="island"), "is not one of"),
    "key of another kind": (
        lambda d: first(d, "subregion")["identity"].update(
            key="oteryn:content.area.region.x"
        ),
        "not an oteryn:content.area.subregion.* key",
    ),
    "duplicate key": (
        lambda d: d["areas"][1]["identity"].update(
            key=d["areas"][0]["identity"]["key"]
        ),
        "duplicate key",
    ),
    "duplicate source id": (
        lambda d: d["areas"][1]["provenance"].update(
            source_id=d["areas"][0]["provenance"]["source_id"]
        ),
        "duplicate source_id",
    ),
    "region with a parent": (
        lambda d: first(d, "region").update(parent=first(d, "city")["parent"]),
        "a region has no parent",
    ),
    "subregion without a parent": (
        lambda d: first(d, "subregion").update(parent=None),
        "not a region of this catalogue",
    ),
    "parent not in the catalogue": (
        lambda d: first(d, "city")["parent"].update(
            key="oteryn:content.area.region.nowhere"
        ),
        "not a region of this catalogue",
    ),
    "name not from source": (
        lambda d: first(d, "region")["provenance"].update(source_name="Other"),
        "whitespace-normalized",
    ),
    "hometown on a subregion": (
        lambda d: first(d, "subregion").update(
            hometown=first(d, "city", True)["hometown"]
        ),
        "only a city is a hometown",
    ),
    "hometown without evidence": (
        lambda d: first(d, "city", True)["provenance"].update(hometown=None),
        "must be set exactly with hometown",
    ),
    "temple moved without an owner check": (
        lambda d: first(d, "city", True)["hometown"]["temple"].update(x=1),
        "differs from the engine temple without an owner_check",
    ),
    "floor out of range": (
        lambda d: first(d, "city", True)["hometown"]["temple"].update(z=16),
        "is not valid under any of the given schemas",
    ),
}


def main() -> int:
    failures = []
    errors = validate(CATALOG)
    if errors:
        failures.append(f"positive: {errors[:5]}")
    for name, (fn, fragment) in NEGATIVE.items():
        errors = validate(mutate(fn))
        if not any(fragment in e for e in errors):
            failures.append(f"{name}: expected {fragment!r}, got {errors[:3]}")
    for failure in failures:
        print(failure, file=sys.stderr)
    print(f"{1 + len(NEGATIVE) - len(failures)}/{1 + len(NEGATIVE)} cases ok")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
