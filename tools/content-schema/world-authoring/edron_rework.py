#!/usr/bin/env python3
"""The Edron underground rework of the base map (owner decision 1a), a reference-derived rule.

The base map has Edron caves on floors 8-12, and `summer-update-2025.otbm` of `maps.7z`
holds a reworked floor 10. The player-recorded real-Tibia minimap
(github.com/tibiamaps/tibia-map-data, `floor-NN-path.png` and `floor-NN-map.png`, one pixel per
tile, frame `xMin` 31744, `yMin` 30976 of `bounds.json`) decides what is real. Everything is
confined to the box `BOX`. Walkable means: no item of the tile has the client `unpass` flag
(solid rock is stored as `unpass` ground). Tibiamaps walkable is a grey path pixel
(`red == green == blue`); yellow is explored but not walkable, magenta unexplored.

Rule 1 (floor 10): summer tiles where tibiamaps agrees with the summer walkability. The core is
every summer tile that is walkable where tibiamaps is walkable. The included set is the core
plus every summer tile in its 8-neighbourhood, if that tile's walkability also agrees. A
position the base has no tile at is filled. A base tile is replaced only where the base
walkability disagrees with tibiamaps and the summer tile agrees. A base tile that agrees is
kept, so a walkable base tile that tibiamaps shows walkable is never removed.

Rule 2 (floors 9 and 10, after rule 1): a position tibiamaps shows walkable that is walkable
in neither the map so far nor the summer file gets a plain ground tile (the most common
walkable ground of that floor in the box) and every non-walkable 8-neighbour without a tile a
blocking rock tile (the most common blocking ground of that floor in the box). A walkable tile
is never overwritten.

Rule 3 (entrances, floors 9-11): a yellow pixel of the map image is a floor-change marker. It
is connected if the final map has a floor-change item at that position or at the same (x, y)
one floor above or below. Otherwise it is reported as unresolved: no item is invented. The
report also gives the reachability of floor 10 (BFS over walkable tiles and floor-change links).
"""

from __future__ import annotations

import hashlib
import io
import json
from collections import Counter, deque
from dataclasses import dataclass, field
from pathlib import Path

import convert_world_metadata as metadata
from convert_world_metadata import ConvertError

ROOT = metadata.ROOT

BOX = (33274, 31786, 33456, 31884)
BOX_NAME = "edron-underground"
RULE1_FLOOR = 10
RULE2_FLOORS = (9, 10)
MARKER_FLOORS = (9, 10, 11)
REACH_FLOORS = (7, 8, 9, 10, 11, 12)
SURFACE_FLOOR = 7
BASE_FLOORS = REACH_FLOORS
SUMMER_FLOORS = (9, 10)
YELLOW_AUTOMAP = 210
ROPE_SPOTS = {386: "rope", 21965: "rope"}
STEPS = {
    "up_north": (0, -1),
    "up_south": (0, 1),
    "up_east": (1, 0),
    "up_west": (-1, 0),
    "up_south_alt": (0, 2),
    "up_east_alt": (2, 0),
    "rope": (0, 1),
}
TIBIAMAPS_URL = "https://raw.githubusercontent.com/tibiamaps/tibia-map-data/main/data/"
TIBIAMAPS_FILES = [
    ("bounds.json", "2217fc32695d1302b6864d078b86247a93aa0cf673a4d9f7740947221c6d4414"),
    (
        "floor-09-map.png",
        "d7cbccd0571896b719161d2a8ebd6520d3374f41666b57f4531183b5482096e6",
    ),
    (
        "floor-09-path.png",
        "62084fb4a301cbb540d87e37e817b6b7284d6ab1c2f817ca8532429332cfd3d6",
    ),
    (
        "floor-10-map.png",
        "4d61058bdf036535e53c51671c73a22b09bea0536101df09d4ad81707c0de19c",
    ),
    (
        "floor-10-path.png",
        "d9bb193a19f59f320f3db3e41074849d83cc118a19ad0abce6454e233c39f696",
    ),
    (
        "floor-11-map.png",
        "28baa7a5aeeea42a72bce58ce1c9133358dee2f52744ec54564b0db660704c93",
    ),
    (
        "floor-11-path.png",
        "ac564b94d99d2d70721f35c565c81f58d8b2c1740f059930062823cabd595a23",
    ),
]
RULE_TEXT = {
    "rule1": (
        "floor 10: summer tiles whose walkability agrees with tibiamaps, the walkable core plus "
        "its 8-neighbourhood; fill where the base has no tile, replace a base tile only where "
        "the base disagrees with tibiamaps and the summer tile agrees"
    ),
    "rule2": (
        "floors 9 and 10: a tibiamaps-walkable position walkable in neither the map nor the "
        "summer file gets the most common walkable ground, its tile-less non-walkable "
        "8-neighbours the most common blocking ground; a walkable tile is never overwritten"
    ),
    "rule3": (
        "floors 9-11: yellow map pixels are floor-change markers, connected when the final map "
        "has a floor-change item at the position or at the same (x, y) one floor apart, else "
        "listed as unresolved and never invented"
    ),
}
# The pin of the index `source.edron` and the capture summary.
PIN = {
    "box": {"bbox": list(BOX), "name": BOX_NAME},
    "floors": {
        "markers": list(MARKER_FLOORS),
        "rule1": [RULE1_FLOOR],
        "rule2": list(RULE2_FLOORS),
    },
    "member": "summer-update-2025.otbm",
    "rule": RULE_TEXT,
    "tibiamaps": {
        "fetched": "2026-09-29",
        "files": [{"name": n, "sha256": s} for n, s in TIBIAMAPS_FILES],
        "frame": {"x_min": 31744, "y_min": 30976},
        "revision": "main branch head at the fetch date (commit id not retrievable)",
        "url": TIBIAMAPS_URL,
    },
}
MARKER_MAX_AREA = 2
NEIGHBOURS = [(dx, dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]


@dataclass
class TibiaMaps:
    """Tibiamaps data inside the box: `walk[z]` and `markers[z]` are sets of (x, y)."""

    walk: dict[int, set] = field(default_factory=dict)
    markers: dict[int, set] = field(default_factory=dict)


def in_box(x: int, y: int) -> bool:
    return BOX[0] <= x <= BOX[2] and BOX[1] <= y <= BOX[3]


def tibiamaps_blob_key(name: str) -> str:
    return f"tibiamaps/{name}"


def read_tibiamaps_root(root: Path) -> dict[str, bytes]:
    """The pinned tibiamaps files of a directory, each checked against its sha256."""
    blobs = {}
    for name, sha256 in TIBIAMAPS_FILES:
        try:
            data = (root / name).read_bytes()
        except OSError as error:
            raise ConvertError(
                f"tibiamaps file {name} is unreadable: {error}"
            ) from error
        if hashlib.sha256(data).hexdigest() != sha256:
            raise ConvertError(f"tibiamaps/{name}: sha256 differs from the pin")
        blobs[tibiamaps_blob_key(name)] = data
    return blobs


def decode_tibiamaps(blobs: dict[str, bytes]) -> TibiaMaps:
    """The walkable and marker pixels of the box. Reading the PNG needs Pillow."""
    try:
        from PIL import Image
    except ImportError as error:
        raise ConvertError(
            "reading tibiamaps needs Pillow (pip install -r requirements-regenerate.txt)"
        ) from error
    bounds = json.loads(blobs[tibiamaps_blob_key("bounds.json")])
    frame = PIN["tibiamaps"]["frame"]
    if (bounds["xMin"], bounds["yMin"]) != (frame["x_min"], frame["y_min"]):
        raise ConvertError("tibiamaps bounds.json differs from the pinned frame")
    size = (bounds["width"], bounds["height"])
    left, top = BOX[0] - frame["x_min"], BOX[1] - frame["y_min"]
    width, height = BOX[2] - BOX[0] + 1, BOX[3] - BOX[1] + 1
    result = TibiaMaps()
    for z in MARKER_FLOORS:
        for kind in ("path", "map"):
            image = Image.open(
                io.BytesIO(blobs[tibiamaps_blob_key(f"floor-{z:02d}-{kind}.png")])
            )
            if image.size != size:
                raise ConvertError(
                    f"tibiamaps floor {z} {kind} image has the wrong size"
                )
            raw = (
                image.convert("RGB")
                .crop((left, top, left + width, top + height))
                .tobytes()
            )
            pixels = [raw[i : i + 3] for i in range(0, len(raw), 3)]
            at = {
                (BOX[0] + i % width, BOX[1] + i // width): p
                for i, p in enumerate(pixels)
            }
            if kind == "path":
                result.walk[z] = {k for k, p in at.items() if p[0] == p[1] == p[2]}
            else:
                result.markers[z] = {k for k, p in at.items() if p == b"\xff\xff\x00"}
    return result


def appearance_ids(root: Path = ROOT) -> tuple[set[int], set[int]]:
    """`(unpass ids, yellow ids)` of the committed official appearances.

    Yellow is the minimap colour of stairs and holes, but also of some ground: an id whose
    automap colour is yellow (index 210, `#ffff00`) explains a yellow map pixel by itself.
    """
    import client_appearance_reader as appearances

    files = sorted((root / "content/assets/files").glob("appearances-*.dat"))
    if len(files) != 1:
        raise ConvertError("expected exactly one committed appearances file")
    found = appearances.read_appearances(files[0].read_bytes())
    return (
        {i for i, a in found.items() if "unpass" in a.flags},
        {i for i, a in found.items() if a.automap_color == YELLOW_AUTOMAP},
    )


def floor_change_kinds(root: Path = ROOT) -> dict[int, str]:
    """`{item id: kind}` of the committed FloorChange records plus the rope spots."""
    kinds = dict(ROPE_SPOTS)
    for path in sorted((root / "content/world/objects").glob("floor-changes-*.json")):
        for record in json.loads(path.read_text(encoding="utf-8"))["records"]:
            declaration = record["declaration"]
            kinds[declaration["source_item_id"]] = declaration["floor_change"]
    unknown = set(kinds.values()) - set(STEPS) - {"down"}
    if unknown:
        raise ConvertError(f"unknown floor change kinds {sorted(unknown)}")
    return kinds


def walkable(tile, blocking: set[int]) -> bool:
    """A tile is walkable when none of its items (containers excluded) blocks."""
    return not any(item[0] in blocking for item in tile[3] if item[1] == 0)


def refuse_special(position, tile, what: str) -> None:
    """Houses, tile zones and teleport destinations belong to `world.otbm`, never to a rework."""
    _flags, house, zones, items = tile
    if (
        house is not None
        or zones
        or any((item[2] or {}).get("dest", (0, 0, 0)) != (0, 0, 0) for item in items)
    ):
        raise ConvertError(f"{what} tile {position} carries a house, zone or teleport")


def common(counter: Counter, what: str, floor: int) -> int:
    if not counter:
        raise ConvertError(f"no {what} ground on floor {floor} of the box")
    return min(counter, key=lambda i: (-counter[i], i))


def ground_tile(server_id: int):
    return (0, None, (), [(server_id, 0, None)])


@dataclass
class Plan:
    """The tiles the rework adds (`add`) and swaps in (`replace`), and its counts."""

    add: dict = field(default_factory=dict)
    replace: dict = field(default_factory=dict)
    stats: dict = field(default_factory=dict)
    final: dict = field(default_factory=dict)


def plan_rules(base: dict, summer: dict, tm: TibiaMaps, blocking: set[int]) -> Plan:
    """Rules 1 and 2. `base` and `summer` map (x, y, z) to `(flags, house, zones, items)`."""
    out = Plan()
    z = RULE1_FLOOR
    walk = tm.walk.get(z, set())
    floor_summer = {p: t for p, t in summer.items() if p[2] == z and in_box(p[0], p[1])}
    core = {
        p
        for p, t in floor_summer.items()
        if (p[0], p[1]) in walk and walkable(t, blocking)
    }
    ring = set(core)
    for x, y, _z in core:
        ring.update(
            (x + dx, y + dy, z)
            for dx, dy in NEIGHBOURS
            if (x + dx, y + dy, z) in floor_summer
        )
    included = sorted(
        p for p in ring if walkable(floor_summer[p], blocking) == ((p[0], p[1]) in walk)
    )
    counts = Counter()
    for p in included:
        tile, mine = floor_summer[p], base.get(p)
        goes_walkable = walkable(tile, blocking)
        if mine is None:
            refuse_special(p, tile, "rule 1 fill")
            out.add[p] = tile
            counts["filled"] += 1
            counts["filled_walkable" if goes_walkable else "filled_blocked"] += 1
        elif walkable(mine, blocking) != ((p[0], p[1]) in walk):
            refuse_special(p, mine, "rule 1 replaced base")
            refuse_special(p, tile, "rule 1 replacement")
            out.replace[p] = tile
            counts["replaced"] += 1
            counts[
                "replaced_to_walkable" if goes_walkable else "replaced_to_blocked"
            ] += 1
        else:
            counts["kept_base"] += 1
    out.stats["rule1"] = {
        "core": len(core),
        "floor": z,
        "included": len(included),
        "kept_base": counts["kept_base"],
        "filled": counts["filled"],
        "filled_blocked": counts["filled_blocked"],
        "filled_walkable": counts["filled_walkable"],
        "replaced": counts["replaced"],
        "replaced_to_blocked": counts["replaced_to_blocked"],
        "replaced_to_walkable": counts["replaced_to_walkable"],
        "summer_tiles_in_box": len(floor_summer),
    }
    final = {**base, **out.replace, **out.add}
    rule2 = {}
    for z in RULE2_FLOORS:
        walk = tm.walk.get(z, set())
        floor = {p: t for p, t in final.items() if p[2] == z and in_box(p[0], p[1])}
        walk_ground, rock_ground = Counter(), Counter()
        for tile in floor.values():
            if not tile[3]:
                continue
            if walkable(tile, blocking):
                walk_ground[tile[3][0][0]] += 1
            elif tile[3][0][0] in blocking:
                rock_ground[tile[3][0][0]] += 1
        ground = common(walk_ground, "walkable", z)
        rock = common(rock_ground, "blocking", z)
        targets = sorted(
            (x, y, z)
            for x, y in walk
            if not ((x, y, z) in floor and walkable(floor[(x, y, z)], blocking))
            and not (
                summer.get((x, y, z)) is not None
                and walkable(summer[(x, y, z)], blocking)
            )
        )
        added = replaced = rocks = 0
        for p in targets:
            mine = final.get(p)
            if mine is not None:
                refuse_special(p, mine, "rule 2 replaced")
                out.replace[p] = ground_tile(ground)
                replaced += 1
            else:
                out.add[p] = ground_tile(ground)
                added += 1
            final[p] = ground_tile(ground)
        for x, y, _z in targets:
            for dx, dy in NEIGHBOURS:
                n = (x + dx, y + dy, z)
                if in_box(n[0], n[1]) and (n[0], n[1]) not in walk and n not in final:
                    out.add[n] = final[n] = ground_tile(rock)
                    rocks += 1
        rule2[str(z)] = {
            "blocking_ground": rock,
            "rock_added": rocks,
            "targets": len(targets),
            "walkable_added": added,
            "walkable_ground": ground,
            "walkable_replaced": replaced,
        }
    out.stats["rule2"] = rule2
    out.final = final
    return out


def floor_changes(final: dict, kinds: dict[int, str]) -> dict:
    """`{(x, y, z): kind}` of the floor-change items of the final tiles."""
    found = {}
    for p, tile in final.items():
        for item in tile[3]:
            if item[1] == 0 and item[0] in kinds:
                found[p] = kinds[item[0]]
    return found


def destination(p, kind):
    x, y, z = p
    if kind == "down":
        return (x, y, z + 1)
    dx, dy = STEPS[kind]
    return (x + dx, y + dy, z - 1)


def area_sizes(pixels: set) -> dict:
    """`{pixel: size of its 8-connected component}` of a set of (x, y)."""
    sizes: dict = {}
    for start in sorted(pixels):
        if start in sizes:
            continue
        seen, queue = {start}, [start]
        for x, y in queue:
            for dx, dy in NEIGHBOURS:
                n = (x + dx, y + dy)
                if n in pixels and n not in seen:
                    seen.add(n)
                    queue.append(n)
        sizes.update(dict.fromkeys(seen, len(seen)))
    return sizes


def entrances(
    final: dict, tm: TibiaMaps, kinds: dict[int, str], yellow: set[int]
) -> dict:
    """Rule 3: connected markers and the unresolved ones (never invented).

    A yellow pixel on a tile that carries a yellow-automap item other than a floor change (for
    example yellow ground) is explained by that item and is not a marker. A pixel in an
    8-connected yellow area of more than `MARKER_MAX_AREA` pixels is ground colour (an icon is
    one tile, two at most), so it is not a marker either.
    """
    changes = floor_changes(final, kinds)
    by_floor: dict[int, set] = {}
    for x, y, z in changes:
        by_floor.setdefault(z, set()).add((x, y))
    markers, connected, unresolved, explained, areas = {}, {}, [], {}, {}
    for z in MARKER_FLOORS:
        pixels = tm.markers.get(z, set())
        area = area_sizes(pixels)
        found, hidden = [], 0
        for p in sorted(pixels):
            if any(
                i[1] == 0 and i[0] in yellow and i[0] not in kinds
                for i in final.get((p[0], p[1], z), (0, 0, 0, ()))[3]
            ):
                hidden += 1
            elif area[p] <= MARKER_MAX_AREA:
                found.append(p)
        explained[str(z)] = hidden
        areas[str(z)] = len(pixels) - hidden - len(found)
        markers[str(z)] = len(found)
        ok = 0
        for x, y in found:
            if any((x, y) in by_floor.get(f, ()) for f in (z, z - 1, z + 1)):
                ok += 1
            else:
                unresolved.append([x, y, z])
        connected[str(z)] = ok
    return {
        "connected": connected,
        "explained_by_yellow_items": explained,
        "yellow_area_pixels": areas,
        "floor_changes_in_box": {
            str(z): sum(1 for p in changes if p[2] == z) for z in REACH_FLOORS
        },
        "markers": markers,
        "unresolved_entrances": sorted(unresolved, key=lambda p: (p[2], p[1], p[0])),
    }


def reachability(
    final: dict, new: set, blocking: set[int], kinds: dict[int, str]
) -> dict:
    """Floor 10 reachability: BFS over walkable tiles (8 directions) and floor-change links.

    `new` are the positions the rework changed. Seeds: the walkable floor 7 tiles of the box
    (surface), and separately every walkable tile of floors 8, 9, 11 and 12.
    """
    tiles = {
        p
        for p, t in final.items()
        if p[2] in REACH_FLOORS and in_box(p[0], p[1]) and walkable(t, blocking)
    }
    links = {p: destination(p, k) for p, k in floor_changes(final, kinds).items()}

    def reach(seeds):
        seen = set(seeds)
        queue = deque(sorted(seen))
        while queue:
            x, y, z = queue.popleft()
            near = [(x + dx, y + dy, z) for dx, dy in NEIGHBOURS]
            if (x, y, z) in links:
                near.append(links[(x, y, z)])
            for n in near:
                if n in tiles and n not in seen:
                    seen.add(n)
                    queue.append(n)
        return seen

    surface = reach(p for p in tiles if p[2] == SURFACE_FLOOR)
    others = reach(p for p in tiles if p[2] not in (SURFACE_FLOOR, RULE1_FLOOR))
    floor10 = sorted(p for p in tiles if p[2] == RULE1_FLOOR)
    fresh = sorted(p for p in floor10 if p in new)
    components, seen = [], set()
    for start in floor10:
        if start in seen:
            continue
        seen.add(start)
        queue, size, hit = deque([start]), 0, False
        while queue:
            x, y, z = queue.popleft()
            size += 1
            hit = hit or (x, y, z) in surface
            for dx, dy in NEIGHBOURS:
                n = (x + dx, y + dy, z)
                if n in tiles and n not in seen:
                    seen.add(n)
                    queue.append(n)
        components.append((size, hit))
    largest = max(components, default=(0, False))
    return {
        "floor10_components": len(components),
        "floor10_components_reached_from_surface": sum(1 for _s, h in components if h),
        "floor10_largest_component": largest[0],
        "floor10_new_walkable": len(fresh),
        "floor10_new_walkable_reached_from_other_floors": sum(
            p in others for p in fresh
        ),
        "floor10_new_walkable_reached_from_surface": sum(p in surface for p in fresh),
        "floor10_walkable": len(floor10),
        "floor10_walkable_reached_from_other_floors": sum(p in others for p in floor10),
        "floor10_walkable_reached_from_surface": sum(p in surface for p in floor10),
        "seed": "walkable floor 7 tiles of the box (surface); other floors 8, 9, 11, 12",
        "walkable_by_floor": {
            str(z): sum(1 for p in tiles if p[2] == z) for z in REACH_FLOORS
        },
    }


def summarize(plan: Plan, tm: TibiaMaps, blocking, kinds, yellow) -> dict:
    """The `edron` record of the capture summary (the caller adds the item counts)."""
    rule1, rule2 = plan.stats["rule1"], plan.stats["rule2"]
    changed = set(plan.add) | set(plan.replace)
    added = Counter(p[2] for p in plan.add)
    replaced = Counter(p[2] for p in plan.replace)
    return {
        "applied": True,
        "entrances": entrances(plan.final, tm, kinds, yellow),
        "reachability": reachability(plan.final, changed, blocking, kinds),
        "rule1": rule1,
        "rule2": rule2,
        "tiles_added": len(plan.add),
        "tiles_added_by_floor": {str(z): n for z, n in sorted(added.items())},
        "tiles_replaced": len(plan.replace),
        "tiles_replaced_by_floor": {str(z): n for z, n in sorted(replaced.items())},
        "tibiamaps_walkable_by_floor": {
            str(z): len(tm.walk.get(z, ())) for z in MARKER_FLOORS
        },
    }


def read_box(raw: bytes, floors) -> dict:
    """The tiles of the box on `floors` of an OTBM file: `{(x, y, z): tile}`."""
    import otbm_reader

    found: dict = {}

    def on_tile(x, y, z, flags, house, zones, items) -> None:
        if z in floors and in_box(x, y):
            found[(x, y, z)] = (flags or 0, house, tuple(zones or ()), list(items))

    otbm_reader.read_tiles(raw, on_tile)
    return found
