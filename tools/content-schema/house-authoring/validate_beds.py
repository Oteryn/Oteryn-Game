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


def load_facts(path=FACTS):
    """item id -> (part, partner_direction); an Item with a null fact is a hold, not a bed."""
    rows = json.loads(path.read_text(encoding="utf-8"))["rows"]
    return {
        int(row["item_key"].rsplit(".i", 1)[1]): (row["part"], row["partner_direction"])
        for row in rows
        if row["part"] and row["partner_direction"]
    }


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


def validate(houses, placed, facts, exceptions):
    """Return `{"errors": [...], "excepted": [...], "pairs": {source_id: n}}`.

    `houses`: dicts with `source_id`, `beds`, `tiles` (x, y, z). `placed`: (item_id, x, y, z).
    `facts`: item id -> (part, partner_direction). `exceptions`: excepted house source ids.
    """
    owner = {}
    for house in houses:
        for tile in house["tiles"]:
            owner.setdefault(tuple(tile), house["source_id"])
    parts = defaultdict(list)  # tile -> [(item_id, part, direction)]
    for item_id, x, y, z in placed:
        if item_id in facts:
            parts[(x, y, z)].append((item_id, *facts[item_id]))
    errors, pairs, bad_houses = [], defaultdict(int), set()

    def fail(code, house, tile, detail=""):
        bad_houses.add(house)
        errors.append(
            {"code": code, "house": house, "tile": list(tile), "detail": detail}
        )

    for tile, here in sorted(parts.items()):
        house = owner.get(tile)
        if house is None:
            continue  # a bed outside every House is not a House bed
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
                fail("NO_MATCHING_PARTNER", house, tile, f"item {item_id} {part}")
            elif len(back) > 1:
                fail("AMBIGUOUS_PARTNER", house, tile, f"item {item_id} {part}")
            elif part == "head":
                pairs[house] += 1
    by_house = {h["source_id"]: h for h in houses}
    excepted = []
    for house_id, house in sorted(by_house.items()):
        found = pairs.get(house_id, 0)
        agrees = found == house["beds"] and house_id not in bad_houses
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
                    {"house": house_id, "beds": house["beds"], "valid_pairs": found}
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
    report = validate(load_houses(), placed, load_facts(), load_exceptions())
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if report["errors"] else 0


if __name__ == "__main__":
    sys.exit(main())
