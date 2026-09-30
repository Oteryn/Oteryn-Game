"""Derive and verify the staticmapdata cell order against CrystalServer map House tiles.

Also compares the House doors (cells holding a `type="door"` item) with the engine map.

Local-only evidence tool (the pinned map is 53 MB and not fetched by CI). Reads the
pinned gzip OTBM `data-global/world/world.otbm` of crystalserver@00ce02a5, collects
every House tile (OTBM node 14: position, engine House id, ground and item ids),
scores all 24 candidate cell orders (axis nesting x z-direction x skip before/after)
and writes samples/otbm-tile-check.json. The winning order is what
convert_houses.layout_tiles implements.
"""

from __future__ import annotations

import argparse
import gzip
import itertools
import json
import re
import struct
import sys
from collections import Counter, defaultdict
from pathlib import Path

from convert_houses import (
    CRYSTAL_REVISION,
    CRYSTAL_SAMPLE,
    DOOR_ITEMS,
    ROOT,
    dump,
    layout_doors,
    layout_tiles,
    load_staged,
    sha256,
)

OTBM_SHA256 = "dcb735549bd11de526c4bd441bbf62e4490efbb60ff7aa334530f1692345d8d7"
OUT = ROOT / "samples" / "otbm-tile-check.json"
SPECIAL = re.compile(rb"[\xfd\xfe\xff]")
TILE_AREA, ITEM, HOUSE_TILE = 4, 6, 14
ATTR_TILE_FLAGS, ATTR_ITEM = 3, 9


def house_tiles(raw: bytes) -> dict[int, dict[tuple, list[int]]]:
    """{engine House id: {(x, y, z): [ground, item ids...]}} from an OTBM byte stream."""
    houses: dict[int, dict[tuple, list[int]]] = defaultdict(dict)
    stack: list[list] = []
    pos = 4  # file identifier
    while (m := SPECIAL.search(raw, pos)) is not None:
        j = m.start()
        if stack:
            stack[-1][1] += raw[pos:j]
        if raw[j] == 0xFD:  # escape
            stack[-1][1].append(raw[j + 1])
            pos = j + 2
        elif raw[j] == 0xFE:  # node start: [type, props, child item ids]
            stack.append([raw[j + 1], bytearray(), []])
            pos = j + 2
        else:  # node end
            kind, props, children = stack.pop()
            pos = j + 1
            if kind == ITEM and stack:
                stack[-1][2].append(struct.unpack_from("<H", props)[0])
            elif kind == HOUSE_TILE:
                if stack[-1][0] != TILE_AREA:
                    raise ValueError("House tile outside a tile area")
                bx, by, bz = struct.unpack_from("<HHB", stack[-1][1])
                x, y, house_id = struct.unpack_from("<BBI", props)
                ground, k = [], 6
                while k < len(props) and props[k] in (ATTR_TILE_FLAGS, ATTR_ITEM):
                    if props[k] == ATTR_ITEM:
                        ground = [struct.unpack_from("<H", props, k + 1)[0]]
                    k += 5 if props[k] == ATTR_TILE_FLAGS else 3
                houses[house_id][(bx + x, by + y, bz)] = ground + children
    if stack:
        raise ValueError("unterminated OTBM node")
    return houses


def candidate_cells(layout: dict, order: str, z_up: bool, skip_after: bool):
    """{position: items} for one candidate cell order."""
    origin, dims = layout["origin"], layout["dimensions"]
    size = {"x": dims["width"], "y": dims["height"], "z": dims["floors"]}
    axes = [
        range(size[a] - 1, -1, -1) if a == "z" and not z_up else range(size[a])
        for a in order
    ]
    positions = [
        tuple(origin[a] + dict(zip(order, v))[a] for a in "xyz")
        for v in itertools.product(*axes)
    ]
    cells, index = {}, 0
    for cell in layout["cells"]:
        index += 0 if skip_after else cell["skip"]
        cells[positions[index]] = cell["items"]
        index += 1 + (cell["skip"] if skip_after else 0)
    return cells


def check(staged: list[dict], crystal: dict, tiles: dict, door_items: set[int]) -> dict:
    engine = {r["client_id"]: r["house_id"] for r in crystal["records"]}
    scores = {}
    for order, z_up, skip_after in itertools.product(
        ("".join(p) for p in itertools.permutations("xyz")),
        (True, False),
        (True, False),
    ):
        hit = 0
        for record in staged:
            engine_tiles = tiles.get(engine[record["source_id"]], {})
            for p, items in candidate_cells(
                record["layout"], order, z_up, skip_after
            ).items():
                hit += bool(items) and engine_tiles.get(p, [None])[0] in items
        name = f"{order}|z_{'up' if z_up else 'down'}|skip_{'after' if skip_after else 'before'}"
        scores[name] = hit
    totals, uncovered = Counter(), []
    for record in staged:
        engine_tiles = tiles.get(engine[record["source_id"]], {})
        client = {tuple(t) for t in layout_tiles(record["layout"])}
        covered = sum(p in client for p in engine_tiles)
        totals["engine_house_tiles"] += len(engine_tiles)
        totals["client_house_tiles"] += len(client)
        totals["engine_tiles_on_client_tiles"] += covered
        totals["houses_identical_tile_sets"] += client == set(engine_tiles)
        client_doors = {tuple(d) for d in layout_doors(record["layout"], door_items)}
        engine_doors = {
            p for p, items in engine_tiles.items() if door_items & set(items)
        }
        totals["client_doors"] += len(client_doors)
        totals["engine_doors"] += len(engine_doors)
        totals["engine_doors_on_client_doors"] += len(engine_doors & client_doors)
        totals["houses_identical_door_sets"] += client_doors == engine_doors
        if engine_tiles and covered / len(engine_tiles) < 0.9:
            uncovered.append(record["source_id"])
    best = max(scores, key=scores.get)
    return {
        "engine": "crystalserver",
        "revision": CRYSTAL_REVISION,
        "otbm_sha256": OTBM_SHA256,
        "ground_match_by_order": dict(sorted(scores.items(), key=lambda kv: -kv[1])),
        "selected_order": best,
        "totals": dict(sorted(totals.items())),
        "houses_below_90_percent_engine_coverage": sorted(uncovered),
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--otbm", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    packed = args.otbm.read_bytes()
    if sha256(packed) != OTBM_SHA256:
        raise ValueError("world.otbm digest mismatch")
    crystal = json.loads(CRYSTAL_SAMPLE.read_text(encoding="utf-8"))
    door_items = set(
        json.loads(DOOR_ITEMS.read_text(encoding="utf-8"))["door_item_ids"]
    )
    result = check(
        load_staged(), crystal, house_tiles(gzip.decompress(packed)), door_items
    )
    if result["selected_order"] != "zxy|z_up|skip_after":
        raise ValueError(f"cell order changed: {result['selected_order']}")
    text = dump(result)
    if args.check:
        ok = OUT.read_text(encoding="utf-8") == text
        print(f"{OUT.name}: {'ok' if ok else 'differs'}")
        return 0 if ok else 1
    OUT.write_text(text, encoding="utf-8")
    print(json.dumps(result["totals"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
