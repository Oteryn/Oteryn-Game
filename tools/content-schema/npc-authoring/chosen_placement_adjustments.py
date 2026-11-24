"""Explicit Oteryn adjacent-cell choices; retained donor coordinates remain evidence."""

import hashlib
import json
import sys
from functools import lru_cache
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "world-authoring"))
import world_region_codec as codec
from scoped_spawn_ground import captured_grounds

MAX_NEAREST_CLEAR_DISTANCE = 4
FLAG_PROTECTION_ZONE = 1


@lru_cache(maxsize=1)
def ground_slots(root):
    palette = json.loads((root / "content/world/placements/index.json").read_bytes())[
        "palette"
    ]
    grounds, scoped = set(), set()
    for path in sorted((root / "content/world/terrain").glob("terrain-*.json")):
        for row in json.loads(path.read_bytes())["records"]:
            if row["kind"].get("value") == "ground" and row.get("walkable") == {
                "state": "KNOWN",
                "value": True,
            }:
                if row["provenance"]["route"]["reason"] == "scoped_spawn_ground":
                    scoped.add(row["identity"]["key"])
                else:
                    grounds.add(row["provenance"]["source_item_id"])
    return grounds, scoped, palette


def ground_item(root, palette_index, cell):
    grounds, scoped, palette = ground_slots(root)
    entry = palette[palette_index]
    item = entry["source_item_id"]
    if entry.get("scoped_spawn_ground"):
        return entry["key"] in scoped and captured_grounds(root).get(cell) == item
    return item in grounds


def choose_cell(root, actual_source, choice, occupied, *, compiler_qualified=False):
    fields = {'source', 'target', 'reason'}
    if set(choice) not in (fields, fields | {'blocked_source'}):
        raise ValueError('chosen placement requires source, target and reason')
    def point(value):
        if set(value) != {"x", "y", "z"} or any(
            type(v) is not int for v in value.values()
        ):
            raise ValueError("invalid chosen placement coordinates")
        x, y, z = value["x"], value["y"], value["z"]
        if not (0 <= x <= 65535 and 0 <= y <= 65535 and 0 <= z <= 15):
            raise ValueError("chosen placement outside native frame")
        return x, y, z

    before, after = point(choice["source"]), point(choice["target"])
    if before != actual_source or before[2] != after[2] or after in occupied:
        raise ValueError("chosen placement must adjoin its exact retained source cell")
    if not isinstance(choice["reason"], str) or not 1 <= len(choice["reason"]) <= 512:
        raise ValueError("chosen placement target occupied or decision missing")
    x, y, z = after
    path = f"content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3"
    raw = (root / path).read_bytes()
    tiles = {t[:2]: t for _, sector in codec.decode_region(raw)[3] for t in sector}
    source_tile = tiles.get(before[:2])
    allowed_flags = source_tile[2] & FLAG_PROTECTION_ZONE if source_tile is not None else 0

    def clear(cell, walkable=False):
        tile = tiles.get(cell)
        return (tile is not None
                and not (tile[2] & ~allowed_flags)
                and not tile[3] and len(tile[5]) == 1
                and (not walkable or ground_item(root, tile[5][0][0], (*cell, z))))
    distance = sum(abs(a-b) for a, b in zip(before[:2], after[:2]))
    source_tile, target_tile = tiles.get(before[:2]), tiles.get(after[:2])
    # A reserved arrival can share its protection zone and floor decoration
    # with its neighbor. The bundle compiler still qualifies the full stack.
    same_walkable_surface = (
        distance == 1 and source_tile is not None and target_tile is not None
        and source_tile[2:] == target_tile[2:]
        and not source_tile[3] and bool(source_tile[5])
        and ground_item(root, source_tile[5][0][0], before)
    )
    if compiler_qualified:
        # The mainland compiler regression checks these decorated contact tiles
        # with the same resolver used by the published bundle.
        tile = tiles.get(after[:2])
        if not 1 <= distance <= MAX_NEAREST_CLEAR_DISTANCE or tile is None or tile[3] or not tile[5]:
            raise ValueError('qualified contact target must be a nearby native tile outside houses')
    elif distance != 1:
        # An absent or explicitly blocked source can use the uniquely nearest clear tile.
        source_present = before[:2] in tiles
        blocked_source = (choice.get("blocked_source") is True and source_present
                          and not clear(before[:2], walkable=True))
        if ((source_present and not blocked_source)
                or (before[0]//256, before[1]//256) != (x//256, y//256)
                or not 1 < distance <= MAX_NEAREST_CLEAR_DISTANCE
                or not clear((x, y), walkable=True)
                or any(clear(cell, walkable=True) for cell in tiles
                       if cell != (x, y)
                       and abs(cell[0]-before[0]) + abs(cell[1]-before[1]) <= distance)):
            raise ValueError('chosen placement must adjoin its exact retained source cell')
    if not compiler_qualified and not clear((x, y)) and not same_walkable_surface:
        raise ValueError('chosen placement target is not clear native ground')
    return after, {'origin':'oteryn', 'basis':'CHOSEN_OTERYN_APPROXIMATION',
                   'source_cell':{'x':before[0],'y':before[1],'floor':before[2]},
                   'cell':{'x':x,'y':y,'floor':z}, 'decision':choice['reason'],
                   'map':{'path':path,'sha256':hashlib.sha256(raw).hexdigest()}}
