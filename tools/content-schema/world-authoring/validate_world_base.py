#!/usr/bin/env python3
"""Structural and semantic validation of the committed WorldPlacement.Base family.

Decodes every region file, so the run takes about a minute on four cores.

    python validate_world_base.py [--root REPOSITORY_ROOT] [--workers N]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from collections import Counter
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import edron_rework as edron
import minimap_draft as draft
import world_region_codec as codec
from convert_world_base import (
    DONOR_PREFIX,
    FILL,
    FILL_RULE,
    ITEM_NAMESPACE,
    PINNED_TOTALS,
    REPLACE_RULE,
    TERRAIN_DIRECTORY,
    TERRAIN_KEY_PREFIX,
    TERRAIN_NAMESPACE,
    world_otbm_totals,
)

HERE = Path(__file__).resolve().parent
DIRECTORY = "content/world/placements"
INDEX = f"{DIRECTORY}/index.json"
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
ITEM_BINDINGS = "imports/crystalserver/bindings/items.json"
REGION_PATH = re.compile(
    rf"^{re.escape(DIRECTORY)}/region-z(\d{{2}})-x(\d{{3}})-y(\d{{3}})\.b3$"
)
SHA256 = re.compile(r"^[0-9a-f]{64}$")
INDEX_KEYS = {
    "codec",
    "coordinate_frame",
    "family",
    "generator",
    "item_bindings",
    "palette",
    "population_state",
    "region_size",
    "regions",
    "schema",
    "sector_size",
    "shards",
    "source",
    "totals",
    "zstd_level",
}
REGION_KEYS = {"items", "path", "sha256", "tiles"}
TOTAL_KEYS = {"items", "regions", "sectors", "tiles"}
PALETTE_KEYS = {"key", "provisional", "source_item_id"}
RETIRED_KEYS = PALETTE_KEYS | {"retired"}


class ValidationError(Exception):
    pass


def canonical(value) -> bytes:
    text = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return (text + "\n").encode()


def load(root: Path, path: str, strict: bool = True):
    try:
        raw = (root / path).read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        raise ValidationError(f"{path}: unreadable ({error})") from error
    if strict and raw != canonical(value):
        raise ValidationError(f"{path}: not canonical JSON")
    return value


_STATE: dict = {}


def _init(root: str, extent: tuple[int, int], palette_size: int) -> None:
    _STATE.update(root=Path(root), extent=extent, palette_size=palette_size)


def check_region(row: dict) -> dict:
    """Validate one region file in isolation; runs in a worker process."""
    path = row["path"]
    result = {
        "errors": [],
        "size": 0,
        "sectors": 0,
        "tiles": 0,
        "items": 0,
        "houses": 0,
        "zones": 0,
        "z": -1,
        "attributes": Counter(),
        "palette": Counter(),
    }
    errors = result["errors"]
    try:
        data = (_STATE["root"] / path).read_bytes()
    except OSError as error:
        errors.append(f"{path}: unreadable ({error})")
        return result
    result["size"] = len(data)
    if hashlib.sha256(data).hexdigest() != row["sha256"]:
        errors.append(f"{path}: sha256 differs from the index")
    match = REGION_PATH.match(path)
    seen: set[int] = set()
    try:
        z, rx, ry, sectors = codec.decode_region(data, seen)
    except (codec.CodecError, OverflowError) as error:
        errors.append(f"{path}: {error}")
        return result
    if match is None or (z, rx, ry) != tuple(map(int, match.groups())):
        errors.append(f"{path}: header floor/region differs from the file name")
    result["z"] = z
    result["sectors"] = len(sectors)
    width, height = _STATE["extent"]
    tiles = items = houses = zones = 0
    attributes = result["attributes"]
    used = result["palette"]
    size = _STATE["palette_size"]
    if any(p >= size for p in seen):
        errors.append(
            f"{path}: palette indexes outside 0..{size - 1}: "
            f"{sorted(p for p in seen if p >= size)[:10]}"
        )
        return result
    for local, sector in sectors:
        if not sector:
            errors.append(f"{path}: sector {local} is empty")
        previous = (-1, -1)
        for x, y, _flags, house, tile_zones, tile_items in sector:
            if (y, x) <= previous:
                errors.append(f"{path}: tiles not strictly sorted at ({x}, {y})")
                break
            previous = (y, x)
            if not (x < width and y < height):
                errors.append(f"{path}: tile ({x}, {y}) outside the map extent")
                break
            if (x // codec.REGION_SIZE, y // codec.REGION_SIZE) != (rx, ry):
                errors.append(f"{path}: tile ({x}, {y}) outside its region")
                break
            houses += house != 0
            zones += bool(tile_zones)
            items += len(tile_items)
            for palette, depth, attrs in tile_items:
                used[palette] += 1
                if depth:
                    attributes["depth"] += 1
                if attrs:
                    attributes.update(attrs.keys())
        tiles += len(sector)
    if (tiles, items) != (row["tiles"], row["items"]):
        errors.append(
            f"{path}: decoded {tiles} tiles/{items} items, index says {row['tiles']}/{row['items']}"
        )
    result.update(tiles=tiles, items=items, houses=houses, zones=zones)
    return result


def check_index(index: dict, summary: dict, errors: list[str]) -> None:
    if set(index) != INDEX_KEYS:
        errors.append(f"{INDEX}: keys must be exactly {sorted(INDEX_KEYS)}")
    expected = {
        "codec": codec.CODEC,
        "family": "WorldPlacement.Base",
        "population_state": "POPULATED",
        "region_size": codec.REGION_SIZE,
        "schema": "OTERYN_FAMILY_INDEX/v1",
        "sector_size": codec.SECTOR_SIZE,
        "zstd_level": codec.ZSTD_LEVEL,
    }
    for key, value in expected.items():
        if index.get(key) != value:
            errors.append(f"{INDEX}: {key} must be {value!r}")
    if index.get("source") != summary.get("source"):
        errors.append(f"{INDEX}: source differs from the capture summary")
    check_fill(index, summary, errors)
    check_replace(index, summary, errors)
    check_edron(index, summary, errors)
    check_draft(index, summary, errors)
    frame = index.get("coordinate_frame")
    if frame != "global-target-2026-09-27":
        errors.append(f"{INDEX}: unexpected coordinate_frame {frame!r}")
    bindings = index.get("item_bindings", {})
    if bindings.get("path") != ITEM_BINDINGS or not SHA256.match(
        str(bindings.get("sha256"))
    ):
        errors.append(
            f"{INDEX}: item_bindings must name {ITEM_BINDINGS} and its sha256"
        )
    generator = str(index.get("generator"))
    if not (HERE.parents[2] / generator).is_file():
        errors.append(f"{INDEX}: generator {generator!r} does not exist")


def check_fill(index: dict, summary: dict, errors: list[str]) -> None:
    """The fill pins are a subset of the committed ones and every count adds up."""
    pins = (index.get("source") or {}).get("fill")
    fill = summary.get("fill")
    if not isinstance(pins, list) or not isinstance(fill, dict):
        errors.append(f"{SUMMARY}: fill pins and summary are required")
        return
    if any(row not in FILL for row in pins):
        errors.append(f"{INDEX}: source.fill differs from the pinned fill sources")
    sources = fill.get("sources")
    if set(fill) != {"items_added", "rule", "sources", "tiles_added"} or not isinstance(
        sources, list
    ):
        errors.append(
            f"{SUMMARY}: fill must hold items_added, rule, sources, tiles_added"
        )
        return
    if fill["rule"] != FILL_RULE:
        errors.append(f"{SUMMARY}: fill rule differs from the documented one")
    if [(s.get("archive"), s.get("member", {}).get("name")) for s in sources] != [
        (p["archive"], p["member"]["name"]) for p in pins
    ]:
        errors.append(f"{SUMMARY}: fill sources differ from the pinned fill sources")
    for source in sources:
        added, skipped = (
            source.get("tiles_added_by_floor"),
            source.get("tiles_skipped_existing_by_floor"),
        )
        if (
            not isinstance(added, dict)
            or not isinstance(skipped, dict)
            or sum(added.values()) != source.get("tiles_added")
            or sum(skipped.values()) != source.get("tiles_skipped_existing")
            or source["tiles_added"]
            + source["tiles_skipped_existing"]
            + source.get("selection", {}).get("tiles_not_selected", 0)
            != source.get("tiles_in_source")
        ):
            errors.append(
                f"{SUMMARY}: fill counts of {source.get('member')} do not add up"
            )
    if fill["tiles_added"] != sum(s.get("tiles_added", 0) for s in sources) or fill[
        "items_added"
    ] != sum(s.get("items_added", 0) for s in sources):
        errors.append(f"{SUMMARY}: fill totals differ from the sources")


def check_replace(index: dict, summary: dict, errors: list[str]) -> None:
    """The recorded replacement matches its pin and stays inside what the fill skipped."""
    replace = summary.get("replace")
    pins = [p for p in (index.get("source") or {}).get("fill", []) if "replace" in p]
    if not isinstance(replace, dict) or set(replace) != {
        "items_added",
        "items_removed",
        "member",
        "rule",
        "tiles_replaced",
        "tiles_replaced_by_floor",
    }:
        errors.append(f"{SUMMARY}: replace record is missing or has the wrong keys")
        return
    if replace["rule"] != REPLACE_RULE:
        errors.append(f"{SUMMARY}: replace rule differs from the documented one")
    by_floor = replace["tiles_replaced_by_floor"]
    if (
        not isinstance(by_floor, dict)
        or sum(by_floor.values()) != replace["tiles_replaced"]
    ):
        errors.append(f"{SUMMARY}: replaced tiles do not add up")
        return
    if not pins:
        if replace["tiles_replaced"] or replace["member"] is not None:
            errors.append(f"{SUMMARY}: tiles are replaced but no fill pins a rule")
        return
    pin = pins[0]
    low, high = pin["replace"]["floors"]
    sources = {
        s.get("member", {}).get("name"): s
        for s in summary.get("fill", {}).get("sources", [])
    }
    source = sources.get(pin["member"]["name"], {})
    skipped = source.get("tiles_skipped_existing_by_floor", {})
    if replace["member"] != pin["member"]["name"]:
        errors.append(f"{SUMMARY}: replace member differs from the pinned fill")
    for floor, count in by_floor.items():
        if not low <= int(floor) <= high or count > skipped.get(floor, 0):
            errors.append(
                f"{SUMMARY}: floor {floor} replaces {count} tiles outside the pinned rule"
            )


DRAFT_KEYS = {
    "applied",
    "areas",
    "drafted_land_on_official_land",
    "items_added",
    "mapping",
    "replaced_items_added",
    "replaced_items_removed",
    "tiles_added",
    "tiles_added_by_floor",
    "tiles_replaced",
    "tiles_replaced_by_floor",
    "unresolved_entrances",
}
DRAFT_ROW_KEYS = {"class", "colour", "floor", "ground", "item", "mapped", "samples"}


def check_draft(index: dict, summary: dict, errors: list[str]) -> None:
    """The minimap draft record matches its pin (tibiamaps sha256s) and every count adds up."""
    record = summary.get("draft")
    pin = (index.get("source") or {}).get("minimap_draft")
    if not isinstance(record, dict):
        errors.append(f"{SUMMARY}: the draft record is required")
        return
    if pin is None:
        if record != draft.unapplied():
            errors.append(f"{SUMMARY}: draft changes tiles but the index pins no draft")
        return
    if pin != draft.PIN:
        errors.append(f"{INDEX}: source.minimap_draft differs from the pinned draft")
    if set(record) != DRAFT_KEYS or record["applied"] is not True:
        errors.append(f"{SUMMARY}: draft record has the wrong keys")
        return
    try:
        problems = draft_counts(record, summary)
    except (KeyError, TypeError, AttributeError) as error:
        problems = [f"malformed record ({error!r})"]
    errors.extend(f"{SUMMARY}: draft {problem}" for problem in problems)


def draft_counts(record: dict, summary: dict) -> list[str]:
    problems: list[str] = []
    added = {str(z): 0 for z in draft.FLOORS}
    replaced = dict(added)
    markers = dict(added)
    if [(a["name"], a["bbox"]) for a in record["areas"]] != [
        (a["name"], a["bbox"]) for a in draft.AREAS
    ]:
        problems.append("areas differ from the pin")
        return problems
    pinned = {a["name"]: {str(z) for z in a["floors"]} for a in draft.AREAS}
    for area in record["areas"]:
        if (
            set(area) != {"bbox", "floors", "name"}
            or set(area["floors"]) != pinned[area["name"]]
        ):
            problems.append(f"area {area['name']} floors differ from the pin")
            continue
        for floor, row in area["floors"].items():
            if set(row) != set(draft.STAT_KEYS):
                problems.append(f"area {area['name']} floor {floor} has the wrong keys")
                continue
            if row["added"] + row["replaced"] != row["walkable"] + row["blocked"]:
                problems.append(f"floor {floor} drafted tiles do not add up")
            if row["explored"] != sum(
                row[k]
                for k in (
                    "added",
                    "replaced",
                    "kept_base",
                    "markers",
                    "no_official_land",
                    "unmapped",
                    "water_over_water",
                )
            ):
                problems.append(
                    f"floor {floor} explored pixels differ from their parts"
                )
            if int(floor) > draft.OFFICIAL_MAX_FLOOR and row["no_official_land"]:
                problems.append(f"floor {floor} skips tiles on the official minimap")
            added[floor] += row["added"]
            replaced[floor] += row["replaced"]
            markers[floor] += row["markers"]
    for key, expected in (("added", added), ("replaced", replaced)):
        by_floor = record[f"tiles_{key}_by_floor"]
        if by_floor != {f: n for f, n in expected.items() if n}:
            problems.append(f"tiles_{key}_by_floor differs from the area counts")
        if sum(by_floor.values()) != record[f"tiles_{key}"]:
            problems.append(f"tiles_{key} differs from its floors")
    floors = summary.get("tiles_by_floor", {})
    if any(n > floors.get(f, 0) for f, n in record["tiles_added_by_floor"].items()):
        problems.append("adds more tiles on a floor than exist")
    if record["items_added"] < record["tiles_added"]:
        problems.append("items_added is below tiles_added")
    if record["replaced_items_added"] < record["tiles_replaced"]:
        problems.append("replaced_items_added is below tiles_replaced")
    mapping = record["mapping"]
    rows = mapping["rows"]
    keys = [(r["floor"], r["colour"], r["class"]) for r in rows]
    if (
        set(mapping) != {"mapped", "min_samples", "rows", "unmapped"}
        or mapping["min_samples"] != draft.MIN_SAMPLES
        or keys != sorted(set(keys))
        or any(set(r) != DRAFT_ROW_KEYS for r in rows)
    ):
        problems.append("mapping table is malformed")
        return problems
    for row in rows:
        if row["mapped"] != (row["samples"] >= draft.MIN_SAMPLES) or row["mapped"] != (
            row["ground"] is not None
        ):
            problems.append(
                f"mapping row {row['floor']} {row['colour']} {row['class']} breaks the sample rule"
            )
        if row["class"] not in ("walkable", "blocked") or (
            row["class"] == "walkable" and row["item"] is not None
        ):
            problems.append(
                f"mapping row {row['floor']} {row['colour']} {row['class']} has a wrong class"
            )
        if row["colour"] == "#" + draft.MARKER.hex():
            problems.append("the marker colour is mapped")
    if mapping["mapped"] != sum(r["mapped"] for r in rows) or mapping[
        "unmapped"
    ] != sum(not r["mapped"] for r in rows):
        problems.append("mapping counts differ from the rows")
    entrances = record["unresolved_entrances"]
    if entrances != sorted(entrances, key=lambda p: (p[2], p[1], p[0])) or len(
        {tuple(p) for p in entrances}
    ) != len(entrances):
        problems.append("unresolved entrances are unsorted or repeated")
    boxes = {a["name"]: a["bbox"] for a in draft.AREAS}
    for p in entrances:
        if (
            len(p) != 3
            or str(p[2]) not in markers
            or not any(
                b[0] <= p[0] <= b[2] and b[1] <= p[1] <= b[3] for b in boxes.values()
            )
        ):
            problems.append("an unresolved entrance lies outside the areas or floors")
    for floor, count in markers.items():
        if sum(1 for p in entrances if str(p[2]) == floor) != count:
            problems.append(f"entrances of floor {floor} differ from the marker counts")
    return problems


EDRON_KEYS = {
    "applied",
    "entrances",
    "items_added",
    "reachability",
    "replaced_items_added",
    "replaced_items_removed",
    "rule1",
    "rule2",
    "tibiamaps_walkable_by_floor",
    "tiles_added",
    "tiles_added_by_floor",
    "tiles_replaced",
    "tiles_replaced_by_floor",
}
EDRON_ZERO_KEYS = {
    "applied",
    "items_added",
    "replaced_items_added",
    "replaced_items_removed",
    "tiles_added",
    "tiles_replaced",
}


def check_edron(index: dict, summary: dict, errors: list[str]) -> None:
    """The Edron rework record matches its pin (tibiamaps sha256s) and every count adds up."""
    rework = summary.get("edron")
    pin = (index.get("source") or {}).get("edron")
    if not isinstance(rework, dict):
        errors.append(f"{SUMMARY}: the edron record is required")
        return
    if pin is None:
        if set(rework) != EDRON_ZERO_KEYS or rework != {
            "applied": False,
            "items_added": 0,
            "replaced_items_added": 0,
            "replaced_items_removed": 0,
            "tiles_added": 0,
            "tiles_replaced": 0,
        }:
            errors.append(
                f"{SUMMARY}: edron changes tiles but the index pins no rework"
            )
        return
    if pin != edron.PIN:
        errors.append(
            f"{INDEX}: source.edron differs from the pinned rework (tibiamaps pins)"
        )
    if set(rework) != EDRON_KEYS or rework["applied"] is not True:
        errors.append(f"{SUMMARY}: edron record has the wrong keys")
        return
    try:
        problems = edron_counts(rework, summary)
    except (KeyError, TypeError, AttributeError) as error:
        problems = [f"malformed record ({error!r})"]
    errors.extend(f"{SUMMARY}: edron {problem}" for problem in problems)


def edron_counts(rework: dict, summary: dict) -> list[str]:
    problems: list[str] = []
    box_floors = {str(z) for z in edron.RULE2_FLOORS}
    rule1, rule2 = rework["rule1"], rework["rule2"]
    if rule1["floor"] != edron.RULE1_FLOOR or set(rule2) != box_floors:
        problems.append("rules name floors outside the pin")
    if rule1["filled"] != rule1["filled_walkable"] + rule1["filled_blocked"]:
        problems.append("rule 1 fill counts do not add up")
    if (
        rule1["replaced"]
        != rule1["replaced_to_walkable"] + rule1["replaced_to_blocked"]
    ):
        problems.append("rule 1 replace counts do not add up")
    if rule1["included"] != rule1["filled"] + rule1["replaced"] + rule1["kept_base"]:
        problems.append("rule 1 included tiles do not add up")
    if (
        rule1["core"] > rule1["included"]
        or rule1["included"] > rule1["summer_tiles_in_box"]
    ):
        problems.append("rule 1 core or included tiles exceed their bounds")
    added = {str(z): 0 for z in edron.RULE2_FLOORS}
    replaced = dict(added)
    added["10"] += rule1["filled"]
    replaced["10"] += rule1["replaced"]
    for floor, row in rule2.items():
        if row["targets"] != row["walkable_added"] + row["walkable_replaced"]:
            problems.append(f"rule 2 floor {floor} targets do not add up")
        if row["walkable_ground"] == row["blocking_ground"]:
            problems.append(f"rule 2 floor {floor} uses one ground for both")
        added[floor] += row["walkable_added"] + row["rock_added"]
        replaced[floor] += row["walkable_replaced"]
    for key, expected in (("added", added), ("replaced", replaced)):
        by_floor = rework[f"tiles_{key}_by_floor"]
        if by_floor != {f: n for f, n in expected.items() if n}:
            problems.append(f"tiles_{key}_by_floor differs from the rule counts")
        if sum(by_floor.values()) != rework[f"tiles_{key}"]:
            problems.append(f"tiles_{key} differs from its floors")
    floors = summary.get("tiles_by_floor", {})
    if any(n > floors.get(f, 0) for f, n in rework["tiles_added_by_floor"].items()):
        problems.append("adds more tiles on a floor than exist")
    entrances = rework["entrances"]
    marker_floors = {str(z) for z in edron.MARKER_FLOORS}
    if not (set(entrances["markers"]) == set(entrances["connected"]) == marker_floors):
        problems.append("entrance floors differ from the pin")
        return problems
    unresolved = entrances["unresolved_entrances"]
    if unresolved != sorted(unresolved, key=lambda p: (p[2], p[1], p[0])) or len(
        {tuple(p) for p in unresolved}
    ) != len(unresolved):
        problems.append("unresolved entrances are unsorted or repeated")
    for floor in marker_floors:
        here = sum(1 for p in unresolved if str(p[2]) == floor)
        if entrances["connected"][floor] + here != entrances["markers"][floor]:
            problems.append(f"entrances of floor {floor} do not add up")
    if any(
        len(p) != 3 or not edron.in_box(p[0], p[1]) or str(p[2]) not in marker_floors
        for p in unresolved
    ):
        problems.append("an unresolved entrance lies outside the box or the floors")
    reach = rework["reachability"]
    if (
        reach["floor10_walkable_reached_from_surface"] > reach["floor10_walkable"]
        or reach["floor10_new_walkable_reached_from_surface"]
        > reach["floor10_new_walkable"]
        or reach["floor10_new_walkable"] > reach["floor10_walkable"]
        or reach["floor10_largest_component"] > reach["floor10_walkable"]
        or reach["floor10_components_reached_from_surface"]
        > reach["floor10_components"]
    ):
        problems.append("reachability counts exceed their totals")
    return problems


def terrain_bindings(root: Path, errors: list[str]) -> dict[int, set[str]]:
    """``{appearance id: Terrain keys}`` from the committed Terrain family, if populated.

    A Terrain key is `oteryn:terrain.a` plus the appearance id in six digits, so the source
    id a palette entry names is the id in the key and in the record's binding.
    """
    path = f"{TERRAIN_DIRECTORY}/index.json"
    index = load(root, path, strict=False) if (root / path).is_file() else {}
    found: dict[int, set[str]] = {}
    for shard in index.get("shards", []):
        for record in load(root, shard, strict=False)["records"]:
            for row in record["source_bindings"]:
                if row["identity_namespace"] != TERRAIN_NAMESPACE:
                    continue
                appearance_id, key = int(row["external_id"]), row["target"]["key"]
                if key != f"{TERRAIN_KEY_PREFIX}{appearance_id:06d}":
                    errors.append(f"{shard}: terrain key {key!r} differs from its id")
                found.setdefault(appearance_id, set()).add(key)
    return found


def check_palette(
    palette,
    bound: dict[int, set[str]],
    terrain: dict[int, set[str]],
    errors: list[str],
) -> bool:
    """Check the palette in isolation; returns False when it cannot be indexed."""
    if not isinstance(palette, list) or not palette:
        errors.append(f"{INDEX}: palette must be a non-empty list")
        return False
    start = len(errors)
    keys: set[str] = set()
    ids: set[int] = set()
    for position, row in enumerate(palette):
        where = f"{INDEX}: palette[{position}]"
        if (
            not isinstance(row, dict)
            or set(row) not in (PALETTE_KEYS, RETIRED_KEYS)
            or row.get("retired", True) is not True
            or not isinstance(row["key"], str)
            or not isinstance(row["provisional"], bool)
            or not isinstance(row["source_item_id"], int)
            or isinstance(row["source_item_id"], bool)
            or row["source_item_id"] < 0
        ):
            errors.append(f"{where}: malformed entry {row!r}"[:200])
            continue
        server_id, key = row["source_item_id"], row["key"]
        if server_id in ids:
            errors.append(f"{where}: source_item_id {server_id} is listed twice")
        ids.add(server_id)
        if key in keys:
            errors.append(f"{where}: key {key!r} is listed twice")
        keys.add(key)
        if row["provisional"]:
            if key != f"{DONOR_PREFIX}{server_id}":
                errors.append(
                    f"{where}: provisional key must be {DONOR_PREFIX}{server_id}"
                )
            if server_id in bound or server_id in terrain:
                errors.append(f"{where}: provisional id {server_id} has a binding")
        elif key not in bound.get(server_id, ()) and key not in terrain.get(
            server_id, ()
        ):
            errors.append(
                f"{where}: key {key!r} is not an Item binding target or a Terrain "
                f"key of server id {server_id}"
            )
        if server_id in bound and server_id in terrain:
            errors.append(f"{where}: id {server_id} is both an Item and a Terrain id")
    return len(errors) == start


def check_palette_use(
    palette: list, used: Counter, summary: dict, items: int, errors: list[str]
) -> None:
    unused = [
        i for i, row in enumerate(palette) if used[i] == 0 and not row.get("retired")
    ]
    if unused:
        errors.append(
            f"{INDEX}: palette entries no item uses: "
            f"{[palette[i]['source_item_id'] for i in unused[:10]]}"
        )
    revived = [i for i, row in enumerate(palette) if used[i] and row.get("retired")]
    if revived:
        errors.append(
            f"{INDEX}: retired palette entries an item still uses: "
            f"{[palette[i]['source_item_id'] for i in revived[:10]]}"
        )
    if sum(used.values()) != items:
        errors.append(f"{INDEX}: palette occurrences differ from the item total")
    entries = occurrences = terrain_entries = terrain_occurrences = 0
    for i, row in enumerate(palette):
        if row["provisional"]:
            entries += 1
            occurrences += used[i]
        if row["key"].startswith(TERRAIN_KEY_PREFIX):
            terrain_entries += 1
            terrain_occurrences += used[i]
    recorded = summary.get("palette")
    provisional = recorded.get("provisional") if isinstance(recorded, dict) else None
    if not isinstance(provisional, dict) or set(recorded) != {
        "entries",
        "provisional",
        "terrain",
    }:
        errors.append(f"{SUMMARY}: palette must hold entries, provisional and terrain")
        return
    if recorded["terrain"] != {
        "entries": terrain_entries,
        "occurrences": terrain_occurrences,
    }:
        errors.append(f"{SUMMARY}: palette.terrain differs from the decoded palette")
    if recorded["entries"] != len(palette):
        errors.append(f"{SUMMARY}: palette.entries differs from the index palette")
    if (provisional.get("entries"), provisional.get("occurrences")) != (
        entries,
        occurrences,
    ):
        errors.append(
            f"{SUMMARY}: palette.provisional entries/occurrences differ from the "
            f"decoded {entries}/{occurrences}"
        )
    split = [provisional.get(k) for k in ("in_items_xml", "appearance_only")]
    if set(provisional) != {
        "appearance_only",
        "entries",
        "in_items_xml",
        "occurrences",
    } or any(
        not isinstance(row, dict) or set(row) != {"entries", "occurrences"}
        for row in split
    ):
        errors.append(f"{SUMMARY}: palette.provisional split is malformed")
    elif tuple(sum(row[k] for row in split) for k in ("entries", "occurrences")) != (
        entries,
        occurrences,
    ):
        errors.append(f"{SUMMARY}: palette.provisional split does not add up")


def unpopulated(root: Path) -> bool:
    """True while the directory is still the READY_UNPOPULATED marker and nothing else."""
    try:
        marker = json.loads((root / INDEX).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return False
    return (
        marker.get("schema") == "OTERYN_GAME_TREE_DIRECTORY/v1"
        and marker.get("population_state") == "READY_UNPOPULATED"
        and [p.name for p in (root / DIRECTORY).iterdir()] == ["index.json"]
    )


def validate(root: Path, pinned: dict | None = None, workers: int = 1) -> list[str]:
    errors: list[str] = []
    if unpopulated(root):
        return errors
    index = load(root, INDEX)
    summary = load(root, SUMMARY)
    check_index(index, summary, errors)
    rows = index.get("regions", [])
    paths = [row.get("path") for row in rows]
    if index.get("shards") != paths:
        errors.append(f"{INDEX}: shards must equal the region paths, in order")
    if paths != sorted(set(paths)):
        errors.append(f"{INDEX}: regions must be unique and sorted by path")
    for row in rows:
        if set(row) != REGION_KEYS or not REGION_PATH.match(str(row.get("path"))):
            errors.append(f"{INDEX}: malformed region row {row!r}"[:200])
        elif (
            not SHA256.match(str(row["sha256"])) or min(row["tiles"], row["items"]) < 0
        ):
            errors.append(f"{INDEX}: malformed region row {row['path']}")
    if errors:
        return errors
    directory = root / DIRECTORY
    actual = sorted(str(p.relative_to(root)) for p in directory.iterdir())
    if actual != sorted([INDEX, *paths]):
        stray = sorted(set(actual) ^ {INDEX, *paths})
        errors.append(
            f"{DIRECTORY}: files other than the index and its shards: {stray}"
        )
        return errors

    bound: dict[int, set[str]] = {}
    for row in load(root, ITEM_BINDINGS, strict=False)["bindings"]:
        if row["identity_namespace"] == ITEM_NAMESPACE:
            bound.setdefault(int(row["external_id"]), set()).add(row["target"]["key"])
    terrain = terrain_bindings(root, errors)
    palette = index["palette"]
    if not check_palette(palette, bound, terrain, errors):
        return errors
    extent = (summary["map"]["width"], summary["map"]["height"])
    args = (str(root), extent, len(palette))
    if workers > 1 and len(rows) > 8:
        with ProcessPoolExecutor(workers, initializer=_init, initargs=args) as pool:
            results = list(pool.map(check_region, rows, chunksize=16))
    else:
        _init(*args)
        results = [check_region(row) for row in rows]

    floors: Counter = Counter()
    attributes: Counter = Counter()
    used: Counter = Counter()
    totals = Counter()
    for result in results:
        errors.extend(result["errors"])
        used.update(result["palette"])
        floors[result["z"]] += result["tiles"]
        attributes.update(result["attributes"])
        totals.update(
            regions=1,
            sectors=result["sectors"],
            tiles=result["tiles"],
            items=result["items"],
            bytes=result["size"],
            houses=result["houses"],
            zones=result["zones"],
        )
    counted = {key: totals[key] for key in sorted(TOTAL_KEYS)}
    if index["totals"] != counted:
        errors.append(
            f"{INDEX}: totals {index['totals']} differ from the decoded {counted}"
        )
    check_palette_use(palette, used, summary, counted["items"], errors)
    if summary.get("totals") != index["totals"]:
        errors.append(f"{SUMMARY}: totals differ from the index")
    if summary.get("bytes_on_disk") != totals["bytes"]:
        errors.append(f"{SUMMARY}: bytes_on_disk differs from the region files")
    if summary.get("tiles_with_house") != totals["houses"]:
        errors.append(f"{SUMMARY}: tiles_with_house differs from the decoded tiles")
    if summary.get("tiles_with_zone") != totals["zones"]:
        errors.append(f"{SUMMARY}: tiles_with_zone differs from the decoded tiles")
    expected_floors = {str(z): n for z, n in sorted(floors.items())}
    if summary.get("tiles_by_floor") != expected_floors:
        errors.append(f"{SUMMARY}: tiles_by_floor differs from the decoded tiles")
    attributes.pop("depth", None)
    if summary.get("item_attributes") != dict(sorted(attributes.items())):
        errors.append(f"{SUMMARY}: item_attributes differs from the decoded items")
    if summary.get("schema") != "OTERYN_WORLD_BASE_SOURCE_CAPTURE/v1":
        errors.append(f"{SUMMARY}: wrong schema")
    if summary.get("rejected_items") != {"unsupported_attributes": 0}:
        errors.append(f"{SUMMARY}: rejected_items must be unsupported_attributes 0")
    info = summary.get("codec", {})
    if (info.get("name"), info.get("zstd_level")) != (codec.CODEC, codec.ZSTD_LEVEL):
        errors.append(f"{SUMMARY}: codec differs from the index")
    if not all(isinstance(v, str) and v for v in info.get("zstd", {}).values()) or set(
        info.get("zstd", {})
    ) != {"backend", "libzstd", "python_package"}:
        errors.append(f"{SUMMARY}: codec.zstd must record backend, libzstd and package")
    if summary.get("map", {}).get("floors") != [0, codec.MAX_FLOOR]:
        errors.append(f"{SUMMARY}: map floors must be [0, {codec.MAX_FLOOR}]")
    fill = summary.get("fill", {})
    base = world_otbm_totals(index["totals"], summary)
    if pinned is not None and {k: base[k] for k in pinned} != pinned:
        errors.append(
            f"{INDEX}: world.otbm totals differ from the pinned source {pinned}"
        )
    for floor, added in (
        (f, n)
        for s in fill.get("sources", [])
        for f, n in s.get("tiles_added_by_floor", {}).items()
    ):
        if added > summary.get("tiles_by_floor", {}).get(floor, 0):
            errors.append(
                f"{SUMMARY}: fill adds more tiles on floor {floor} than exist"
            )
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=HERE.parents[2])
    parser.add_argument("--workers", type=int, default=min(4, os.cpu_count() or 1))
    args = parser.parse_args()
    try:
        errors = validate(args.root.resolve(), PINNED_TOTALS, args.workers)
    except ValidationError as error:
        errors = [str(error)]
    for error in errors[:50]:
        print(f"FAIL {error}", file=sys.stderr)
    if errors:
        return 1
    state = "unpopulated marker" if unpopulated(args.root.resolve()) else "populated"
    print(f"PASS world base family ({state})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
