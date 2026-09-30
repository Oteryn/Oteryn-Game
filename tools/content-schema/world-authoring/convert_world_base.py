#!/usr/bin/env python3
"""Convert the pinned CrystalServer world map into the WorldPlacement.Base region family.

Writes content/world/placements/ (region files and index.json) plus the committed capture
summary. The source is OTS_HYPOTHESIS_ONLY migration evidence. Region files store, per
item, an index into the ``palette`` of index.json. The palette holds one entry per distinct
map server item id and is append-only: a fresh build orders entries by ascending id, and a
build over a committed palette keeps every entry at its index, appends new ids at the end
in ascending id order and flags an entry the map no longer uses ``"retired": true``. An id
bound in the item bindings resolves to its binding target key, an appearance-only id that the
committed Terrain family covers to its `oteryn:terrain.a<id>` key. Any other id gets a
provisional donor key. The conversion fails closed on an item or tile attribute it does
not carry.

After `world.otbm`, fragment maps of `maps.7z` (`FILL`, currently `blue_valley.otbm`) fill
tiles the base map lacks: a tile is added only where the base map has no tile at its position,
nothing existing is overwritten or merged. Reading the archive needs `py7zr`
(`requirements-regenerate.txt`); the fill is pinned by the sha256 of the archive and of the
member, and its counts are recorded in the capture summary.

After the fills, the Edron underground box is reworked from the summer file and the
real-Tibia minimap (`edron_rework.py`, pinned tibiamaps files read from `--tibiamaps-root`).
Then areas that no source has are drafted from the minimap (`minimap_draft.py`, rough draft:
first the Temple of Light) where the base has no tile or plain water.

    python convert_world_base.py --crystal-root /path/to/crystalserver \
        --tibiamaps-root /path/to/tibiamaps-data [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import struct
import sys
import tempfile
from collections import Counter
from itertools import pairwise
from pathlib import Path
from xml.etree import ElementTree

import convert_world_metadata as metadata
import edron_rework as edron
import minimap_draft as draft
import otbm_reader
import world_region_codec as codec
import zstandard
from client_map_reader import unframe
from convert_world_metadata import ConvertError, canonical

ROOT = metadata.ROOT
HERE = metadata.HERE
SUMMARY = HERE / "samples/world-base-capture-v1.json"
ITEM_BINDINGS = metadata.ITEM_BINDINGS
OTBM = "data-global/world/world.otbm"
ITEMS_XML = "data/items/items.xml"
DIRECTORY = "content/world/placements"
GENERATOR = "tools/content-schema/world-authoring/convert_world_base.py"
PINNED_TOTALS = {"items": 24925845, "tiles": 19325129}
ITEM_NAMESPACE = "ots/item_server_id"
DONOR_PREFIX = f"donor:crystalserver@{metadata.SOURCE['revision'][:8]}:item/"
TERRAIN_DIRECTORY = "content/world/terrain"
TERRAIN_NAMESPACE = "tibia-client/appearance-id"
TERRAIN_KEY_PREFIX = "oteryn:terrain.a"

MAPS_ARCHIVE = "data-global/world/maps.7z"
# Fragment maps inside `maps.7z` that fill tiles the base map lacks. Absolute Tibia
# coordinates, the frame of `world.otbm`, so no offset. A tile is added only where the base
# map has no tile at that position: nothing existing is overwritten or merged.
FILL_RULE = "a tile is added only where the base map has no tile at that position"
REPLACE_RULE = (
    "a base tile is replaced only where its ground is water, the fragment tile's ground is "
    "land and the official minimap shows land"
)
GROUND_CLASSES = HERE / "island-ground-classes.json"
FILL = [
    {
        "archive": {
            "path": MAPS_ARCHIVE,
            "sha256": "c770e2399f60203f87b68b11b7393cd625f19e069177787dc991b5ee8f53ab77",
        },
        "member": {
            "name": "blue_valley.otbm",
            "sha256": "9f0bd617651de2219b5f4227d8e34166fd143eeb519a3bdde6658346ba68d63d",
        },
        # The one place a base tile is replaced, by `replacement_candidates`: on these
        # floors a base tile whose ground is water becomes the fragment tile where that
        # tile's ground is land and the official 15.30 minimap shows land.
        "replace": {
            "base_ground": "water",
            "floors": [7, 7],
            "fragment_ground": "land",
            "official": "official-minimap-land",
        },
    },
    {
        "archive": {
            "path": MAPS_ARCHIVE,
            "sha256": "c770e2399f60203f87b68b11b7393cd625f19e069177787dc991b5ee8f53ab77",
        },
        "member": {
            "name": "summer-update-2025.otbm",
            "sha256": "d4b4baeedbccdae35011e197a78cab557ec6de774aeb52a1e47cec0646e99a26",
        },
        # Selection rule of a partial fill, computed by `select_tiles`: the fragment tiles
        # the base (after the earlier fills) lacks, on floors 8-15 whole 4-connected
        # components except those touching an exclusion box, on floors 0-7 single tiles
        # that the official 15.30 minimap shows as land.
        "select": {
            "connectivity": 4,
            "exclude": [
                {
                    "bbox": [33274, 31786, 33456, 31884],
                    "floors": [8, 12],
                    "name": "edron-underground",
                }
            ],
            "surface": {"floors": [0, 7], "test": "official-minimap-land"},
            "underground": {"floors": [8, 15], "unit": "component"},
        },
    },
]

SOURCE = {
    **metadata.SOURCE,
    "files": [
        *(row for row in metadata.SOURCE["files"] if row["path"] == OTBM),
        {
            "path": ITEMS_XML,
            "sha256": "13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece",
        },
    ],
    "fill": FILL,
}
# OTBM item attribute -> carried name.
CARRIED = {
    4: "action",
    5: "unique",
    6: "text",
    7: "description",
    8: "teleport",
    10: "depot",
    14: "door",
    15: "count",
    22: "charges",
}
DEST_KEY = "dest"
UNSET_DEST = (0, 0, 0)
_INDEX = list(range(codec.SECTOR_SIZE * codec.SECTOR_SIZE))


def donor_key(server_id: int) -> str:
    """The provisional key of a server id that has no item binding."""
    return f"{DONOR_PREFIX}{server_id}"


def bound_keys(bindings: bytes) -> dict[int, str]:
    """Return ``{server id: target key}`` for every ``ots/item_server_id`` binding."""
    keys: dict[int, str] = {}
    for row in json.loads(bindings)["bindings"]:
        if row["identity_namespace"] != ITEM_NAMESPACE:
            continue
        server_id, key = int(row["external_id"]), row["target"]["key"]
        if keys.setdefault(server_id, key) != key:
            raise ConvertError(f"server id {server_id} is bound to two item keys")
    return keys


def terrain_keys(root: Path = ROOT) -> dict[int, str]:
    """``{appearance id: Terrain key}`` of the committed Terrain family (empty until populated)."""
    path = root / TERRAIN_DIRECTORY / "index.json"
    index = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
    keys: dict[int, str] = {}
    for shard in index.get("shards", []):
        for record in json.loads((root / shard).read_text(encoding="utf-8"))["records"]:
            for row in record["source_bindings"]:
                if row["identity_namespace"] == TERRAIN_NAMESPACE:
                    keys[int(row["external_id"])] = row["target"]["key"]
    return keys


def palette_entry(
    server_id: int, bound: dict[int, str], terrain: dict[int, str] | None = None
) -> dict:
    """Item binding key, else Terrain key (appearance-only ids), else the provisional donor key."""
    item_key = bound.get(server_id)
    terrain_key = (terrain or {}).get(server_id)
    if item_key and terrain_key:
        raise ConvertError(f"server id {server_id} is both an Item and a Terrain id")
    key = item_key or terrain_key
    return {
        "key": key or donor_key(server_id),
        "provisional": key is None,
        "source_item_id": server_id,
    }


def build_palette(
    bound: dict[int, str],
    occurrences: Counter,
    previous: list[dict] | None = None,
    terrain: dict[int, str] | None = None,
) -> list[dict]:
    """The append-only palette.

    Without a committed palette: one entry per distinct map server id, ascending by id.
    With one: every committed entry keeps its index (its key is refreshed from the current
    bindings), an id the map no longer uses is kept and flagged ``retired``, and new ids are
    appended in ascending id order, so region files never need renumbering.
    """
    palette = []
    seen: set[int] = set()
    for row in previous or ():
        server_id = row["source_item_id"]
        if server_id in seen:
            raise ConvertError(f"committed palette lists server id {server_id} twice")
        seen.add(server_id)
        entry = palette_entry(server_id, bound, terrain)
        if server_id not in occurrences:
            entry["retired"] = True
        palette.append(entry)
    palette.extend(
        palette_entry(server_id, bound, terrain)
        for server_id in sorted(occurrences)
        if server_id not in seen
    )
    keys = Counter(row["key"] for row in palette)
    duplicated = sorted(key for key, count in keys.items() if count > 1)
    if duplicated:
        raise ConvertError(
            f"palette keys shared by several server ids: {duplicated[:5]}"
        )
    return palette


def committed_palette(root: Path = ROOT) -> list[dict] | None:
    """The palette of the committed index, or None while the family is unpopulated."""
    path = root / DIRECTORY / "index.json"
    if not path.is_file():
        return None
    palette = json.loads(path.read_text(encoding="utf-8")).get("palette")
    if palette is None:
        return None
    if not all(
        isinstance(row, dict) and isinstance(row.get("source_item_id"), int)
        for row in palette
    ):
        raise ConvertError(f"{DIRECTORY}/index.json: committed palette is malformed")
    return palette


def items_xml_ids(xml: bytes) -> set[int]:
    """Every item id items.xml declares, ranges included."""
    ids: set[int] = set()
    for element in ElementTree.fromstring(xml).iter("item"):
        if "id" in element.attrib:
            ids.add(int(element.attrib["id"]))
        else:
            ids.update(
                range(int(element.attrib["fromid"]), int(element.attrib["toid"]) + 1)
            )
    return ids


class Collector:
    """Buffers encoded tiles per sector; identical tile bodies share one bytes object.

    Bodies are encoded with the source server id as the item number and remapped to
    palette indexes once the palette is known (``remap``).
    """

    def __init__(self):
        self.sectors: dict[tuple[int, int, int], tuple[list, list]] = {}
        self.bodies: dict[bytes, bytes] = {}
        self.final: dict[bytes, bytes] = {}
        self.sector_items: Counter = Counter()
        self.tiles = self.items = 0
        self.occurrences: Counter = Counter()
        self.unsupported: Counter = Counter()
        self.attributes: Counter = Counter()
        self.house_tiles = self.zone_tiles = 0

    def __call__(self, x, y, z, flags, house, zones, items) -> None:
        converted = []
        for server_id, depth, attrs in items:
            self.occurrences[server_id] += 1
            named = None
            if attrs:
                named = {}
                for key, value in attrs.items():
                    name = CARRIED.get(8 if key == DEST_KEY else key)
                    if name is None:
                        self.unsupported[key] += 1
                    else:
                        named[name] = value
                        self.attributes[name] += 1
            converted.append((server_id, depth, named))
        if house == 0:
            raise ConvertError(f"house tile ({x}, {y}, {z}) has house id 0")
        body = codec.encode_tile(flags or 0, house or 0, zones or (), converted)
        body = self.bodies.setdefault(body, body)
        key = (z, x // codec.SECTOR_SIZE, y // codec.SECTOR_SIZE)
        sector = self.sectors.get(key)
        if sector is None:
            sector = self.sectors[key] = ([], [])
        sector[0].append(
            _INDEX[(y % codec.SECTOR_SIZE) * codec.SECTOR_SIZE + x % codec.SECTOR_SIZE]
        )
        sector[1].append(body)
        self.sector_items[key] += len(converted)
        self.tiles += 1
        self.items += len(items)
        self.house_tiles += house is not None
        self.zone_tiles += bool(zones)

    def check(self) -> None:
        if self.unsupported:
            raise ConvertError(
                f"unsupported OTBM item attributes (attr: occurrences): "
                f"{dict(sorted(self.unsupported.items()))}"
            )

    def remap(self, index_of: dict[int, int]) -> None:
        """Rewrite every distinct tile body from server ids to palette indexes."""
        for body in self.bodies:
            _x, _y, flags, house, zones, items = codec.decode_sector(
                b"\x01\x00" + body, 0, 0
            )[0]
            self.final[body] = codec.encode_tile(
                flags,
                house,
                zones,
                [(index_of[sid], depth, attrs) for sid, depth, attrs in items],
            )

    def sector_payload(self, key: tuple[int, int, int]) -> bytes:
        indexes, raw = self.sectors.pop(key)
        bodies = [self.final[body] for body in raw]
        order = range(len(indexes))
        if indexes != sorted(indexes):
            order = sorted(order, key=indexes.__getitem__)
        entries = [(indexes[i], bodies[i]) for i in order]
        for a, b in pairwise(entries):
            if a[0] == b[0]:
                raise ConvertError(
                    f"duplicate tile in sector {key} at local index {a[0]}"
                )
        return codec.encode_sector(entries)


def fill_key(row: dict) -> str:
    """The `blobs` key of a fill member: archive path, `!`, member name."""
    return f"{row['archive']['path']}!{row['member']['name']}"


def sector_slot(x: int, y: int) -> int:
    return _INDEX[(y % codec.SECTOR_SIZE) * codec.SECTOR_SIZE + x % codec.SECTOR_SIZE]


def minimap_land(root: Path = ROOT):
    """A function `(x, y, z) -> bool`: the official 15.30 minimap shows land there.

    Land is a pixel that is neither black (no data) nor water (`#336699`). The minimap
    exists for floors 0-7 only; a position outside its files is not land. Files load
    lazily from the committed client assets.
    """
    # A file name holds the top-left tile of a 512 x 512 block divided by 32.
    files: dict[tuple[int, int, int], Path] = {}
    for path in (root / "content/assets/files").glob("minimap-32-*.bmp.lzma"):
        a, b, z = (int(v) for v in path.name.split("-")[2:5])
        files[(a * 32, b * 32, z)] = path
    if len({(x % 512, y % 512) for x, y, _z in files}) > 1:
        raise ConvertError("minimap blocks are not on one 512-tile grid")
    grid_x, grid_y = next(iter(files))[:2] if files else (0, 0)
    grid_x, grid_y = grid_x % 512, grid_y % 512
    cache: dict[tuple[int, int, int], tuple[bytes, int, int, int] | None] = {}

    def load(key):
        if key not in cache:
            path = files.get(key)
            if path is None:
                cache[key] = None
            else:
                bmp = unframe(path.read_bytes())
                offset = struct.unpack("<I", bmp[10:14])[0]
                width, height = struct.unpack("<ii", bmp[18:26])
                if height <= 0 or bmp[28] != 32:
                    raise ConvertError(f"{path.name}: unexpected minimap bitmap")
                cache[key] = (bmp, offset, width, height)
        return cache[key]

    def land(x: int, y: int, z: int) -> bool:
        block_x, block_y = x - (x - grid_x) % 512, y - (y - grid_y) % 512
        tile = load((block_x, block_y, z))
        if tile is None:
            return False
        bmp, offset, width, height = tile
        if x - block_x >= width or y - block_y >= height:
            return False
        row = height - 1 - (y - block_y)
        at = offset + (row * width + (x - block_x)) * 4
        blue, green, red = bmp[at], bmp[at + 1], bmp[at + 2]
        return (red, green, blue) not in ((0, 0, 0), (0x33, 0x66, 0x99))

    return land


def select_tiles(pending: dict, select: dict, land) -> tuple[set, dict]:
    """The positions of `pending` (`{(x, y, z): tile}`) that a partial fill takes.

    Surface floors keep a tile if `land` says so. Underground floors keep every
    4-connected component (per floor) that has no tile inside an exclusion box.
    """
    low, high = select["underground"]["floors"]
    surface_low, surface_high = select["surface"]["floors"]
    taken: set = set()
    stats = {
        "components_excluded": 0,
        "components_included": 0,
        "tiles_excluded": Counter(),
        "tiles_without_land": Counter(),
    }
    boxes = select["exclude"]
    by_floor: dict[int, set] = {}
    for x, y, z in pending:
        if surface_low <= z <= surface_high:
            if land(x, y, z):
                taken.add((x, y, z))
            else:
                stats["tiles_without_land"][z] += 1
        elif low <= z <= high:
            by_floor.setdefault(z, set()).add((x, y))
        else:
            raise ConvertError(
                f"fill tile on floor {z} is outside the selection floors"
            )
    for z, points in sorted(by_floor.items()):
        seen: set = set()
        for start in sorted(points):
            if start in seen:
                continue
            seen.add(start)
            stack, component = [start], []
            while stack:
                x, y = stack.pop()
                component.append((x, y))
                for near in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                    if near in points and near not in seen:
                        seen.add(near)
                        stack.append(near)
            excluded = any(
                box["floors"][0] <= z <= box["floors"][1]
                and any(
                    box["bbox"][0] <= x <= box["bbox"][2]
                    and box["bbox"][1] <= y <= box["bbox"][3]
                    for x, y in component
                )
                for box in boxes
            )
            if excluded:
                stats["components_excluded"] += 1
                stats["tiles_excluded"][z] += len(component)
            else:
                stats["components_included"] += 1
                taken.update((x, y, z) for x, y in component)
    return taken, stats


def ground_ids(path: Path = GROUND_CLASSES) -> tuple[set[int], set[int]]:
    """The water and the lava ground item ids of the committed island ground classes."""
    classes = json.loads(path.read_bytes())
    found = []
    for name in ("water", "lava"):
        ids: set[int] = set()
        for text in classes[name].values():
            for part in text.split(","):
                low, _, high = part.partition("-")
                ids.update(range(int(low), int(high or low) + 1))
        found.append(ids)
    return found[0], found[1]


def has_destination(items) -> bool:
    return any(
        (attrs or {}).get(DEST_KEY, UNSET_DEST) != UNSET_DEST for *_, attrs in items
    )


def replacement_candidates(raw: bytes, replace: dict, grounds, land) -> dict:
    """The fragment tiles that may replace a base tile: `{(x, y, z): tile}`.

    Land ground (present, neither water nor lava) on the floors of `replace`, where the
    official minimap shows land. A candidate with a house, zone or teleport is refused.
    """
    water, lava = grounds
    low, high = replace["floors"]
    found: dict = {}

    def on_tile(x, y, z, flags, house, zones, items) -> None:
        if not low <= z <= high or not items or items[0][0] in water | lava:
            return
        if not land(x, y, z):
            return
        if house is not None or zones or has_destination(items):
            raise ConvertError(
                f"replacement tile ({x}, {y}, {z}) carries a house, zone or teleport"
            )
        found.setdefault((x, y, z), (flags, house, zones, items))

    otbm_reader.read_tiles(raw, on_tile)
    return found


class Replacer:
    """Feeds the base tiles to the collector, swapping in the candidates over water."""

    def __init__(self, collector: Collector, candidates: dict, water: set[int]):
        self.collector, self.candidates, self.water = collector, candidates, water
        self.by_floor: Counter = Counter()
        self.items_added = self.items_removed = 0

    def __call__(self, x, y, z, flags, house, zones, items) -> None:
        tile = self.candidates.get((x, y, z))
        if tile is not None and items and items[0][0] in self.water:
            if house is not None or zones or has_destination(items):
                raise ConvertError(
                    f"replaced base tile ({x}, {y}, {z}) carries a house, zone or teleport"
                )
            self.by_floor[z] += 1
            self.items_removed += len(items)
            self.items_added += len(tile[3])
            flags, house, zones, items = tile
        self.collector(x, y, z, flags, house, zones, items)


class Overrider:
    """Feeds the base tiles to `inner`, swapping in the Edron rework's replacement tiles."""

    def __init__(self, inner, replacements: dict):
        self.inner, self.replacements = inner, replacements
        self.seen = self.items_added = self.items_removed = 0

    def __call__(self, x, y, z, flags, house, zones, items) -> None:
        tile = self.replacements.get((x, y, z))
        if tile is not None:
            self.seen += 1
            self.items_removed += len(items)
            self.items_added += len(tile[3])
            flags, house, zones, items = tile
        self.inner(x, y, z, flags, house, zones, items)


def apply_edron(collector: Collector, adds: dict, what: str = "edron") -> int:
    """Add a rework's tiles, each at a position that has no tile yet. Returns items."""
    items = 0
    for (x, y, z), tile in sorted(
        adds.items(), key=lambda kv: (kv[0][2], kv[0][0], kv[0][1])
    ):
        sector = collector.sectors.get(
            (z, x // codec.SECTOR_SIZE, y // codec.SECTOR_SIZE)
        )
        if sector and sector_slot(x, y) in sector[0]:
            raise ConvertError(
                f"{what} tile ({x}, {y}, {z}) is added over an existing tile"
            )
        collector(x, y, z, *tile)
        items += len(tile[3])
    return items


def replace_summary(row: dict | None, replacer: Replacer | None) -> dict:
    by_floor = replacer.by_floor if replacer else Counter()
    return {
        "items_added": replacer.items_added if replacer else 0,
        "items_removed": replacer.items_removed if replacer else 0,
        "member": row["member"]["name"] if row else None,
        "rule": REPLACE_RULE,
        "tiles_replaced": sum(by_floor.values()),
        "tiles_replaced_by_floor": {str(z): n for z, n in sorted(by_floor.items())},
    }


def apply_fill(
    collector: Collector,
    raw: bytes,
    width: int,
    height: int,
    select: dict | None = None,
    land=None,
) -> dict:
    """Add the tiles of one fragment map that the collected base map lacks.

    A tile whose position the base map (or an earlier tile of this fragment) already has is
    skipped, so nothing is overwritten or merged. A tile that would be added with a house, a
    tile zone or a teleport destination is refused: houses and teleports are bound to
    `world.otbm` metadata, and this fill carries terrain and decoration only. A teleport
    item whose destination is unset (0, 0, 0) leads nowhere and is carried as an item. With
    `select`, only the tiles `select_tiles` takes are added (in map order); the rest count
    as not selected.
    """
    present: dict[tuple[int, int, int], set[int]] = {}
    pending: dict[tuple[int, int, int], tuple] = {}
    stats = {
        "added": Counter(),
        "items_added": 0,
        "not_selected": Counter(),
        "selection": None,
        "source": Counter(),
        "skipped": Counter(),
    }

    def add(x, y, z, flags, house, zones, items) -> None:
        if house is not None or zones or has_destination(items):
            raise ConvertError(
                f"fill tile ({x}, {y}, {z}) carries a house, zone or teleport"
            )
        collector(x, y, z, flags, house, zones, items)
        stats["added"][z] += 1
        stats["items_added"] += len(items)

    def on_tile(x, y, z, flags, house, zones, items) -> None:
        stats["source"][z] += 1
        if x >= width or y >= height:
            raise ConvertError(f"fill tile ({x}, {y}, {z}) is outside the map extent")
        key = (z, x // codec.SECTOR_SIZE, y // codec.SECTOR_SIZE)
        if key not in present:
            sector = collector.sectors.get(key)
            present[key] = set(sector[0]) if sector else set()
        slot = sector_slot(x, y)
        if slot in present[key]:
            stats["skipped"][z] += 1
            return
        present[key].add(slot)
        if select is None:
            add(x, y, z, flags, house, zones, items)
        else:
            pending[(x, y, z)] = (flags, house, zones, items)

    facts = otbm_reader.read_tiles(raw, on_tile)
    if facts.unknown_item_attrs or facts.unknown_tile_attrs:
        raise ConvertError("fill map has unknown OTBM attributes")
    if select is not None:
        taken, chosen = select_tiles(pending, select, land)
        for position, tile in pending.items():
            if position in taken:
                add(*position, *tile)
            else:
                stats["not_selected"][position[2]] += 1
        stats["selection"] = chosen
    return stats


def fill_summary(row: dict, raw: bytes, stats: dict) -> dict:
    def by_floor(counter: Counter) -> dict[str, int]:
        return {str(z): n for z, n in sorted(counter.items())}

    summary = {
        "archive": row["archive"],
        "items_added": stats["items_added"],
        "member": {**row["member"], "bytes": len(raw)},
        "tiles_added": sum(stats["added"].values()),
        "tiles_added_by_floor": by_floor(stats["added"]),
        "tiles_in_source": sum(stats["source"].values()),
        "tiles_skipped_existing": sum(stats["skipped"].values()),
        "tiles_skipped_existing_by_floor": by_floor(stats["skipped"]),
    }
    if stats["selection"] is not None:
        chosen = stats["selection"]
        summary["selection"] = {
            "components_excluded": chosen["components_excluded"],
            "components_included": chosen["components_included"],
            "tiles_excluded_by_floor": by_floor(chosen["tiles_excluded"]),
            "tiles_not_selected": sum(stats["not_selected"].values()),
            "tiles_without_land_by_floor": by_floor(chosen["tiles_without_land"]),
        }
    return summary


def edron_summary(plan, tibiamaps, sets, overrider, items_added) -> dict:
    counts = {
        "items_added": 0,
        "replaced_items_added": 0,
        "replaced_items_removed": 0,
        "tiles_added": 0,
        "tiles_replaced": 0,
    }
    if plan is None:
        return {"applied": False, **counts}
    return {
        **edron.summarize(plan, tibiamaps, *sets),
        "items_added": items_added,
        "replaced_items_added": overrider.items_added,
        "replaced_items_removed": overrider.items_removed,
    }


def zstd_info() -> dict:
    return {
        "backend": zstandard.backend,
        "libzstd": ".".join(map(str, zstandard.ZSTD_VERSION)),
        "python_package": zstandard.__version__,
    }


def build(
    blobs: dict[str, bytes],
    bindings: bytes | None = None,
    previous: list[dict] | None = None,
    terrain: dict[int, str] | None = None,
    land=None,
    tibiamaps=None,
    blocking: set[int] | None = None,
    kinds: dict[int, str] | None = None,
    yellow: set[int] | None = None,
) -> dict[str, bytes]:
    """Build every output; `previous` is the committed palette to extend, if any and
    `terrain` the committed Terrain keys of appearance-only ids and `land` the minimap land
    test of a partial fill (default: the committed 15.30 minimap). The Edron rework runs when
    `tibiamaps` (decoded, default: the pinned files among `blobs`) is available, with the
    `blocking` ids and floor-change `kinds` of the committed assets and objects."""
    if bindings is None:
        bindings = ITEM_BINDINGS.read_bytes()
    bound = bound_keys(bindings)
    collector = Collector()
    replace_row = next(
        (r for r in FILL if "replace" in r and fill_key(r) in blobs), None
    )
    replacer = None
    if replace_row is not None:
        land = land or minimap_land()
        grounds = ground_ids()
        replacer = Replacer(
            collector,
            replacement_candidates(
                blobs[fill_key(replace_row)], replace_row["replace"], grounds, land
            ),
            grounds[0],
        )
    edron_key = fill_key(FILL[1])
    plan = None
    if tibiamaps is None and all(
        edron.tibiamaps_blob_key(name) in blobs for name, _sha in edron.TIBIAMAPS_FILES
    ):
        tibiamaps = edron.decode_tibiamaps(blobs)
    if tibiamaps is not None:
        if edron_key not in blobs:
            raise ConvertError("the Edron rework needs the summer-update-2025 member")
        if blocking is None or yellow is None:
            found = edron.appearance_ids()
            blocking = found[0] if blocking is None else blocking
            yellow = found[1] if yellow is None else yellow
        kinds = edron.floor_change_kinds() if kinds is None else kinds
        plan = edron.plan_rules(
            edron.read_box(blobs[OTBM], edron.BASE_FLOORS),
            edron.read_box(blobs[edron_key], edron.SUMMER_FLOORS),
            tibiamaps,
            blocking,
        )
    draft_plan = None
    if draft.has_files(blobs):
        images = draft.decode_images(blobs)
        scan = draft.Scanner(images)
        otbm_reader.read_tiles(blobs[OTBM], scan)
        # a tile a fill fragment holds is real data: the draft keeps clear of it
        claimed: set[tuple[int, int, int]] = set()
        for row in FILL:
            raw = blobs.get(fill_key(row))
            if raw is not None:
                otbm_reader.read_tiles(
                    raw,
                    lambda x, y, z, *_rest: (
                        z in draft.FLOORS and claimed.add((x, y, z))
                    ),
                )
        scan.claim(claimed)
        land = land or minimap_land()
        draft_plan = draft.plan_draft(scan, images, land, ground_ids()[0])
        if plan and set(plan.replace) & set(draft_plan.replace):
            raise ConvertError("the draft replaces a tile of the Edron rework")
    draft_over = Overrider(
        replacer or collector, draft_plan.replace if draft_plan else {}
    )
    overrider = Overrider(draft_over, plan.replace if plan else {})
    facts = otbm_reader.read_tiles(blobs[OTBM], overrider)
    if plan and overrider.seen != len(plan.replace):
        raise ConvertError("an Edron replacement has no base tile to replace")
    if draft_plan and draft_over.seen != len(draft_plan.replace):
        raise ConvertError("a draft replacement has no base tile to replace")
    if facts.unknown_item_attrs or facts.unknown_tile_attrs:
        raise ConvertError(
            f"unknown OTBM attributes: items {dict(facts.unknown_item_attrs)}, "
            f"tiles {dict(facts.unknown_tile_attrs)}"
        )
    collector.check()
    if facts.width > 0xFFFF or facts.height > 0xFFFF:
        raise ConvertError("map extent exceeds the region coordinate range")
    if collector.tiles != facts.tiles:
        raise ConvertError("tile count differs from the reader")
    fills, applied = [], []
    for row in FILL:
        raw = blobs.get(fill_key(row))
        if raw is None:
            continue
        if "select" in row and land is None:
            land = minimap_land()
        stats = apply_fill(
            collector, raw, facts.width, facts.height, row.get("select"), land
        )
        fills.append(fill_summary(row, raw, stats))
        applied.append(row)
    edron_items = apply_edron(collector, plan.add) if plan else 0
    draft_items = apply_edron(collector, draft_plan.add, "draft") if draft_plan else 0
    collector.check()
    source = {**SOURCE, "fill": applied}
    if plan:
        source["edron"] = edron.PIN
    if draft_plan:
        source["minimap_draft"] = draft.PIN
    palette = build_palette(bound, collector.occurrences, previous, terrain)
    collector.remap({row["source_item_id"]: i for i, row in enumerate(palette)})
    declared = items_xml_ids(blobs[ITEMS_XML])
    provisional = {"appearance_only": [0, 0], "in_items_xml": [0, 0]}
    terrain_entries = terrain_occurrences = 0
    for row in palette:
        if row["key"].startswith(TERRAIN_KEY_PREFIX):
            terrain_entries += 1
            terrain_occurrences += collector.occurrences[row["source_item_id"]]
        if row["provisional"]:
            kind = (
                "in_items_xml"
                if row["source_item_id"] in declared
                else "appearance_only"
            )
            provisional[kind][0] += 1
            provisional[kind][1] += collector.occurrences[row["source_item_id"]]

    per_region: dict[tuple[int, int, int], dict[int, bytes]] = {}
    region_tiles: Counter = Counter()
    region_items: Counter = Counter()
    floors: Counter = Counter()
    size = codec.SECTORS_PER_SIDE
    for key in sorted(collector.sectors):
        z, sx, sy = key
        region = (z, sx // size, sy // size)
        region_tiles[region] += len(collector.sectors[key][0])
        region_items[region] += collector.sector_items[key]
        floors[z] += len(collector.sectors[key][0])
        payload = collector.sector_payload(key)
        per_region.setdefault(region, {})[(sy % size) * size + sx % size] = payload
    out: dict[str, bytes] = {}
    regions = []
    sector_count = disk = 0
    for region in sorted(per_region):
        z, rx, ry = region
        sectors = per_region[region]
        data = codec.encode_region(z, rx, ry, sectors)
        path = f"{DIRECTORY}/{codec.region_name(z, rx, ry)}"
        out[path] = data
        regions.append(
            {
                "items": region_items[region],
                "path": path,
                "sha256": hashlib.sha256(data).hexdigest(),
                "tiles": region_tiles[region],
            }
        )
        sector_count += len(sectors)
        disk += len(data)
    totals = {
        "items": collector.items,
        "regions": len(regions),
        "sectors": sector_count,
        "tiles": collector.tiles,
    }
    out[f"{DIRECTORY}/index.json"] = canonical(
        {
            "codec": codec.CODEC,
            "coordinate_frame": metadata.COORDINATE_FRAME,
            "family": "WorldPlacement.Base",
            "generator": GENERATOR,
            "item_bindings": {
                "path": str(ITEM_BINDINGS.relative_to(ROOT)),
                "sha256": hashlib.sha256(bindings).hexdigest(),
            },
            "palette": palette,
            "population_state": "POPULATED",
            "region_size": codec.REGION_SIZE,
            "regions": regions,
            "schema": "OTERYN_FAMILY_INDEX/v1",
            "sector_size": codec.SECTOR_SIZE,
            "shards": [row["path"] for row in regions],
            "source": source,
            "totals": totals,
            "zstd_level": codec.ZSTD_LEVEL,
        }
    )
    summary = {
        "bytes_on_disk": disk,
        "codec": {
            "name": codec.CODEC,
            "zstd": zstd_info(),
            "zstd_level": codec.ZSTD_LEVEL,
        },
        "item_attributes": dict(sorted(collector.attributes.items())),
        "map": {
            "floors": [0, metadata.MAX_FLOOR],
            "height": facts.height,
            "otbm_version": facts.version,
            "width": facts.width,
        },
        "palette": {
            "entries": len(palette),
            "provisional": {
                "appearance_only": {
                    "entries": provisional["appearance_only"][0],
                    "occurrences": provisional["appearance_only"][1],
                },
                "entries": sum(v[0] for v in provisional.values()),
                "in_items_xml": {
                    "entries": provisional["in_items_xml"][0],
                    "occurrences": provisional["in_items_xml"][1],
                },
                "occurrences": sum(v[1] for v in provisional.values()),
            },
            "terrain": {
                "entries": terrain_entries,
                "occurrences": terrain_occurrences,
            },
        },
        "rejected_items": {"unsupported_attributes": len(collector.unsupported)},
        "edron": edron_summary(
            plan, tibiamaps, (blocking, kinds, yellow), overrider, edron_items
        ),
        "draft": (
            draft.summarize(draft_plan, draft_items)
            if draft_plan
            else draft.unapplied()
        ),
        "replace": replace_summary(replace_row, replacer),
        "fill": {
            "items_added": sum(f["items_added"] for f in fills),
            "rule": FILL_RULE,
            "sources": fills,
            "tiles_added": sum(f["tiles_added"] for f in fills),
        },
        "schema": "OTERYN_WORLD_BASE_SOURCE_CAPTURE/v1",
        "source": source,
        "tiles_by_floor": {str(z): n for z, n in sorted(floors.items())},
        "tiles_with_house": collector.house_tiles,
        "tiles_with_zone": collector.zone_tiles,
        "totals": totals,
    }
    out[str(SUMMARY.relative_to(ROOT))] = canonical(summary)
    return out


def read_source(
    crystal_root: Path, tibiamaps_root: Path | None = None
) -> dict[str, bytes]:
    blobs = {}
    if tibiamaps_root is not None:
        blobs.update(edron.read_tibiamaps_root(tibiamaps_root))
        blobs.update(draft.read_files(tibiamaps_root))
    for row in SOURCE["files"]:
        data = (crystal_root / row["path"]).read_bytes()
        if hashlib.sha256(data).hexdigest() != row["sha256"]:
            raise ConvertError(f"{row['path']}: sha256 differs from the pinned source")
        blobs[row["path"]] = data
    for row in FILL:
        archive = (crystal_root / row["archive"]["path"]).read_bytes()
        if hashlib.sha256(archive).hexdigest() != row["archive"]["sha256"]:
            raise ConvertError(f"{row['archive']['path']}: sha256 differs from the pin")
        data = extract_member(archive, row["member"]["name"])
        if hashlib.sha256(data).hexdigest() != row["member"]["sha256"]:
            raise ConvertError(
                f"{fill_key(row)}: sha256 differs from the pinned source"
            )
        blobs[fill_key(row)] = data
    return blobs


def extract_member(archive: bytes, name: str) -> bytes:
    """One member of a 7z archive. `py7zr` is needed only to regenerate the fill."""
    try:
        import py7zr
    except ImportError as error:
        raise ConvertError(
            "reading maps.7z needs py7zr (pip install -r requirements-regenerate.txt)"
        ) from error
    with tempfile.TemporaryDirectory() as tmp:
        with py7zr.SevenZipFile(io.BytesIO(archive)) as handle:
            handle.extract(path=tmp, targets=[name])
        return (Path(tmp) / name).read_bytes()


def world_otbm_totals(totals: dict, summary: dict) -> dict:
    """The `world.otbm` totals: the index totals minus every fill and rework change."""
    fill = summary.get("fill", {})
    replace = summary.get("replace", {})
    rework = summary.get("edron", {})
    drafted = summary.get("draft", {})

    def count(row: dict, key: str) -> int:
        return row.get(key, 0)

    return {
        "items": totals["items"]
        - count(fill, "items_added")
        - count(replace, "items_added")
        - count(rework, "items_added")
        - count(rework, "replaced_items_added")
        - count(drafted, "items_added")
        - count(drafted, "replaced_items_added")
        + count(replace, "items_removed")
        + count(rework, "replaced_items_removed")
        + count(drafted, "replaced_items_removed"),
        "tiles": totals["tiles"]
        - count(fill, "tiles_added")
        - count(rework, "tiles_added")
        - count(drafted, "tiles_added"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--crystal-root", type=Path, required=True)
    parser.add_argument(
        "--tibiamaps-root",
        type=Path,
        required=True,
        help="directory holding the pinned tibiamaps files (see edron_rework.py)",
    )
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    args = parser.parse_args()
    try:
        out = build(
            read_source(args.crystal_root, args.tibiamaps_root),
            previous=committed_palette(),
            terrain=terrain_keys(),
        )
        totals = json.loads(out[f"{DIRECTORY}/index.json"])["totals"]
        summary = json.loads(out[str(SUMMARY.relative_to(ROOT))])
        base = world_otbm_totals(totals, summary)
        if base != PINNED_TOTALS:
            raise ConvertError(
                f"world.otbm totals {base} differ from the pinned source {PINNED_TOTALS}"
            )
    except (ConvertError, otbm_reader.OtbmError, OSError) as error:
        print(f"FAIL {error}", file=sys.stderr)
        return 1
    stale = [
        path
        for path, data in out.items()
        if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
    ]
    directory = ROOT / DIRECTORY
    extra = sorted(
        str(p.relative_to(ROOT))
        for p in (directory.iterdir() if directory.is_dir() else [])
        if str(p.relative_to(ROOT)) not in out
    )
    if args.check:
        for path in stale:
            print(f"STALE {path}", file=sys.stderr)
        for path in extra:
            print(f"EXTRA {path}", file=sys.stderr)
        return 1 if stale or extra else 0
    for path in extra:
        (ROOT / path).unlink()
    for path in stale:
        (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / path).write_bytes(out[path])
    print(f"wrote {len(stale)} of {len(out)} files, removed {len(extra)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
