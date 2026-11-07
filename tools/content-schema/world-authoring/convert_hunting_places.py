#!/usr/bin/env python3
"""Convert the pinned English TibiaWiki hunting-place snapshot into `Area.HuntingPlace` records.

Reads `imports/tibiawiki/hunting-places/fandom-snapshot-v1.json` (offline; captured by
`fandom_hunting_snapshot.py`) and the AREAS-1 City Areas (content/world/areas/cities/), and writes
content/world/areas/hunting-places/ plus `samples/hunting-places-capture-v1.json`. Only
facts that parse unambiguously are written; everything else is omitted and counted.

    python convert_hunting_places.py [--snapshot PATH] [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import convert_world_metadata as base

ROOT = base.ROOT
HERE = base.HERE
SNAPSHOT = "imports/tibiawiki/hunting-places/fandom-snapshot-v1.json"
SUMMARY = "tools/content-schema/world-authoring/samples/hunting-places-capture-v1.json"
CRYSTAL_SUMMARY = "tools/content-schema/world-authoring/samples/source-capture-v1.json"
GENERATOR = "tools/content-schema/world-authoring/convert_hunting_places.py"
CITY_DIRECTORY = "content/world/areas/cities"
SNAPSHOT_SCHEMA = "OTERYN_TIBIAWIKI_FANDOM_HUNTING_PLACES_SNAPSHOT/v1"
NAMESPACE = "tibiawiki-fandom/page-id"
SOURCE_KEY = "oteryn:source.tibiawiki"
FAMILY = "Area.HuntingPlace"
KEY_PREFIX = "oteryn:area.hunting_place."
VOCATIONS = {"lvlknights": "knight", "lvlpaladins": "paladin", "lvlmages": "mage"}
MAX_LEVEL = 5000
CREATURE_NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9 '().,:&!/-]{0,63}$")
SECTOR = 256


class ConvertError(base.ConvertError):
    pass


def clean(text: str) -> str:
    """Plain text of a short wiki value: links, comments and `<br>` reduced to their text."""
    text = re.sub(r"<!--.*?-->", "", text, flags=re.DOTALL)
    text = re.sub(r"\[\[[^\]|]*\|([^\]]*)\]\]", r"\1", text)
    text = re.sub(r"\[\[([^\]]*)\]\]", r"\1", text)
    text = re.sub(r"<br\s*/?>", " ", text, flags=re.IGNORECASE)
    return re.sub(r"\s+", " ", text.replace("_", " ")).strip()


def parse_level(raw: str | None) -> int | None:
    """A plain integer level, or None ('?', '600?', empty, ranges and the like are unparsed)."""
    if raw is None or not re.fullmatch(r"\d{1,4}", raw.strip()):
        return None
    value = int(raw)
    return value if 1 <= value <= MAX_LEVEL else None


def parse_mapper_coords(raw: str) -> tuple[int, int, int] | None:
    """`x.y|x.y|z|...` (sector.offset) -> absolute (x, y, z), or None if not exactly that form.

    TibiaWiki's Mapper writes a tile coordinate as `sector.offset` with x = sector * 256 +
    offset (Thais temple 32369,32241,7 is `126.113|125.241|7`). Named parameters such as
    `text=here` may appear before or after the positional ones and are ignored.
    """
    positional = [
        part.strip()
        for part in raw.split("|")
        if not re.match(r"^\s*[A-Za-z_][A-Za-z0-9_ ]*=", part)
    ]
    if len(positional) < 3:
        return None
    axes = []
    for part in positional[:2]:
        match = re.fullmatch(r"(\d{1,3})\.(\d{1,3})", part)
        if not match or int(match.group(2)) >= SECTOR:
            return None
        axes.append(int(match.group(1)) * SECTOR + int(match.group(2)))
    if not re.fullmatch(r"\d{1,2}", positional[2]):
        return None
    floor = int(positional[2])
    return (axes[0], axes[1], floor) if floor <= base.MAX_FLOOR else None


def city_names(root: Path) -> dict[str, str]:
    """Lower-cased City Area name -> key, from the AREAS-1 city records in main."""
    names: dict[str, str] = {}
    for shard in sorted((root / CITY_DIRECTORY).glob("areas-*.json")):
        for area in json.loads(shard.read_text(encoding="utf-8"))["areas"]:
            names[area["name"].lower()] = area["identity"]["key"]
    return names


def map_extent(root: Path) -> tuple[int, int]:
    summary = json.loads((root / CRYSTAL_SUMMARY).read_text(encoding="utf-8"))
    return summary["map"]["width"], summary["map"]["height"]


def load_snapshot(data: bytes) -> dict:
    snapshot = json.loads(data)
    if snapshot.get("schema") != SNAPSHOT_SCHEMA:
        raise ConvertError("snapshot schema differs")
    ids = [row["pageid"] for row in snapshot["pages"]]
    if not ids or len(set(ids)) != len(ids):
        raise ConvertError("snapshot pages are empty or repeat a page id")
    if any(row["pageid"] < 1 or row["revid"] < 1 for row in snapshot["pages"]):
        raise ConvertError("snapshot page or revision id is not positive")
    return snapshot


def source(snapshot: dict, data: bytes) -> dict:
    return {
        "category": snapshot["category"],
        "evidence": "Derived",
        "fetched_at": snapshot["fetched_at"],
        "license": snapshot["license"],
        "site": snapshot["source_url"],
        "snapshot": {"path": SNAPSHOT, "sha256": hashlib.sha256(data).hexdigest()},
        "source_key": SOURCE_KEY,
    }


def wiki_binding(key: str, row: dict) -> dict:
    return {
        "disposition": "EXACT",
        "external_id": str(row["pageid"]),
        "identity_namespace": NAMESPACE,
        "source_key": SOURCE_KEY,
        "source_revision": str(row["revid"]),
        "target": base.ref("Area", key),
    }


def hunting_places(snapshot: dict, root: Path, counts: dict) -> list[dict]:
    cities = city_names(root)
    width, height = map_extent(root)
    rows = snapshot["pages"]
    keys = base.assign_keys(
        [(str(row["pageid"]), row["title"]) for row in rows],
        base.committed_keys(root, FAMILY, NAMESPACE),
        KEY_PREFIX,
        FAMILY,
    )
    records = []
    for row in rows:
        facts = row["facts"]
        key = keys[str(row["pageid"])]
        name = row["title"]
        if not 1 <= len(name) <= 64:
            raise ConvertError(f"{key}: page title is not a usable name")
        declaration: dict = {
            "area_kind": "hunting_place",
            "identity": {"key": key, "revision": base.REVISION},
            "kind": "Area",
            "name": name,
        }
        source_facts: dict = {}
        city_name = clean(facts.get("city", ""))
        if city_name:
            source_facts["city_name"] = city_name
            if city_name.lower() in cities:
                declaration["city"] = base.ref("Area", cities[city_name.lower()])
                counts["city"] += 1
            else:
                counts["city_unmatched"] += 1
        else:
            counts["city_absent"] += 1
        coordinates = facts["location_coordinates"]
        if not coordinates:
            counts["position_absent"] += 1
        elif len(coordinates) > 1:
            counts["position_ambiguous"] += 1
        else:
            parsed = parse_mapper_coords(coordinates[0])
            if parsed is None or not (parsed[0] < width and parsed[1] < height):
                counts["position_unparsed"] += 1
            else:
                declaration["position"] = base.position(*parsed)
                counts["position"] += 1
        levels = {
            vocation: level
            for field, vocation in VOCATIONS.items()
            if (level := parse_level(facts.get(field))) is not None
        }
        if levels:
            declaration["recommended_levels"] = dict(sorted(levels.items()))
            counts["levels"] += 1
        else:
            counts["levels_absent"] += 1
        creatures = set()
        for raw in facts["creatures"]:
            creature = clean(raw)
            if CREATURE_NAME.fullmatch(creature):
                creatures.add(creature)
            else:
                counts["creature_names_unparsed"] += 1
        if creatures:
            source_facts["creature_names"] = sorted(creatures)
            counts["creatures"] += 1
        if source_facts:
            declaration["source_facts"] = source_facts
        records.append(
            {"declaration": declaration, "source_bindings": [wiki_binding(key, row)]}
        )
    return base.unique(records, FAMILY)


def build(snapshot_bytes: bytes, root: Path = ROOT) -> dict[str, bytes]:
    snapshot = load_snapshot(snapshot_bytes)
    counts = {
        name: 0
        for name in (
            "city",
            "city_absent",
            "city_unmatched",
            "creature_names_unparsed",
            "creatures",
            "levels",
            "levels_absent",
            "position",
            "position_absent",
            "position_ambiguous",
            "position_unparsed",
        )
    }
    records = hunting_places(snapshot, root, counts)
    pinned = source(snapshot, snapshot_bytes)
    out = base.shard_files(FAMILY, records, pinned, GENERATOR)
    summary = {
        "families": {FAMILY: len(records)},
        "records_with": {
            "city": counts.pop("city"),
            "creatures": counts.pop("creatures"),
            "levels": counts.pop("levels"),
            "position": counts.pop("position"),
        },
        "not_imported": {
            **counts,
            "pages_without_hunt_infobox": len(snapshot["pages_without_hunt_infobox"]),
        },
        "schema": "OTERYN_HUNTING_PLACES_SOURCE_CAPTURE/v1",
        "source": pinned,
    }
    out[SUMMARY] = base.canonical(summary)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--snapshot", type=Path, default=ROOT / SNAPSHOT)
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    args = parser.parse_args()
    try:
        out = build(args.snapshot.read_bytes())
    except (base.ConvertError, OSError, KeyError, json.JSONDecodeError) as error:
        print(f"FAIL {error!r}", file=sys.stderr)
        return 1
    return base.apply(out, args.check)


if __name__ == "__main__":
    raise SystemExit(main())
