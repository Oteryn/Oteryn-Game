"""BED-0 §3 bed validator (BED-CONTENT-1): every placed bed in a bundle is one head and one foot.

For each bed part placed on a House tile it checks that the part names a partner through its
`partner_direction`, that a part of the other kind is placed on that tile and names it back, and
that both parts lie on tiles of the same House. A House's valid pairs are then compared with its
catalogue `beds` count. The Houses in `bed-exception-houses.json` (the known-discrepancy houses of
`samples/otbm-tile-check.json`) are reported with their valid pairs instead of failing; a listed
House that now agrees is an error, because it must leave the list. Any other mismatch fails.

Pure over its inputs so a fixture House can test it; `main` reads a placed-bed census
(`{"beds": [{"item_id", "x", "y", "z"}]}`), the House catalogue and the bed fact packet.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
EXCEPTIONS = ROOT / "bed-exception-houses.json"
FACTS = REPO / "docs" / "agents" / "evidence" / "OTV2-20261005-bed-facts-v1.json"
HOUSE_SHARDS = sorted((REPO / "content" / "houses").glob("houses-*.json"))
STEP = {"north": (0, -1), "south": (0, 1), "east": (1, 0), "west": (-1, 0)}
OTHER = {"head": "foot", "foot": "head"}


def load_exceptions(path=EXCEPTIONS):
    return set(json.loads(path.read_text(encoding="utf-8"))["source_ids"])


def _item_id(key):
    return int(key.rsplit(".i", 1)[1])


def load_facts(path=FACTS):
    """item id -> (part, partner_direction), lowercase, for every Item carrying group 19."""
    promotions = json.loads(path.read_text(encoding="utf-8"))["promotions"]
    return {
        _item_id(row["item_key"]): (
            row["typed_value"]["value"]["part"].lower(),
            row["typed_value"]["value"]["partner_direction"].lower(),
        )
        for row in promotions
    }


def load_held(path=FACTS):
    """Canary bed Item ids the packet holds: they carry no group 19."""
    holds = json.loads(path.read_text(encoding="utf-8"))["holds"]
    return {_item_id(row["item_key"]) for row in holds}


def load_houses(shards=None):
    houses = []
    for shard in HOUSE_SHARDS if shards is None else shards:
        for house in json.loads(Path(shard).read_text(encoding="utf-8"))["houses"]:
            houses.append(
                {
                    "source_id": house["provenance"]["source_id"],
                    "beds": house["beds"],
                    "tiles": [tuple(tile) for tile in house["tiles"]],
                }
            )
    return houses


def validate(houses, placed, facts, exceptions, held=frozenset()):
    """Return `{"errors": [...], "excepted": [...], "pairs": {source_id: n}}`.

    `houses`: dicts with `source_id`, `beds`, `tiles` (x, y, z). `placed`: (item_id, x, y, z).
    `facts`: item id -> (part, partner_direction). `exceptions`: excepted house source ids.
    `held`: bed Item ids without group 19; a placed one is a PART_WITHOUT_GROUP_19 finding.
    A census id in neither `facts` nor `held` is an UNKNOWN_BED_ITEM finding.
    """
    owner = {}
    for house in houses:
        for tile in house["tiles"]:
            owner.setdefault(tuple(tile), house["source_id"])
    parts = defaultdict(list)  # tile -> [(item_id, part, direction)]
    errors, pairs, bad_houses = [], defaultdict(int), set()
    for item_id, x, y, z in placed:
        if item_id in held:
            house = owner.get((x, y, z))
            if house is not None:
                bad_houses.add(house)
                errors.append(
                    {
                        "code": "PART_WITHOUT_GROUP_19",
                        "house": house,
                        "tile": [x, y, z],
                        "detail": f"item {item_id} is a bed part without group 19",
                    }
                )
        elif item_id in facts:
            parts[(x, y, z)].append((item_id, *facts[item_id]))
        else:  # fail closed: a census bed id with neither group 19 facts nor a hold
            house = owner.get((x, y, z))
            if house is not None:
                bad_houses.add(house)
                errors.append(
                    {
                        "code": "UNKNOWN_BED_ITEM",
                        "house": house,
                        "tile": [x, y, z],
                        "detail": f"item {item_id} is in neither the facts nor the held set",
                    }
                )

    def fail(code, house, tile, detail=""):
        bad_houses.add(house)
        errors.append(
            {"code": code, "house": house, "tile": list(tile), "detail": detail}
        )

    soft = defaultdict(list)  # exception house -> structural pair findings (BED-0 §3)

    def pair_fail(code, house, tile, detail):
        if (
            house in exceptions
        ):  # known-discrepancy house: report it, offer valid pairs only
            soft[house].append({"code": code, "tile": list(tile), "detail": detail})
        else:
            fail(code, house, tile, detail)

    for tile, here in sorted(parts.items()):
        house = owner.get(tile)
        if house is None:
            continue  # a bed outside every House is not a House bed
        heads = [item_id for item_id, part, _ in here if part == "head"]
        if len(heads) > 1:  # BedKey is keyed by the head tile: two heads would collide
            fail("MULTIPLE_HEADS_ON_TILE", house, tile, f"items {sorted(heads)}")
        for item_id, part, direction in here:
            dx, dy = STEP[direction]
            other_tile = (tile[0] + dx, tile[1] + dy, tile[2])
            other_house = owner.get(other_tile)
            if other_house != house:
                fail(
                    "PARTNER_OUTSIDE_HOUSE",
                    house,
                    tile,
                    f"item {item_id} -> {other_house}",
                )
                continue
            back = [
                p
                for p in parts.get(other_tile, [])
                if p[1] == OTHER[part] and STEP[p[2]] == (-dx, -dy)
            ]
            if not back:
                pair_fail("NO_MATCHING_PARTNER", house, tile, f"item {item_id} {part}")
            elif len(back) > 1:
                pair_fail("AMBIGUOUS_PARTNER", house, tile, f"item {item_id} {part}")
            elif part == "head":
                pairs[house] += 1
    by_house = {h["source_id"]: h for h in houses}
    excepted = []
    for house_id, house in sorted(by_house.items()):
        found = pairs.get(house_id, 0)
        agrees = (
            found == house["beds"]
            and house_id not in bad_houses
            and house_id not in soft
        )
        if house_id in exceptions:
            if agrees:
                errors.append(
                    {
                        "code": "STALE_EXCEPTION",
                        "house": house_id,
                        "tile": [],
                        "detail": "now agrees; remove it from bed-exception-houses.json",
                    }
                )
            else:
                excepted.append(
                    {
                        "house": house_id,
                        "beds": house["beds"],
                        "valid_pairs": found,
                        "pair_findings": soft.get(house_id, []),
                    }
                )
        elif found != house["beds"]:
            errors.append(
                {
                    "code": "BED_COUNT_MISMATCH",
                    "house": house_id,
                    "tile": [],
                    "detail": f"catalogue {house['beds']} valid pairs {found}",
                }
            )
    for house_id in sorted(exceptions - set(by_house)):
        errors.append(
            {
                "code": "UNKNOWN_EXCEPTION_HOUSE",
                "house": house_id,
                "tile": [],
                "detail": "",
            }
        )
    return {
        "errors": errors,
        "excepted": excepted,
        "pairs": dict(sorted(pairs.items())),
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "census", type=Path, help='{"beds": [{"item_id", "x", "y", "z"}]}'
    )
    args = parser.parse_args(argv)
    census = json.loads(args.census.read_text(encoding="utf-8"))["beds"]
    placed = [(b["item_id"], b["x"], b["y"], b["z"]) for b in census]
    report = validate(
        load_houses(), placed, load_facts(), load_exceptions(), load_held()
    )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if report["errors"] else 0


if __name__ == "__main__":
    sys.exit(main())
