#!/usr/bin/env python3
"""Minimap-based draft of map areas that no source has (owner decision 2a), a rough draft.

The base map has no tiles for some real areas (first one: the Temple of Light island,
`AREAS`) and no OTBM file of the pinned sources holds them. The player-recorded real-Tibia
minimap (github.com/tibiamaps/tibia-map-data, `floor-NN-map.png` and `floor-NN-path.png`,
one pixel per tile, frame of `bounds.json`, pinned by sha256) gives the shape and the
walkability. The draft gives each drafted tile a generic ground: correct shape and
walkability, no borders, decorations, doors or furniture.

Colour mapping (learned, never written by hand): over the whole base map on `FLOORS`, outside
the drafted areas, every base tile is paired with the tibiamaps map colour at its position. The path image gives the
class: a grey pixel (`red == green == blue`) is walkable, any other explored pixel is
blocked. For each (colour, class) the table holds the most frequent ground id, for a
blocked class also its most frequent single top item (the last item above the ground, or none
when that is the most frequent outcome). A (colour, class) with fewer than `MIN_SAMPLES`
samples is unmapped: its pixels are skipped and counted. Yellow (`#ffff00`) is the
floor-change marker colour and never enters the table.

Draft rule, per area, floor of `FLOORS` and position of the bbox: the map image is coloured
(not black, explored) and the base has no tile there or a plain water tile (water ground
only, no house or zone). On floor 7 the official 15.30 client minimap must also show land
there, unless the mapped ground is water. A base tile that is not plain water is never
replaced. A yellow pixel is not turned into an item; it is listed in `unresolved_entrances`.
"""

from __future__ import annotations

import io
import json
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

import edron_rework as edron
from convert_world_metadata import ConvertError

NAME = "minimap-draft"
FLOORS = (6, 7)
OFFICIAL_FLOOR = 7
MIN_SAMPLES = 50
BLACK = b"\x00\x00\x00"
MARKER = b"\xff\xff\x00"
AREAS = [
    {
        "bbox": [31912, 31979, 32027, 32099],
        "floors": list(FLOORS),
        "name": "temple-of-light",
    }
]
DRAFT_FILES = [
    (
        "floor-06-map.png",
        "8d88bbcd64e75b71a13e70b9f86ddf7dcb0abb53af327e0ea37ca1edb34a6349",
    ),
    (
        "floor-06-path.png",
        "112fc0f2a1895708bc8065270059c81d4d6a81a63af7cdf1fa3e7d1db4d3011c",
    ),
    (
        "floor-07-map.png",
        "c21f517522e02e3c31d24f4008044b56662d1dd6fe1aeee225feaebcd88692ac",
    ),
    (
        "floor-07-path.png",
        "6f100ecde039e445910db31230a990af2b64fd12916bb83814172e7a64267db2",
    ),
]
BOUNDS = next(row for row in edron.TIBIAMAPS_FILES if row[0] == "bounds.json")
FILES = [BOUNDS, *DRAFT_FILES]
RULE_TEXT = {
    "draft": (
        "rough draft: a tile is drafted where the tibiamaps map image is coloured and the base "
        "has no tile or a plain water tile; correct shape and walkability, generic ground, no "
        "borders, decorations, doors or furniture; a base tile that is not plain water is "
        "never replaced"
    ),
    "floor_change_markers": (
        "yellow map pixels are floor-change markers, listed as unresolved entrances and never "
        "turned into items"
    ),
    "mapping": (
        "per (map colour, path class) over the whole base map on floors 6 and 7 outside the "
        "drafted areas: the most frequent ground id, for a blocked class also the most frequent "
        "single top item; fewer than min_samples samples is unmapped and skipped"
    ),
    "official_minimap": (
        "floor 7 also needs the official 15.30 client minimap to show land, unless the mapped "
        "ground is water"
    ),
}
# The pin of the index `source.minimap_draft` and the capture summary.
PIN = {
    "areas": AREAS,
    "min_samples": MIN_SAMPLES,
    "official_minimap_floor": OFFICIAL_FLOOR,
    "rule": RULE_TEXT,
    "tibiamaps": {
        **edron.PIN["tibiamaps"],
        "files": [{"name": n, "sha256": s} for n, s in FILES],
    },
}
STAT_KEYS = (
    "added",
    "blocked",
    "explored",
    "kept_base",
    "markers",
    "no_official_land",
    "replaced",
    "unmapped",
    "walkable",
    "water_over_water",
)


def blob_key(name: str) -> str:
    return edron.tibiamaps_blob_key(name)


def has_files(blobs: dict[str, bytes]) -> bool:
    return all(blob_key(name) in blobs for name, _sha in FILES)


def read_files(root: Path) -> dict[str, bytes]:
    """The draft's tibiamaps files of a directory, each checked against its sha256."""
    import hashlib

    blobs = {}
    for name, sha256 in FILES:
        try:
            data = (root / name).read_bytes()
        except OSError as error:
            raise ConvertError(
                f"tibiamaps file {name} is unreadable: {error}"
            ) from error
        if hashlib.sha256(data).hexdigest() != sha256:
            raise ConvertError(f"tibiamaps/{name}: sha256 differs from the pin")
        blobs[blob_key(name)] = data
    return blobs


@dataclass
class Images:
    """Decoded map and path pixels: `maps[z]` and `paths[z]` are RGB bytes of the whole frame."""

    x_min: int
    y_min: int
    width: int
    height: int
    maps: dict[int, bytes] = field(default_factory=dict)
    paths: dict[int, bytes] = field(default_factory=dict)

    def at(self, z: int, x: int, y: int):
        """`(map rgb, path rgb)` at a position, or None outside the frame."""
        dx, dy = x - self.x_min, y - self.y_min
        if not (0 <= dx < self.width and 0 <= dy < self.height):
            return None
        i = (dy * self.width + dx) * 3
        return self.maps[z][i : i + 3], self.paths[z][i : i + 3]


def decode_images(blobs: dict[str, bytes]) -> Images:
    """The draft floors of the pinned frame. Reading the PNG needs Pillow."""
    try:
        from PIL import Image
    except ImportError as error:
        raise ConvertError(
            "reading tibiamaps needs Pillow (pip install -r requirements-regenerate.txt)"
        ) from error
    bounds = json.loads(blobs[blob_key("bounds.json")])
    frame = edron.PIN["tibiamaps"]["frame"]
    if (bounds["xMin"], bounds["yMin"]) != (frame["x_min"], frame["y_min"]):
        raise ConvertError("tibiamaps bounds.json differs from the pinned frame")
    size = (bounds["width"], bounds["height"])
    images = Images(frame["x_min"], frame["y_min"], *size)
    for z in FLOORS:
        for kind, store in (("map", images.maps), ("path", images.paths)):
            image = Image.open(io.BytesIO(blobs[blob_key(f"floor-{z:02d}-{kind}.png")]))
            if image.size != size:
                raise ConvertError(
                    f"tibiamaps floor {z} {kind} image has the wrong size"
                )
            store[z] = image.convert("RGB").tobytes()
    return images


def path_class(path: bytes) -> str:
    return "walkable" if path[0] == path[1] == path[2] else "blocked"


def in_area(area: dict, x: int, y: int) -> bool:
    b = area["bbox"]
    return b[0] <= x <= b[2] and b[1] <= y <= b[3]


class Scanner:
    """One pass over the base tiles: the colour samples of `FLOORS` and the tiles of the areas."""

    def __init__(self, images: Images, areas=AREAS):
        self.images, self.areas = images, areas
        self.grounds: dict[tuple, Counter] = {}
        self.tops: dict[tuple, Counter] = {}
        self.tiles: dict[tuple, tuple] = {}

    def __call__(self, x, y, z, flags, house, zones, items) -> None:
        if z not in FLOORS:
            return
        if any(z in a["floors"] and in_area(a, x, y) for a in self.areas):
            # the base is unreliable where it is drafted (water under land), never a sample
            self.tiles[(x, y, z)] = (flags or 0, house, tuple(zones or ()), list(items))
            return
        pixel = self.images.at(z, x, y)
        if pixel is None or not items or pixel[0] in (BLACK, MARKER):
            return
        key = (pixel[0], path_class(pixel[1]))
        ground = items[0][0]
        self.grounds.setdefault(key, Counter())[ground] += 1
        top = items[-1][0] if len(items) > 1 and items[-1][1] == 0 else None
        self.tops.setdefault((*key, ground), Counter())[top] += 1


def best(counter: Counter):
    return min(counter, key=lambda i: (-counter[i], i is not None, i or 0))


def learn(scan: Scanner) -> dict[tuple, dict]:
    """`{(rgb, class): row}` of every sampled (colour, class); `row["mapped"]` needs samples."""
    table = {}
    for key, counter in scan.grounds.items():
        ground = best(counter)
        samples = sum(counter.values())
        item = best(scan.tops[(*key, ground)]) if key[1] == "blocked" else None
        table[key] = {
            "ground": ground,
            "item": item,
            "mapped": samples >= MIN_SAMPLES,
            "samples": samples,
        }
    return table


def hexcolour(rgb: bytes) -> str:
    return "#" + rgb.hex()


def ground_tile(ground: int, item: int | None, flags: int = 0):
    return (flags, None, (), [(ground, 0, None)] + ([(item, 0, None)] if item else []))


@dataclass
class Plan:
    """The tiles the draft adds (`add`) and swaps in (`replace`), and its records."""

    add: dict = field(default_factory=dict)
    replace: dict = field(default_factory=dict)
    replaced_items_added: int = 0
    replaced_items_removed: int = 0
    table: dict = field(default_factory=dict)
    stats: dict = field(default_factory=dict)
    entrances: list = field(default_factory=list)
    on_official_land: Counter = field(default_factory=Counter)


def plain_water(tile, water: set[int]) -> bool:
    _flags, house, zones, items = tile
    return house is None and not zones and len(items) == 1 and items[0][0] in water


def plan_draft(scan: Scanner, images: Images, land, water: set[int]) -> Plan:
    """The draft of every area: see the module docstring."""
    plan = Plan(table=learn(scan))
    unseen: dict[tuple, int] = {}
    for area in scan.areas:
        b = area["bbox"]
        rows = plan.stats.setdefault(area["name"], {})
        for z in area["floors"]:
            stat = rows.setdefault(str(z), dict.fromkeys(STAT_KEYS, 0))
            for y in range(b[1], b[3] + 1):
                for x in range(b[0], b[2] + 1):
                    pixel = images.at(z, x, y)
                    if pixel is None or pixel[0] == BLACK:
                        continue
                    stat["explored"] += 1
                    base = scan.tiles.get((x, y, z))
                    if base is not None and not plain_water(base, water):
                        stat["kept_base"] += 1
                        continue
                    if pixel[0] == MARKER:
                        stat["markers"] += 1
                        plan.entrances.append([x, y, z])
                        continue
                    klass = path_class(pixel[1])
                    key = (pixel[0], klass)
                    row = plan.table.get(key)
                    if row is None or not row["mapped"]:
                        stat["unmapped"] += 1
                        unseen.setdefault(key, 0)
                        continue
                    is_water = row["ground"] in water
                    if z == OFFICIAL_FLOOR and not is_water and not land(x, y, z):
                        stat["no_official_land"] += 1
                        continue
                    if is_water and base is not None:
                        stat["water_over_water"] += 1
                        continue
                    stat[klass] += 1
                    if not is_water:
                        plan.on_official_land[z] += bool(land(x, y, z))
                    if base is None:
                        plan.add[(x, y, z)] = ground_tile(row["ground"], row["item"])
                        stat["added"] += 1
                    else:
                        tile = ground_tile(row["ground"], row["item"], base[0])
                        plan.replace[(x, y, z)] = tile
                        plan.replaced_items_removed += len(base[3])
                        plan.replaced_items_added += len(tile[3])
                        stat["replaced"] += 1
    for key in unseen:
        plan.table.setdefault(
            key, {"ground": None, "item": None, "mapped": False, "samples": 0}
        )
    plan.entrances.sort(key=lambda p: (p[2], p[1], p[0]))
    return plan


def unapplied() -> dict:
    return {
        "applied": False,
        "items_added": 0,
        "replaced_items_added": 0,
        "replaced_items_removed": 0,
        "tiles_added": 0,
        "tiles_replaced": 0,
    }


def summarize(plan: Plan, items_added: int) -> dict:
    """The `draft` record of the capture summary (the caller adds the item count)."""
    added = Counter(p[2] for p in plan.add)
    replaced = Counter(p[2] for p in plan.replace)
    rows = [
        {
            "class": key[1],
            "colour": hexcolour(key[0]),
            "ground": row["ground"],
            "item": row["item"],
            "mapped": row["mapped"],
            "samples": row["samples"],
        }
        for key, row in sorted(plan.table.items(), key=lambda kv: (kv[0][0], kv[0][1]))
    ]
    return {
        "applied": True,
        "areas": [
            {
                "bbox": area["bbox"],
                "floors": plan.stats[area["name"]],
                "name": area["name"],
            }
            for area in AREAS
        ],
        "drafted_land_on_official_land": {
            str(z): n for z, n in sorted(plan.on_official_land.items())
        },
        "items_added": items_added,
        "mapping": {
            "mapped": sum(r["mapped"] for r in rows),
            "min_samples": MIN_SAMPLES,
            "rows": rows,
            "unmapped": sum(not r["mapped"] for r in rows),
        },
        "replaced_items_added": plan.replaced_items_added,
        "replaced_items_removed": plan.replaced_items_removed,
        "tiles_added": len(plan.add),
        "tiles_added_by_floor": {str(z): n for z, n in sorted(added.items())},
        "tiles_replaced": len(plan.replace),
        "tiles_replaced_by_floor": {str(z): n for z, n in sorted(replaced.items())},
        "unresolved_entrances": plan.entrances,
    }
