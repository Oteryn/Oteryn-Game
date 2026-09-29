"""No-network fixture tests for `client_appearance_census.py` (task B3): the pinned
client-file guard, the engine-defined id union, the appearance-only classification and
the census shape. Run with `python test_client_appearance_census.py`.
"""

from __future__ import annotations

import hashlib
import json
import tempfile
from pathlib import Path

import client_appearance_census as census

CHECKS = 0


def check(condition, message):
    global CHECKS
    CHECKS += 1
    if not condition:
        raise AssertionError(message)


def appearance(item_id, flags=None, name=None):
    return {"id": item_id, "flags": flags or {}, "name": name, "description": None}


def test_client_key_is_provisional():
    key = census.client_key(53000)
    check(key == "client:tibia@15.30-2dfa943b:item/53000", key)
    check(not key.startswith("oteryn:"), key)


def test_client_file_is_pinned_by_size_and_digest():
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "appearances.dat"
        path.write_bytes(b"not the client file")
        for kwargs in ({}, {"size": len(b"not the client file")}):
            try:
                census.load_client_appearances(path, **kwargs)
            except SystemExit:
                continue
            raise AssertionError(f"unpinned client file accepted: {kwargs}")
        check(True, "size and digest both guard the client file")
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        check(digest != census.CLIENT_APPEARANCES_SHA256, digest)


def test_classify_from_appearance_alone():
    corpse = census.classify(appearance(1, {"flags.corpse": True}))
    check(
        corpse == {"outcome": "routed", "owner": "WorldObject", "reason": "corpse"},
        corpse,
    )
    ground = census.classify(appearance(2, {"flags.unmove": True, "flags.bank": True}))
    check(
        ground["owner"] == "Terrain" and ground["reason"] == "ground_or_border", ground
    )
    fixture = census.classify(appearance(3, {"flags.unmove": True}))
    check(fixture["reason"] == "immovable_unclassified", fixture)
    take = census.classify(appearance(4, {"flags.take": True}, name="herbs"))
    check(take == {"outcome": "pickupable_candidate"}, take)
    other = census.classify(appearance(5, {"flags.usable": True}))
    check(other == {"outcome": "unclassified"}, other)


def test_build_census_skips_engine_defined_ids():
    appearances = {
        10: appearance(10, {"flags.take": True}, name="defined elsewhere"),
        52980: appearance(52980, {"flags.take": True, "clothes.slot": 1}, name="hat"),
        52981: appearance(52981, {"flags.unmove": True, "flags.clip": True}),
        30000: appearance(30000, {"flags.usable": True}),
    }
    result = census.build_census(appearances, {10}, {"crystal": 1})
    totals = result["totals"]
    check(totals["undefined_ids"] == 3, totals)
    check(totals["undefined_by_range"] == {"new_15_30": 2, "older": 1}, totals)
    check("10" not in result["items"], "engine-defined id is never censused")
    check(
        result["outcome"]
        == {"pickupable_candidate": 1, "routed": 1, "unclassified": 1},
        result,
    )
    check(
        result["routed"] == {"Terrain:ground_or_border": {"items": 1, "ids": [52981]}},
        result,
    )
    hat = result["items"]["52980"]
    check(hat["name"] == "hat" and hat["outcome"] == "pickupable_candidate", hat)
    check(hat["flags"] == {"flags.take": True, "clothes.slot": 1}, hat)
    check(hat["client_key"].startswith("client:tibia@15.30-"), hat)
    first = census.census_document_bytes(result)
    second = census.census_document_bytes(
        census.build_census(dict(reversed(appearances.items())), {10}, {"crystal": 1})
    )
    check(first == second, "census bytes do not depend on input order")
    check(json.loads(first)["schema"] == "OTERYN_CLIENT_APPEARANCE_CENSUS/v1", "schema")


if __name__ == "__main__":
    for test in (
        test_client_key_is_provisional,
        test_client_file_is_pinned_by_size_and_digest,
        test_classify_from_appearance_alone,
        test_build_census_skips_engine_defined_ids,
    ):
        test()
    print(f"PASS {CHECKS} checks")
