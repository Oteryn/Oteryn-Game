#!/usr/bin/env python3
"""Convert the pinned TibiaWiki island snapshot and the base map into `Area.Island` records.

Owner rule: an island is imported only when the committed base map confirms it, that is, when
the wiki's coordinate lies on a land component that water or lava fully encloses. Everything
else is excluded and counted with its reason. Reads, all offline:

- `imports/tibiawiki/islands/fandom-snapshot-v1.json` (`fandom_island_snapshot.py`),
- the `WorldPlacement.Base` region files (`content/world/placements/`),
- `island-ground-classes.json` (the water and lava ground item ids, derived from the pinned
  CrystalServer `items.xml` names), the committed City Areas and the committed teleports,
- `island-evidence-anchors.json`: for wiki pages without a usable coordinate, a start tile that
  a pinned CrystalServer NPC or monster spawn (or a committed teleport destination) places on
  an enclosed component. `--crystal-root` re-checks each spawn against the pinned XML files.

and writes content/world/areas/islands/ plus `samples/islands-capture-v1.json`.

    python convert_islands.py [--snapshot PATH] [--check] [--crystal-root PATH]

Components are found by a bounded 4-neighbour breadth-first search over land tiles of one
floor (a tile is land unless its first, ground, item is water or lava, or the tile is absent).
The search stops at 400,000 tiles: a component that large is the mainland and is excluded as
`part_of_landmass`. `--crystal-root` additionally re-derives the ground classes from the pinned
`items.xml` and fails or rewrites when they differ, and verifies the evidence spawns.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter, deque
from pathlib import Path
from xml.etree import ElementTree

import convert_hunting_places as hunting
import convert_world_base as world_base
import convert_world_metadata as base
import world_region_codec as codec

ROOT = base.ROOT
HERE = base.HERE
SNAPSHOT = "imports/tibiawiki/islands/fandom-snapshot-v1.json"
SUMMARY = "tools/content-schema/world-authoring/samples/islands-capture-v1.json"
GROUND_CLASSES = "tools/content-schema/world-authoring/island-ground-classes.json"
EVIDENCE = "tools/content-schema/world-authoring/island-evidence-anchors.json"
TELEPORTS = "content/world/transitions"
PLACEMENT_INDEX = "content/world/placements/index.json"
GENERATOR = "tools/content-schema/world-authoring/convert_islands.py"
SNAPSHOT_SCHEMA = "OTERYN_TIBIAWIKI_FANDOM_ISLANDS_SNAPSHOT/v1"
GROUND_SCHEMA = "OTERYN_ISLAND_GROUND_CLASSES/v1"
EVIDENCE_SCHEMA = "OTERYN_ISLAND_EVIDENCE_ANCHORS/v1"
SPAWN_KINDS = {
    "data-global/world/world-monster.xml": "crystalserver-monster",
    "data-global/world/world-npc.xml": "crystalserver-npc",
}
SUMMARY_SCHEMA = "OTERYN_ISLANDS_SOURCE_CAPTURE/v1"
FAMILY = "Area.Island"
KEY_PREFIX = "oteryn:area.island."
NAMESPACE = hunting.NAMESPACE
CAP = 400_000
ANCHOR_RADIUS = 5
VOID, LAND, WATER, LAVA = 0, 1, 2, 3
WATER_NAMES = frozenset(
    {
        "bog water",
        "brown sea floor",
        "dirty water",
        "flowing water",
        "muddy water",
        "ocean floor",
        "sea floor",
        "shallow water",
        "water",
        "waterfall",
        "waterway",
        "waterway cataract",
    }
)
LAVA_NAMES = frozenset({"lava", "lava hole", "lava stream", "steaming hot lava"})
REGION_NAME = re.compile(r"region-z(\d{2})-x(\d{3})-y(\d{3})\.b3")
EMPTY_REGION = bytes(codec.REGION_SIZE * codec.REGION_SIZE)


class ConvertError(base.ConvertError):
    pass


# --------------------------------------------------------------------------------------------
# Ground classes (water and lava item ids)
# --------------------------------------------------------------------------------------------


def format_ids(ids: set[int]) -> str:
    """Sorted ids as `a-b,c` ranges."""
    parts, run = [], []
    for value in sorted(ids):
        if run and value == run[-1] + 1:
            run.append(value)
            continue
        if run:
            parts.append(run)
        run = [value]
    parts.append(run)
    return ",".join(str(r[0]) if len(r) == 1 else f"{r[0]}-{r[-1]}" for r in parts if r)


def parse_ids(text: str) -> set[int]:
    ids: set[int] = set()
    for part in text.split(","):
        low, _, high = part.partition("-")
        ids.update(range(int(low), int(high or low) + 1))
    return ids


def items_xml_names(xml: bytes) -> dict[int, str]:
    """Server item id -> lower-cased name for every named `<item>`; ranges expand."""
    names: dict[int, str] = {}
    for element in ElementTree.fromstring(xml).iter("item"):
        name = element.attrib.get("name")
        if name is None:
            continue
        if "fromid" in element.attrib:
            ids = range(int(element.attrib["fromid"]), int(element.attrib["toid"]) + 1)
        else:
            ids = [int(element.attrib["id"])]
        for item_id in ids:
            names[item_id] = name.lower()
    return names


def palette_ids(root: Path) -> set[int]:
    index = json.loads((root / PLACEMENT_INDEX).read_text(encoding="utf-8"))
    return {row["source_item_id"] for row in index["palette"]}


def derive_ground_classes(xml: bytes, palette: set[int]) -> dict:
    """The ground class file: base map palette ids whose `items.xml` name is water or lava."""
    names = items_xml_names(xml)
    document: dict = {
        "lava": {},
        "schema": GROUND_SCHEMA,
        "selection": (
            "Base map palette ids whose items.xml name (case-insensitive) is listed. A tile "
            "is water or lava when its first (ground) item is listed, void when absent and "
            "land otherwise."
        ),
        "source": {
            "path": world_base.ITEMS_XML,
            "repository": base.SOURCE["repository"],
            "revision": base.SOURCE["revision"],
            "sha256": hashlib.sha256(xml).hexdigest(),
        },
        "water": {},
    }
    for section, wanted in (("water", WATER_NAMES), ("lava", LAVA_NAMES)):
        for name in sorted(wanted):
            ids = {i for i in palette if names.get(i) == name}
            if ids:
                document[section][name] = format_ids(ids)
    return document


def load_ground_classes(data: bytes) -> tuple[set[int], set[int]]:
    document = json.loads(data)
    if document.get("schema") != GROUND_SCHEMA:
        raise ConvertError("ground class file schema differs")
    water = set().union(*(parse_ids(v) for v in document["water"].values()))
    lava = set().union(*(parse_ids(v) for v in document["lava"].values()))
    if water & lava or not water or not lava:
        raise ConvertError("ground classes must be non-empty and disjoint")
    return water, lava


# --------------------------------------------------------------------------------------------
# Map access and components
# --------------------------------------------------------------------------------------------


class GroundMap:
    """Per-tile ground class of the committed base map, decoded region by region on demand."""

    def __init__(self, root: Path, water: set[int], lava: set[int]) -> None:
        index = json.loads((root / PLACEMENT_INDEX).read_text(encoding="utf-8"))
        self.root = root
        self.paths: dict[tuple[int, int, int], str] = {}
        for row in index["regions"]:
            match = REGION_NAME.fullmatch(Path(row["path"]).name)
            if not match:
                raise ConvertError(f"{row['path']}: not a region file name")
            self.paths[tuple(int(g) for g in match.groups())] = row["path"]
        self.palette = [
            LAVA
            if p["source_item_id"] in lava
            else WATER
            if p["source_item_id"] in water
            else LAND
            for p in index["palette"]
        ]
        self.regions: dict[tuple[int, int, int], bytes] = {}

    def region(self, z: int, rx: int, ry: int) -> bytes:
        key = (z, rx, ry)
        cached = self.regions.get(key)
        if cached is not None:
            return cached
        tiles = bytearray(codec.REGION_SIZE * codec.REGION_SIZE)
        path = self.paths.get(key)
        if path is not None:
            _, _, _, sectors = codec.decode_region((self.root / path).read_bytes())
            for _, sector in sectors:
                for tile in sector:
                    items = tile[5]
                    tiles[(tile[1] & 255) * 256 + (tile[0] & 255)] = (
                        self.palette[items[0][0]] if items else LAND
                    )
        self.regions[key] = bytes(tiles) if path is not None else EMPTY_REGION
        return self.regions[key]

    def cls(self, x: int, y: int, z: int) -> int:
        if not (0 <= x <= 0xFFFF and 0 <= y <= 0xFFFF):
            return VOID
        return self.region(z, x >> 8, y >> 8)[(y & 255) * 256 + (x & 255)]


def pack(x: int, y: int) -> int:
    return (x << 16) | y


def anchor_tile(cls, x: int, y: int, z: int) -> tuple[int, int] | None:
    """The coordinate itself when it is land, else the nearest land tile within 5 tiles.

    Distance is squared Euclidean; rows are scanned north to south and columns west to east,
    and the first strictly nearer tile wins, so the choice is fixed.
    """
    if cls(x, y, z) == LAND:
        return x, y
    best = None
    for dy in range(-ANCHOR_RADIUS, ANCHOR_RADIUS + 1):
        for dx in range(-ANCHOR_RADIUS, ANCHOR_RADIUS + 1):
            if cls(x + dx, y + dy, z) == LAND:
                d = dx * dx + dy * dy
                if best is None or d < best[0]:
                    best = (d, x + dx, y + dy)
    return None if best is None else (best[1], best[2])


def land_component(cls, x: int, y: int, z: int, cap: int = CAP):
    """`(tiles, boundary, capped)` of the land component holding the land tile (x, y, z).

    `tiles` are packed `x << 16 | y`; `boundary` counts the distinct non-land neighbours per
    class. The search stops as soon as `cap` tiles are found (`capped`).
    """
    seen = {pack(x, y)}
    queue = deque([(x, y)])
    boundary: dict[int, int] = {}
    while queue:
        cx, cy = queue.popleft()
        for nx, ny in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
            key = pack(nx, ny)
            if key in seen or key in boundary:
                continue
            kind = cls(nx, ny, z)
            if kind == LAND:
                seen.add(key)
                if len(seen) >= cap:
                    return seen, Counter(boundary.values()), True
                queue.append((nx, ny))
            else:
                boundary[key] = kind
    return seen, Counter(boundary.values()), False


class Components:
    """Resolves coordinates to enclosed land components, each searched once."""

    def __init__(self, cls, cap: int = CAP) -> None:
        self.cls = cls
        self.cap = cap
        self.owner: dict[tuple[int, int], int] = {}
        self.found: list[dict] = []
        self.capped: set[tuple[int, int]] = set()

    def resolve(self, x: int, y: int, z: int) -> dict:
        """`{verdict, anchor, component}`; verdict is island, part_of_landmass or not_on_map."""
        anchor = anchor_tile(self.cls, x, y, z)
        if anchor is None:
            return {"anchor": None, "component": None, "verdict": "not_on_map"}
        key = (z, pack(*anchor))
        if key in self.owner:
            return self._island(anchor, self.owner[key])
        if key in self.capped:
            return {"anchor": anchor, "component": None, "verdict": "part_of_landmass"}
        tiles, boundary, capped = land_component(self.cls, *anchor, z, self.cap)
        if capped:
            self.capped.update((z, k) for k in tiles)
            return {"anchor": anchor, "component": None, "verdict": "part_of_landmass"}
        xs = [k >> 16 for k in tiles]
        ys = [k & 0xFFFF for k in tiles]
        index = len(self.found)
        self.found.append(
            {
                "boundary": {
                    "lava": boundary[LAVA],
                    "void": boundary[VOID],
                    "water": boundary[WATER],
                },
                "floor": z,
                "max_x": max(xs),
                "max_y": max(ys),
                "min_x": min(xs),
                "min_y": min(ys),
                "tile_count": len(tiles),
            }
        )
        self.owner.update(((z, k), index) for k in tiles)
        return self._island(anchor, index)

    def _island(self, anchor, index: int) -> dict:
        return {"anchor": anchor, "component": index, "verdict": "island"}

    def contains(self, index: int, x: int, y: int, z: int) -> bool:
        return self.owner.get((z, pack(x, y))) == index


# --------------------------------------------------------------------------------------------
# Evidence anchors (start tiles for pages without a wiki coordinate)
# --------------------------------------------------------------------------------------------


def committed_teleports(root: Path) -> dict[str, dict]:
    """Teleport key -> declaration for every committed `Transition.Teleport`."""
    directory = root / TELEPORTS
    index = json.loads((directory / "index.json").read_text(encoding="utf-8"))
    found = {}
    for shard in index["shards"]:
        for record in json.loads((root / shard).read_text(encoding="utf-8"))["records"]:
            found[record["declaration"]["identity"]["key"]] = record["declaration"]
    return found


def load_evidence(data: bytes, snapshot: dict, root: Path) -> dict[str, dict]:
    """Title -> anchor entry; every entry is tied to its snapshot page and its evidence."""
    document = json.loads(data)
    if document.get("schema") != EVIDENCE_SCHEMA:
        raise ConvertError("evidence anchor file schema differs")
    pages = {row["title"]: row for row in snapshot["pages"]}
    teleports = None
    found: dict[str, dict] = {}
    for entry in document["anchors"]:
        title = entry["title"]
        page = pages.get(title)
        if page is None or page["pageid"] != entry["pageid"] or title in found:
            raise ConvertError(
                f"{title}: evidence anchor is not a unique snapshot page"
            )
        if entry.get("underground", False) != (entry["floor"] > 7):
            raise ConvertError(
                f"{title}: underground must be set exactly below floor 7"
            )
        source = entry["anchor_source"]
        if "spawn" in entry:
            spawn = entry["spawn"]
            expected = f"{SPAWN_KINDS[spawn['file']]}:{spawn['name']}"
            if source != expected or (spawn["x"], spawn["y"]) != (
                entry["x"],
                entry["y"],
            ):
                raise ConvertError(f"{title}: anchor differs from its spawn evidence")
        else:
            teleports = teleports or committed_teleports(root)
            key = source.removeprefix("teleport:")
            destination = teleports.get(key, {}).get("to")
            if not destination or (
                destination["x"],
                destination["y"],
                destination["floor"],
            ) != (entry["x"], entry["y"], entry["floor"]):
                raise ConvertError(f"{title}: anchor is not the destination of {key}")
        found[title] = entry
    return found


def spawn_positions(xml: bytes) -> set[tuple[str, int, int, int]]:
    """(name, x, y, z) of every `<npc>` or `<monster>` entry: centre plus the entry offset."""
    positions = set()
    for group in ElementTree.fromstring(xml):
        cx, cy = int(group.attrib["centerx"]), int(group.attrib["centery"])
        for member in group:
            positions.add(
                (
                    member.attrib["name"],
                    cx + int(member.attrib["x"]),
                    cy + int(member.attrib["y"]),
                    int(member.attrib["z"]),
                )
            )
    return positions


def verify_spawns(crystal_root: Path, data: bytes) -> None:
    """Check the pinned XML digests and that every evidence spawn is present in them."""
    document = json.loads(data)
    positions = {}
    for row in document["source"]["files"]:
        xml = (crystal_root / row["path"]).read_bytes()
        if hashlib.sha256(xml).hexdigest() != row["sha256"]:
            raise ConvertError(f"{row['path']}: sha256 differs from the pinned source")
        positions[row["path"]] = spawn_positions(xml)
    if document["source"]["revision"] != base.SOURCE["revision"]:
        raise ConvertError("evidence source revision differs from the pinned source")
    for entry in document["anchors"]:
        spawn = entry.get("spawn")
        if (
            spawn
            and (
                spawn["name"],
                spawn["x"],
                spawn["y"],
                spawn["z"],
            )
            not in positions[spawn["file"]]
        ):
            raise ConvertError(f"{entry['title']}: spawn is not in {spawn['file']}")


# --------------------------------------------------------------------------------------------
# Snapshot
# --------------------------------------------------------------------------------------------


def load_snapshot(data: bytes) -> dict:
    snapshot = json.loads(data)
    if snapshot.get("schema") != SNAPSHOT_SCHEMA:
        raise ConvertError("snapshot schema differs")
    rows = snapshot["pages"]
    ids = [row["pageid"] for row in rows]
    titles = [row["title"] for row in rows]
    if not ids or len(set(ids)) != len(ids) or len(set(titles)) != len(titles):
        raise ConvertError("snapshot pages are empty or repeat a page id or title")
    if any(row["pageid"] < 1 or row["revid"] < 1 for row in rows):
        raise ConvertError("snapshot page or revision id is not positive")
    known = set(titles) | {row["title"] for row in snapshot["coordinate_pages"]}
    for row in rows:
        for name in ("alias_of", "place_within"):
            if name in row and row[name] not in titles:
                raise ConvertError(f"{row['title']}: {name} is not a snapshot page")
        for pointer in [c for c in row["coordinates"] if "source_page" in c] + [
            row.get("map_correction", {})
        ]:
            if pointer and pointer["source_page"] not in known:
                raise ConvertError(
                    f"{row['title']}: source page is not in the snapshot"
                )
    return snapshot


def city_index(root: Path) -> dict[str, dict]:
    """Lower-cased AREAS-1 city name -> {key, temple} (`temple` None without a hometown).

    The city records are owned by `area-authoring` (`content/world/areas/cities/areas-*.json`,
    keys `oteryn:content.area.city.<slug>`); this tool only reads them.
    """
    cities = {}
    for shard in sorted((root / "content/world/areas/cities").glob("areas-*.json")):
        for record in json.loads(shard.read_text(encoding="utf-8"))["areas"]:
            hometown = record["hometown"]
            temple = hometown and hometown["temple"]
            cities[record["name"].lower()] = {
                "key": record["identity"]["key"],
                "temple": temple
                and {"x": temple["x"], "y": temple["y"], "floor": temple["z"]},
            }
    return cities


def source(
    snapshot_bytes: bytes, snapshot: dict, root: Path, ground: bytes, evidence: bytes
) -> dict:
    return {
        "base_map": {
            "path": PLACEMENT_INDEX,
            "sha256": hashlib.sha256((root / PLACEMENT_INDEX).read_bytes()).hexdigest(),
        },
        "evidence": "Derived",
        "evidence_anchors": {
            "path": EVIDENCE,
            "sha256": hashlib.sha256(evidence).hexdigest(),
        },
        "fetched_at": snapshot["fetched_at"],
        "ground_classes": {
            "path": GROUND_CLASSES,
            "sha256": hashlib.sha256(ground).hexdigest(),
        },
        "license": snapshot["license"],
        "site": snapshot["source_url"],
        "snapshot": {
            "path": SNAPSHOT,
            "sha256": hashlib.sha256(snapshot_bytes).hexdigest(),
        },
        "source_key": hunting.SOURCE_KEY,
    }


# --------------------------------------------------------------------------------------------
# Evaluation
# --------------------------------------------------------------------------------------------


def evaluate(
    row: dict, components: Components, cities: dict, anchors: dict | None = None
) -> dict:
    """Resolve one page to its enclosed components, or to the reason it has none.

    An evidence anchor (`use: only`) replaces the wiki coordinates, which must not themselves
    lie on an island; `use: primary` puts the anchor's component first and keeps the wiki
    coordinates, so a page can own two components (Newhaven: island, then temple islet).
    """
    title = row["title"]
    results = []
    correction = row.get("map_correction")
    coordinates = list(row["coordinates"])
    for coordinate in coordinates:
        if coordinate["origin"] == "city_temple":
            city = cities.get(coordinate["city"].lower())
            if city is None:
                # A CrystalServer town with no AREAS-1 city: the snapshot pins the
                # coordinate, there is no temple here to compare it with.
                continue
            temple = city["temple"]
            if not temple or (temple["x"], temple["y"]) != (
                coordinate["x"],
                coordinate["y"],
            ):
                raise ConvertError(f"{title}: city temple differs from the snapshot")
    if correction:
        # The wiki coordinate must not itself be a confirmed island, or no correction is due.
        wiki = components.resolve(
            coordinates[0]["x"], coordinates[0]["y"], coordinates[0]["floor"]
        )
        if wiki["verdict"] == "island":
            raise ConvertError(
                f"{title}: the wiki coordinate is an island, no correction"
            )
        coordinates = [{**correction, "origin": "map_correction"}]
    entry = (anchors or {}).get(title)
    if entry:
        anchor = {
            "floor": entry["floor"],
            "origin": "evidence_anchor",
            "x": entry["x"],
            "y": entry["y"],
        }
        if entry["use"] == "only":
            for coordinate in coordinates:
                wiki = components.resolve(
                    coordinate["x"], coordinate["y"], coordinate["floor"]
                )
                if wiki["verdict"] == "island":
                    raise ConvertError(
                        f"{title}: the wiki coordinate is an island, no evidence anchor"
                    )
            coordinates = [anchor]
        else:
            coordinates = [anchor, *coordinates]
    for coordinate in coordinates:
        found = components.resolve(
            coordinate["x"], coordinate["y"], coordinate["floor"]
        )
        if coordinate["origin"] == "evidence_anchor" and (
            found["verdict"] != "island"
            or found["anchor"] != (coordinate["x"], coordinate["y"])
        ):
            raise ConvertError(
                f"{title}: evidence anchor is not a land tile of an island"
            )
        results.append({**found, "coordinate": coordinate})
    enclosed = []
    for result in results:
        if result["verdict"] == "island" and result["component"] not in [
            r["component"] for r in enclosed
        ]:
            enclosed.append(result)
    if enclosed:
        return {"results": enclosed, "row": row}
    if not results:
        reason = "no_coordinates"
    elif any(r["verdict"] == "part_of_landmass" for r in results):
        reason = "part_of_landmass"
    else:
        reason = "event_only_not_on_map" if row["event_only"] else "not_on_map"
    return {"reason": reason, "results": [], "row": row}


def coordinate_position(row: dict) -> dict:
    return base.position(row["x"], row["y"], row["floor"])


def footprint(component: dict) -> dict:
    return {
        "coordinate_frame": base.COORDINATE_FRAME,
        "floor": component["floor"],
        "max_x": component["max_x"],
        "max_y": component["max_y"],
        "min_x": component["min_x"],
        "min_y": component["min_y"],
        "tile_count": component["tile_count"],
    }


def resolve_relations(evaluated: dict[str, dict]) -> dict[str, list[dict]]:
    """Apply `place_within` (excluded) and `alias_of` (merged); fail closed on other overlaps.

    Returns owner title -> alias evaluations. Excluded rows gain a `reason`.
    """
    aliases: dict[str, list[dict]] = {}
    for title, item in sorted(evaluated.items()):
        row = item["row"]
        if "place_within" in row and "reason" not in item:
            target = evaluated[row["place_within"]]
            mine = {r["component"] for r in item["results"]}
            if not mine <= {r["component"] for r in target["results"]}:
                raise ConvertError(f"{title}: not inside {row['place_within']}")
            item.update(reason="part_of_landmass", results=[])
            item["detail"] = f"place within {row['place_within']}"
        elif "alias_of" in row and "reason" not in item:
            target = evaluated[row["alias_of"]]
            mine = [r["component"] for r in item["results"]]
            if mine != [r["component"] for r in target["results"]]:
                raise ConvertError(
                    f"{title}: does not share {row['alias_of']}'s component"
                )
            aliases.setdefault(row["alias_of"], []).append(item)
    owners: dict[int, str] = {}
    for title, item in sorted(evaluated.items()):
        if "reason" in item or "alias_of" in item["row"]:
            continue
        if item["row"]["wiki_class"] == "archipelago" and len(item["results"]) > 1:
            continue
        for result in item["results"]:
            component = result["component"]
            if component in owners:
                raise ConvertError(
                    f"{title} shares a component with {owners[component]}"
                )
            owners[component] = title
    return aliases


def records_for(
    evaluated: dict[str, dict],
    aliases: dict[str, list[dict]],
    components: Components,
    cities: dict,
    root: Path,
    counts: Counter,
    anchors: dict,
) -> list[dict]:
    owners = {
        title: item
        for title, item in evaluated.items()
        if "reason" not in item and "alias_of" not in item["row"]
    }
    keys = base.assign_keys(
        [(str(i["row"]["pageid"]), t) for t, i in sorted(owners.items())],
        base.committed_keys(root, FAMILY, NAMESPACE),
        KEY_PREFIX,
        FAMILY,
    )
    records = []
    for title, item in sorted(owners.items()):
        row = item["row"]
        merged = [row, *(a["row"] for a in aliases.get(title, []))]
        key = keys[str(row["pageid"])]
        results = item["results"]
        is_archipelago = row["wiki_class"] == "archipelago" and len(results) > 1
        if is_archipelago:
            results = sorted(
                results,
                key=lambda r: (
                    components.found[r["component"]]["min_x"],
                    components.found[r["component"]]["min_y"],
                ),
            )
        kind = "archipelago" if is_archipelago else row.get("kind_hint", "island")
        primary = results[0] if not is_archipelago else None
        declaration: dict = {
            "area_kind": kind,
            "identity": {"key": key, "revision": base.REVISION},
            "kind": "Area",
            "name": title,
        }
        if len(merged) > 1:
            declaration["also_known_as"] = sorted(r["title"] for r in merged[1:])
            counts["also_known_as"] += 1
        if any(r["event_only"] for r in merged):
            declaration["event_only"] = True
            counts["event_only"] += 1
        inside: set[str] = set()
        boundary: Counter = Counter()
        tile_count = 0
        rows_out = []
        for result in results:
            component = components.found[result["component"]]
            floor = component["floor"]
            city_keys = sorted(
                c["key"]
                for c in cities.values()
                if c["temple"]
                and c["temple"]["floor"] == floor
                and components.contains(
                    result["component"], c["temple"]["x"], c["temple"]["y"], floor
                )
            )
            inside.update(city_keys)
            boundary.update(component["boundary"])
            tile_count += component["tile_count"]
            rows_out.append(
                {
                    "anchor": base.position(*result["anchor"], floor),
                    "footprint": footprint(component),
                    "component": result["component"],
                }
            )
        if inside:
            declaration["cities"] = [base.ref("Area", k) for k in sorted(inside)]
            counts["cities"] += 1
        if is_archipelago:
            declaration["components"] = [
                {"anchor": r["anchor"], "footprint": r["footprint"]} for r in rows_out
            ]
            counts["components"] += len(rows_out)
        else:
            declaration["anchor"] = rows_out[0]["anchor"]
            declaration["footprint"] = rows_out[0]["footprint"]
            counts["components"] += len(rows_out)
            if len(rows_out) > 1:
                declaration["additional_components"] = [
                    {"anchor": r["anchor"], "footprint": r["footprint"]}
                    for r in rows_out[1:]
                ]
                counts["additional_components"] += 1
        facts: dict = {"evidence": row["evidence"], "wiki_class": row["wiki_class"]}
        if "evidence_page" in row:
            facts["evidence_page"] = row["evidence_page"]
        if row.get("status"):
            facts["wiki_status"] = row["status"]
        if row.get("removed"):
            facts["removed_from_game"] = True
        wiki_cities = sorted(
            {
                cities[n.lower()]["key"]
                for n in row.get("wiki_cities", [])
                if n.lower() in cities
            }
        )
        counts["wiki_city_unmatched"] += len(row.get("wiki_cities", [])) - len(
            wiki_cities
        )
        if wiki_cities:
            facts["wiki_cities"] = [base.ref("Area", k) for k in wiki_cities]
            counts["wiki_cities"] += 1
        if primary is not None:
            coordinate = primary["coordinate"]
            facts["anchor_origin"] = coordinate["origin"]
            if coordinate["origin"] == "evidence_anchor":
                entry = anchors[title]
                facts["anchor_source"] = entry["anchor_source"]
                if "note" in entry:
                    facts["component_note"] = entry["note"]
                if entry.get("underground"):
                    declaration["underground"] = True
                    counts["underground"] += 1
                counts["evidence_anchored"] += 1
            elif "map_correction" in row:
                declaration["anchor_corrected_from_wiki"] = True
                wiki = row["coordinates"][0]
                facts["source_coordinate"] = coordinate_position(wiki)
                facts["anchor_correction_note"] = row["map_correction"]["note"]
                counts["anchor_corrected"] += 1
            elif (coordinate["x"], coordinate["y"]) != tuple(primary["anchor"]):
                facts["source_coordinate"] = coordinate_position(coordinate)
                counts["anchor_moved"] += 1
        declaration["source_facts"] = facts
        bindings = [
            hunting.wiki_binding(key, r)
            for r in sorted(merged, key=lambda r: r["pageid"])
        ]
        records.append({"declaration": declaration, "source_bindings": bindings})
        item["record"] = {
            "area_kind": kind,
            "boundary_tiles": dict(sorted(boundary.items())),
            "components": len(rows_out),
            "key": key,
            "tile_count": tile_count,
            "title": title,
        }
    # An archipelago component that is also an island of its own links to that record.
    by_footprint = {
        tuple(sorted(r["declaration"]["footprint"].items())): r["declaration"][
            "identity"
        ]["key"]
        for r in records
        if "footprint" in r["declaration"]
    }
    for record in records:
        for part in record["declaration"].get("components", []):
            island = by_footprint.get(tuple(sorted(part["footprint"].items())))
            if island:
                part["island"] = base.ref("Area", island)
                counts["component_island_links"] += 1
    return base.unique(records, FAMILY)


def build(
    snapshot_bytes: bytes, root: Path = ROOT, cls=None, cap: int | None = None
) -> dict[str, bytes]:
    snapshot = load_snapshot(snapshot_bytes)
    ground = (root / GROUND_CLASSES).read_bytes()
    evidence = (root / EVIDENCE).read_bytes()
    anchors = load_evidence(evidence, snapshot, root)
    if cls is None:
        water, lava = load_ground_classes(ground)
        cls = GroundMap(root, water, lava).cls
    cities = city_index(root)
    cap = CAP if cap is None else cap
    components = Components(cls, cap)
    evaluated = {
        row["title"]: evaluate(row, components, cities, anchors)
        for row in snapshot["pages"]
    }
    aliases = resolve_relations(evaluated)
    counts: Counter = Counter()
    records = records_for(evaluated, aliases, components, cities, root, counts, anchors)
    pinned = source(snapshot_bytes, snapshot, root, ground, evidence)
    excluded = [
        {
            **({"detail": item["detail"]} if "detail" in item else {}),
            "pageid": item["row"]["pageid"],
            "reason": item["reason"],
            "title": title,
        }
        for title, item in sorted(evaluated.items())
        if "reason" in item
    ]
    included = sorted(
        (item["record"] for item in evaluated.values() if "record" in item),
        key=lambda r: r["key"],
    )
    kinds = Counter(r["area_kind"] for r in included)
    out = base.shard_files(FAMILY, records, pinned, GENERATOR)
    summary = {
        "area_kinds": dict(sorted(kinds.items())),
        "excluded": excluded,
        "excluded_reasons": dict(
            sorted(Counter(e["reason"] for e in excluded).items())
        ),
        "families": {FAMILY: len(records)},
        "included": included,
        "parameters": {"anchor_radius": ANCHOR_RADIUS, "component_cap": cap},
        "records_with": {
            "additional_components": counts["additional_components"],
            "also_known_as": counts["also_known_as"],
            "anchor_corrected": counts["anchor_corrected"],
            "anchor_moved": counts["anchor_moved"],
            "cities": counts["cities"],
            "component_island_links": counts["component_island_links"],
            "components": counts["components"],
            "event_only": counts["event_only"],
            "evidence_anchored": counts["evidence_anchored"],
            "underground": counts["underground"],
            "wiki_cities": counts["wiki_cities"],
        },
        "schema": SUMMARY_SCHEMA,
        "snapshot_pages": len(snapshot["pages"]),
        "source": pinned,
        "wiki_cities_unmatched": counts["wiki_city_unmatched"],
    }
    out[SUMMARY] = base.canonical(summary)
    return out


def verify_ground_classes(crystal_root: Path, root: Path) -> tuple[bytes, bool]:
    """Re-derive the ground class file from the pinned items.xml; return (bytes, stale)."""
    pinned = next(
        f["sha256"]
        for f in world_base.SOURCE["files"]
        if f["path"] == world_base.ITEMS_XML
    )
    xml = (crystal_root / world_base.ITEMS_XML).read_bytes()
    if hashlib.sha256(xml).hexdigest() != pinned:
        raise ConvertError("items.xml sha256 differs from the pinned source")
    derived = base.canonical(derive_ground_classes(xml, palette_ids(root)))
    committed = root / GROUND_CLASSES
    return derived, not committed.is_file() or committed.read_bytes() != derived


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--snapshot", type=Path, default=ROOT / SNAPSHOT)
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    parser.add_argument(
        "--crystal-root",
        type=Path,
        help="pinned crystalserver checkout: re-derive the ground classes from items.xml "
        "and verify the evidence spawns",
    )
    args = parser.parse_args()
    try:
        extra: dict[str, bytes] = {}
        if args.crystal_root:
            verify_spawns(args.crystal_root, (ROOT / EVIDENCE).read_bytes())
            derived, stale = verify_ground_classes(args.crystal_root, ROOT)
            if stale:
                extra[GROUND_CLASSES] = derived
            if stale and not args.check:
                (ROOT / GROUND_CLASSES).write_bytes(derived)
                extra = {}
        out = build(args.snapshot.read_bytes())
    except (base.ConvertError, OSError, KeyError, json.JSONDecodeError) as error:
        print(f"FAIL {error!r}", file=sys.stderr)
        return 1
    stale = [
        path
        for path, data in out.items()
        if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
    ]
    stale += sorted(extra)
    if args.check:
        for path in stale:
            print(f"STALE {path}", file=sys.stderr)
        return 1 if stale else 0
    for path in stale:
        if path in out:
            (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
            (ROOT / path).write_bytes(out[path])
    print(f"wrote {len(stale)} of {len(out)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
