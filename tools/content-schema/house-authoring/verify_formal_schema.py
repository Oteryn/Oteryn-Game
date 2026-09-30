"""Focused positive/negative cases for the House authoring schema candidate v1.

Regenerates `synthetic-valid-house.json` (a small hand-built catalog, not transcribed
from any source) and checks that each mutation is rejected for the intended reason.
"""

import copy
import json
import sys
from pathlib import Path

from validate_houses import validate

ROOT = Path(__file__).resolve().parent
SHA = "0" * 64
TILES = [[1000, 1000, 7], [1000, 1001, 7], [1001, 1000, 7], [1001, 1001, 7]]


def house(slug, name, source_id, engine_id, kind="private_house", restriction=""):
    return {
        "identity": {
            "key": f"oteryn:content.house.{slug}",
            "revision": "definition-r1",
        },
        "name": name,
        "kind": kind,
        "town": {"family": "Area", "key": "oteryn:content.area.city.example_town"},
        "entrance": {"x": 1000, "y": 1000, "z": 7},
        "map_marker": {"x": 1002, "y": 1001, "z": 7},
        "size_sqm": 20,
        "beds": 0 if kind == "shop" else 2,
        "rent_gold": 50000,
        "entry_restriction": {"vocations": ["sorcerer"]} if restriction else None,
        "footprint": {
            "origin": {"x": 999, "y": 999, "z": 6},
            "width": 6,
            "height": 4,
            "floors": 2,
        },
        "tiles": TILES,
        "doors": [TILES[source_id - 1]],
        "provenance": {
            "source": "cipsoft/staticdata/house_id",
            "client_version": "15.30",
            "source_id": source_id,
            "source_name": name,
            "staticdata_sha256": SHA,
            "staticmapdata_sha256": SHA,
            "restrictions_text": restriction,
            "engine_house": {
                "engine": "crystalserver",
                "revision": "a" * 40,
                "house_id": engine_id,
            },
        },
    }


CATALOG = {
    "schema": "OTERYN_HOUSE_AUTHORING/candidate-1",
    "houses": [
        house("example_lane_1", "Example Lane 1", 1, 11),
        house("example_lane_2_shop", "Example Lane 2 (Shop)", 2, 12, kind="shop"),
        house(
            "mage_row_1", "Mage Row 1", 3, 13, restriction="Only Sorcerers can enter."
        ),
    ],
}


def mutate(fn):
    data = copy.deepcopy(CATALOG)
    fn(data)
    return data


def h0(data):
    return data["houses"][0]


NEGATIVE = {
    "unknown field": (lambda d: h0(d).update(owner="someone"), "Additional properties"),
    "runtime rent state": (
        lambda d: h0(d).update(rent_paid_until=0),
        "Additional properties",
    ),
    "bad kind": (lambda d: h0(d).update(kind="castle"), "is not one of"),
    "negative rent": (lambda d: h0(d).update(rent_gold=-1), "less than the minimum"),
    "zero size": (lambda d: h0(d).update(size_sqm=0), "less than the minimum"),
    "bad town key": (
        lambda d: h0(d)["town"].update(key="oteryn:content.area.region.x"),
        "does not match",
    ),
    "z out of range": (
        lambda d: h0(d)["entrance"].update(z=16),
        "greater than the maximum",
    ),
    "unknown vocation": (
        lambda d: h0(d).update(entry_restriction={"vocations": ["wizard"]}),
        "is not valid",
    ),
    "double space name": (
        lambda d: h0(d).update(name="Example  Lane 1"),
        "does not match",
    ),
    "wrong key family": (
        lambda d: h0(d)["identity"].update(key="oteryn:content.npc.example"),
        "not an oteryn:content.house.* key",
    ),
    "duplicate key": (
        lambda d: d["houses"][1]["identity"].update(
            key="oteryn:content.house.example_lane_1"
        ),
        "duplicate key",
    ),
    "duplicate source id": (
        lambda d: d["houses"][1]["provenance"].update(source_id=1),
        "duplicate source_id",
    ),
    "duplicate engine id": (
        lambda d: d["houses"][1]["provenance"]["engine_house"].update(house_id=11),
        "duplicate engine_house_id",
    ),
    "marker outside footprint": (
        lambda d: h0(d)["map_marker"].update(x=2000),
        "map_marker: outside",
    ),
    "name not from source": (
        lambda d: h0(d)["provenance"].update(source_name="Other 1"),
        "whitespace-normalized",
    ),
    "unmodeled restriction": (
        lambda d: h0(d)["provenance"].update(
            restrictions_text="Only Knights can enter."
        ),
        "unmodeled restriction",
    ),
    "restriction mismatch": (
        lambda d: d["houses"][2].update(entry_restriction=None),
        "does not match",
    ),
    "shop name": (lambda d: h0(d).update(kind="shop"), "shop without"),
    "no tiles": (lambda d: h0(d).update(tiles=[]), "should be non-empty"),
    "tile arity": (lambda d: h0(d)["tiles"].append([1, 2]), "is too short"),
    "duplicate tile": (lambda d: h0(d)["tiles"].append([1001, 1001, 7]), "non-unique"),
    "tile outside footprint": (
        lambda d: h0(d)["tiles"].append([2000, 1001, 7]),
        "tiles: a tile is outside",
    ),
    "unsorted tiles": (lambda d: h0(d)["tiles"].reverse(), "tiles: not sorted"),
    "no doors": (lambda d: h0(d).update(doors=[]), "should be non-empty"),
    "door off the tiles": (
        lambda d: h0(d).update(doors=[[1002, 1000, 7]]),
        "doors: a door is not on a House tile",
    ),
    "unsorted doors": (
        lambda d: h0(d).update(doors=[[1001, 1001, 7], [1000, 1000, 7]]),
        "doors: not sorted",
    ),
    "door in two houses": (
        lambda d: d["houses"][1].update(doors=[[1000, 1000, 7]]),
        "also belongs to /houses/0",
    ),
}


def main() -> int:
    (ROOT / "synthetic-valid-house.json").write_text(
        json.dumps(CATALOG, indent=2) + "\n", encoding="utf-8"
    )
    failures = []
    errors = validate(CATALOG)
    if errors:
        failures.append(f"positive: {errors}")
    for name, (fn, fragment) in NEGATIVE.items():
        errors = validate(mutate(fn))
        if not any(fragment in e for e in errors):
            failures.append(f"{name}: expected {fragment!r}, got {errors}")
    for failure in failures:
        print(failure, file=sys.stderr)
    print(f"{1 + len(NEGATIVE) - len(failures)}/{1 + len(NEGATIVE)} cases ok")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
