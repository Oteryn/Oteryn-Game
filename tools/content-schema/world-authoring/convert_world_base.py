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

    python convert_world_base.py --crystal-root /path/to/crystalserver [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import sys
import tempfile
from collections import Counter
from itertools import pairwise
from pathlib import Path
from xml.etree import ElementTree

import convert_world_metadata as metadata
import otbm_reader
import world_region_codec as codec
import zstandard
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
    }
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


def apply_fill(collector: Collector, raw: bytes, width: int, height: int) -> dict:
    """Add the tiles of one fragment map that the collected base map lacks.

    A tile whose position the base map (or an earlier tile of this fragment) already has is
    skipped, so nothing is overwritten or merged. A tile that would be added with a house, a
    tile zone or a teleport destination is refused: houses and teleports are bound to
    `world.otbm` metadata, and this fill carries terrain and decoration only.
    """
    present: dict[tuple[int, int, int], set[int]] = {}
    stats = {
        "added": Counter(),
        "items_added": 0,
        "source": Counter(),
        "skipped": Counter(),
    }

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
        if house is not None or zones or any(DEST_KEY in attrs for *_, attrs in items):
            raise ConvertError(
                f"fill tile ({x}, {y}, {z}) carries a house, zone or teleport"
            )
        present[key].add(slot)
        collector(x, y, z, flags, house, zones, items)
        stats["added"][z] += 1
        stats["items_added"] += len(items)

    facts = otbm_reader.read_tiles(raw, on_tile)
    if facts.unknown_item_attrs or facts.unknown_tile_attrs:
        raise ConvertError("fill map has unknown OTBM attributes")
    return stats


def fill_summary(row: dict, raw: bytes, stats: dict) -> dict:
    def by_floor(counter: Counter) -> dict[str, int]:
        return {str(z): n for z, n in sorted(counter.items())}

    return {
        "archive": row["archive"],
        "items_added": stats["items_added"],
        "member": {**row["member"], "bytes": len(raw)},
        "tiles_added": sum(stats["added"].values()),
        "tiles_added_by_floor": by_floor(stats["added"]),
        "tiles_in_source": sum(stats["source"].values()),
        "tiles_skipped_existing": sum(stats["skipped"].values()),
        "tiles_skipped_existing_by_floor": by_floor(stats["skipped"]),
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
) -> dict[str, bytes]:
    """Build every output; `previous` is the committed palette to extend, if any and
    `terrain` the committed Terrain keys of appearance-only ids."""
    if bindings is None:
        bindings = ITEM_BINDINGS.read_bytes()
    bound = bound_keys(bindings)
    collector = Collector()
    facts = otbm_reader.read_tiles(blobs[OTBM], collector)
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
        stats = apply_fill(collector, raw, facts.width, facts.height)
        fills.append(fill_summary(row, raw, stats))
        applied.append(row)
    collector.check()
    source = {**SOURCE, "fill": applied}
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


def read_source(crystal_root: Path) -> dict[str, bytes]:
    blobs = {}
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--crystal-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    args = parser.parse_args()
    try:
        out = build(
            read_source(args.crystal_root),
            previous=committed_palette(),
            terrain=terrain_keys(),
        )
        totals = json.loads(out[f"{DIRECTORY}/index.json"])["totals"]
        fill = json.loads(out[str(SUMMARY.relative_to(ROOT))])["fill"]
        base = {
            "items": totals["items"] - fill["items_added"],
            "tiles": totals["tiles"] - fill["tiles_added"],
        }
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
